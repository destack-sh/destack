import { type ReplicaSource } from "@destack/sync";
import type { ResourceContext } from "@destack/resource/context";
import { DeclarationReference } from "@destack/package/declare";
import { Server, type ServerOptions, type ServiceImplementation } from "../server/index.ts";
import type { Service } from "../declare/service.ts";
import type { RunClient, Trigger } from "../trigger/index.ts";
import { type Alarm, AlarmClock } from "../control/index.ts";
import { Health } from "../health/index.ts";
import { ServiceError } from "../error/index.ts";
import type { AuditHistory, Workload } from "./workload.ts";
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

    /** Start a workload. */
    static async start(
        workload: Workload,
        options: WorkloadInstanceOptions,
    ): Promise<WorkloadInstance> {
        // clean up a failed start, keeping the services' alarms on the host's one wake-up
        const instance = new WorkloadInstance();
        const clock = options.alarm === undefined ? undefined : new AlarmClock(options.alarm);
        try {
            // start the workload
            const implementation = await workload.start({
                resources: options.resources,
                history: options.history,
                ...(options.replicas === undefined ? {} : { replicas: options.replicas }),
                runs: options.runs,
                callKey: options.callKey,
                signal: instance.#controller.signal,
                shutdown: () => instance.shutdown(),
                report: options.report,
                defer: (dispose) => instance.#cleanup.defer(dispose),
            });

            // start each service
            for (const service of implementation.services) {
                instance.signal.throwIfAborted();
                const key = keyOf(service.service);
                if (instance.#services.has(key)) {
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
                instance.#services.set(key, { service: service.service, server });
            }

            // register the triggers
            for (const trigger of implementation.triggers ?? []) {
                const key = keyOf(trigger);
                if (instance.#triggers.has(key)) {
                    throw new TypeError(`duplicate workload trigger: ${key}`);
                }
                instance.#triggers.set(key, trigger);
            }

            // reject a cancelled start
            instance.signal.throwIfAborted();
        } catch (error) {
            // release a failed start
            try {
                await instance.close();
            } catch (cleanup) {
                throw new AggregateError([error, cleanup], "workload startup and cleanup failed");
            }
            throw error;
        }

        return instance;
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

    /** Settle once no service's controller key is due now or reconciling. */
    async idle(): Promise<void> {
        await Promise.all([...this.#services.values()].map((served) => served.server.idle()));
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
        const errors = results.flatMap((result) =>
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
    /** The audit history the workload's outboxes deliver to. */
    readonly history: AuditHistory;
    /** The source of the copies of the installation's space: its chain, and the global rows it reads. */
    readonly replicas?: { readonly scope: string; readonly source: ReplicaSource };
    /** The cell recording the installation's runs. */
    readonly runs: RunClient;
    /** Read the key the workload's journals fingerprint sensitive inputs under. */
    readonly callKey: CallKey;
    /** Keep a wake-up for the services' earliest due controller key, such as a Durable Object's alarm. */
    readonly alarm?: Alarm;
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
