import type { ResourceContext } from "@destack/resource/context";
import { DeclarationReference } from "@destack/package/declare";
import { Server, type ServerOptions, type ServiceImplementation } from "../server/index.ts";
import type { Service } from "../declare/service.ts";
import type { RunClient, Trigger } from "../trigger/index.ts";
import { type Alarm, AlarmClock } from "../control/index.ts";
import { Health } from "../health/index.ts";
import { ServiceError } from "../error/index.ts";
import type { AuditHistory, InstallationContext, Workload, WorkloadContext } from "./workload.ts";
import { type CallKey } from "../request/index.ts";

/** A running workload instance. */
export class WorkloadInstance implements AsyncDisposable {
    /** The services and their servers by declaration key. */
    readonly #services = new Map<string, { readonly service: Service; readonly server: Server }>();
    /** The triggers of the workload's package, by declaration key. */
    readonly #triggers = new Map<string, Trigger>();
    /** The shutdown controller. */
    readonly #controller = new AbortController();
    /** The resources released after draining. */
    readonly #cleanup = new AsyncDisposableStack();
    /** The shutdown. */
    #closing?: Promise<void>;
    /** The clock keeping the services' alarms on the host's one wake-up, absent without one. */
    readonly #clock: AlarmClock | undefined;

    /** Keep the clock of the host's wake-up. */
    private constructor(clock: AlarmClock | undefined) {
        this.#clock = clock;
    }

    /** Start a workload. */
    static async start(
        workload: Workload,
        options: WorkloadInstanceOptions,
    ): Promise<WorkloadInstance> {
        // clean up a failed start, keeping the services' alarms on the host's one wake-up
        const clock = options.alarm === undefined ? undefined : new AlarmClock(options.alarm);
        const instance = new WorkloadInstance(clock);
        try {
            // start the workload
            const implementation = await workload.start({
                resources: options.resources,
                ...(options.history === undefined ? {} : { history: options.history }),
                ...(options.installation === undefined
                    ? {}
                    : { installation: options.installation }),
                ...(options.runs === undefined ? {} : { runs: options.runs }),
                callKey: options.callKey,
                ...(options.ephemeral === undefined ? {} : { ephemeral: options.ephemeral }),
                signal: instance.#controller.signal,
                shutdown: () => instance.shutdown(),
                report: (error) => options.report(error),
                defer: (dispose) => instance.#cleanup.defer(dispose),
            });

            // start each service and register the triggers
            for (const service of implementation.services) {
                instance.#serve(service, options, clock);
            }
            for (const trigger of implementation.triggers ?? []) {
                instance.#register(trigger);
            }

            // reject a cancelled start
            instance.signal.throwIfAborted();
        } catch (error) {
            // release a failed start
            try {
                await instance.close();
            } catch (cleanup) {
                throw new AggregateError([error, cleanup], "workload startup and cleanup failed", {
                    cause: cleanup,
                });
            }
            throw error;
        }

        return instance;
    }

    /** Start the server of one implemented service, refusing a duplicate. */
    #serve(
        service: ServiceImplementation,
        options: WorkloadInstanceOptions,
        clock: AlarmClock | undefined,
    ): void {
        // refuse a cancelled start and a duplicate service
        this.signal.throwIfAborted();
        const key = keyOf(service.service);
        if (this.#services.has(key)) {
            throw new TypeError(`duplicate workload service: ${key}`);
        }

        // start the server
        const server = Server.start({
            ...service,
            ...options.service(service.service),
            health: new Health(service.service.name),
            resources: options.resources,
            ...(clock === undefined ? {} : { alarm: clock.alarm() }),
        });
        this.#services.set(key, { service: service.service, server });
    }

    /** Register one trigger of the workload's package, refusing a duplicate. */
    #register(trigger: Trigger): void {
        const key = keyOf(trigger);
        if (this.#triggers.has(key)) {
            throw new TypeError(`duplicate workload trigger: ${key}`);
        }
        this.#triggers.set(key, trigger);
    }

    /** The shutdown signal. */
    get signal(): AbortSignal {
        return this.#controller.signal;
    }

    /** The implemented services, in workload order. */
    get services(): readonly Service[] {
        return [...this.#services.values()].map((served) => served.service);
    }

    /** The triggers of the workload's package, in workload order. */
    get triggers(): readonly Trigger[] {
        return [...this.#triggers.values()];
    }

    /** Find a trigger of the workload by its package and name. */
    trigger(packageId: string, name: string): Trigger | undefined {
        return this.#triggers.get(`${packageId}/${name}`);
    }

    /** Report whether the workload implements a service. */
    has(service: Service): boolean {
        return this.#services.has(keyOf(service));
    }

    /** Request shutdown. */
    shutdown(): void {
        this.#controller.abort();
    }

    /** Run the controllers at the host's wake-up until no key is due now or reconciling, or until a deadline. */
    async alarm(deadline: number): Promise<void> {
        // set the wake-up again, which the host cleared as it rang
        await this.#clock?.rang();
        await Promise.all(
            [...this.#services.values()].map((served) => served.server.idle(deadline)),
        );
    }

    /** Dispatch a request to a service. */
    fetch(service: Service, request: Request): Promise<Response> {
        // require the service
        const server = this.#services.get(keyOf(service))?.server;
        if (!server) {
            throw new ServiceError("NOT_FOUND", {
                message: `unknown workload service: ${keyOf(service)}`,
            });
        }

        return server.fetch(request);
    }

    /** Close the workload. */
    close(): Promise<void> {
        this.shutdown();
        this.#closing ??= this.#close();

        return this.#closing;
    }

    /** Close the workload. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.close();
    }

    /** Close every service and resource, reporting all failures. */
    async #close(): Promise<void> {
        // drain each service
        const servers = [...this.#services.values()].map((served) => served.server);
        const results = await Promise.allSettled(servers.map((server) => server.close()));
        const errors = results.flatMap((result): unknown[] =>
            result.status === "rejected" ? [result.reason] : [],
        );

        // wait for the servers to stop
        await Promise.all(servers.map((server) => server.stopped));

        // release the resources
        try {
            await this.#cleanup.disposeAsync();
        } catch (error) {
            errors.push(error);
        }

        // report the failures
        if (errors.length) {
            throw new AggregateError(errors, "workload shutdown failed");
        }
    }
}

/** The hosting configuration of a workload. */
export interface WorkloadInstanceOptions {
    /** The installation's resources. */
    readonly resources: ResourceContext;
    /** The audit history the workload's outboxes deliver to, absent for a workload journaling only in its own database. */
    readonly history?: AuditHistory;
    /** The installation the workload runs as, absent outside a space. */
    readonly installation?: InstallationContext;
    /** The cell recording the installation's runs, absent outside a space. */
    readonly runs?: RunClient;
    /** Read the key the workload's journals fingerprint sensitive inputs under. */
    readonly callKey: CallKey;
    /** Keep a wake-up for the services' earliest due controller key, such as a Durable Object's alarm. */
    readonly alarm?: Alarm;
    /** Open the database the instance's ephemeral objects live in, absent where the workload opens its own. */
    readonly ephemeral?: WorkloadContext["ephemeral"];
    /** Report a failure of the workload's background work to its host. */
    report(error: unknown): void;
    /** Select the options of one service. */
    service(
        service: Service,
    ): Omit<ServerOptions, keyof ServiceImplementation | "health" | "resources">;
}

/** Key a declaration by its package and name. */
function keyOf(declaration: Service | Trigger): string {
    const { packageId, name } = DeclarationReference.of(declaration);

    return `${packageId}/${name}`;
}
