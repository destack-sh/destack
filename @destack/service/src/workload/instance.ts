import { type Clock, present } from "@destack/schema";
import type { Caller } from "@destack/access";
import type { ResourceContext } from "@destack/resource/context";
import { DeclarationReference } from "@destack/package/declare";
import { Server, type ServerOptions, type ServiceImplementation } from "../server/index.ts";
import type { Service } from "../declare/service.ts";
import type { RunClient, Trigger } from "../trigger/index.ts";
import { type Alarm, AlarmClock } from "../control/index.ts";
import { Health } from "../health/index.ts";
import { ServiceError } from "../error/index.ts";
import type {
    AuditHistory,
    EphemeralHost,
    InstallationContext,
    Workload,
    WorkloadContext,
} from "./workload.ts";
import { CallKey } from "../request/index.ts";
import type { Hibernation, SocketHandler } from "./socket.ts";

/** A running workload instance, serving its current generation. */
export class WorkloadInstance implements AsyncDisposable {
    /** The workload the instance runs. */
    readonly #workload: Workload;
    /** The hosting configuration each generation starts with. */
    readonly #options: WorkloadInstanceOptions;
    /** The shutdown controller. */
    readonly #controller: AbortController;
    /** The clock keeping the services' alarms on the host's one wake-up, absent without one. */
    readonly #clock: AlarmClock | undefined;
    /** The generation serving requests. */
    #current: Generation;
    /** The reload in progress, which a further reload waits for. */
    #reloading: Promise<void> = Promise.resolve();
    /** The shutdown. */
    #closing?: Promise<void>;

    /** Keep the workload, its hosting, its shutdown, the clock and the first generation. */
    private constructor(
        workload: Workload,
        options: WorkloadInstanceOptions,
        controller: AbortController,
        clock: AlarmClock | undefined,
        current: Generation,
    ) {
        // keep what each further start needs, and the start serving now
        this.#workload = workload;
        this.#controller = controller;
        this.#options = options;
        this.#clock = clock;
        this.#current = current;
    }

    /** Start a workload. */
    static async start(
        workload: Workload,
        options: WorkloadInstanceOptions,
    ): Promise<WorkloadInstance> {
        // start the first generation, which reloads through the instance once it exists
        const clock = options.alarm === undefined ? undefined : new AlarmClock(options.alarm);
        const controller = new AbortController();
        let instance: WorkloadInstance | undefined;
        const first = await Generation.start(workload, options, clock, {
            signal: controller.signal,
            reload: () => present(instance, "the started instance").reload(),
            shutdown: () => controller.abort(),
        });
        instance = new WorkloadInstance(workload, options, controller, clock, first);

        return instance;
    }

    /** Start the workload again, serve the new generation once started, then drain the old one. */
    reload(): Promise<void> {
        this.#reloading = this.#reloading.then(async () => {
            // keep serving the current generation while the next one starts
            this.signal.throwIfAborted();
            const next = await Generation.start(this.#workload, this.#options, this.#clock, {
                signal: this.signal,
                reload: () => this.reload(),
                shutdown: () => this.shutdown(),
            });

            // switch to the next generation and drain the previous one
            const previous = this.#current;
            this.#current = next;
            await previous.close();
        });

        return this.#reloading;
    }

    /** The shutdown signal. */
    get signal(): AbortSignal {
        return this.#controller.signal;
    }

    /** The implemented services, in workload order. */
    get services(): readonly Service[] {
        return this.#current.services;
    }

    /** The triggers of the workload's package, in workload order. */
    get triggers(): readonly Trigger[] {
        return this.#current.triggers;
    }

    /** Find a trigger of the workload by its package and name. */
    trigger(packageId: string, name: string): Trigger | undefined {
        return this.#current.trigger(packageId, name);
    }

    /** Report whether the workload implements a service. */
    has(service: Service): boolean {
        return this.#current.has(service);
    }

    /** What the workload does with its held sockets' events, refusing a workload holding none. */
    requireSocketHandler(): SocketHandler {
        return this.#current.requireSocketHandler();
    }

    /** Request shutdown. */
    shutdown(): void {
        this.#controller.abort();
    }

    /** Run the controllers at the host's wake-up. */
    async alarm(deadline: number): Promise<void> {
        await this.#clock?.rang();
        await this.#current.alarm(deadline);
    }

    /** Dispatch a request to a service of the current generation. */
    fetch(service: Service, request: Request, caller?: Caller): Promise<Response> {
        return this.#current.fetch(service, request, caller);
    }

    /** Close the workload. */
    close(): Promise<void> {
        this.shutdown();
        this.#closing ??= this.#reloading.catch(() => undefined).then(() => this.#current.close());

        return this.#closing;
    }

    /** Close the workload. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.close();
    }
}

/** One start of a workload: its servers, triggers and sockets, and the resources it releases. */
class Generation {
    /** The services and their servers by declaration key. */
    readonly #services = new Map<string, { readonly service: Service; readonly server: Server }>();
    /** The triggers of the workload's package, by declaration key. */
    readonly #triggers = new Map<string, Trigger>();
    /** The controller aborting the start's background work at its retirement. */
    readonly #controller = new AbortController();
    /** The resources released after draining. */
    readonly #cleanup = new AsyncDisposableStack();
    /** What the start does with its held sockets' events, absent for a start holding none. */
    #sockets: SocketHandler | undefined;
    /** The shutdown. */
    #closing?: Promise<void>;

    /** Start a workload once, closing what it opened when the start fails. */
    static async start(
        workload: Workload,
        options: WorkloadInstanceOptions,
        clock: AlarmClock | undefined,
        host: Pick<WorkloadContext, "signal" | "reload" | "shutdown">,
    ): Promise<Generation> {
        // abort the start with its instance
        const generation = new Generation();
        host.signal.addEventListener("abort", () => generation.shutdown(), { once: true });
        if (host.signal.aborted) {
            generation.shutdown();
        }
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
                deriver: await CallKey.deriver(await options.callKey()),
                clock: options.clock,
                ...(options.ephemeral === undefined ? {} : { ephemeral: options.ephemeral }),
                ...(options.hibernation === undefined ? {} : { hibernation: options.hibernation }),
                signal: generation.#controller.signal,
                shutdown: () => host.shutdown(),
                reload: () => host.reload(),
                report: (error) => options.report(error),
                defer: (dispose) => generation.#cleanup.defer(dispose),
            });

            // start each service and register the triggers
            for (const service of implementation.services) {
                generation.#serve(service, options, clock);
            }
            for (const trigger of implementation.triggers ?? []) {
                generation.#register(trigger);
            }
            generation.#sockets = implementation.sockets;

            // reject a cancelled start
            generation.#controller.signal.throwIfAborted();
        } catch (error) {
            // release a failed start
            try {
                await generation.close();
            } catch (cleanup) {
                throw new AggregateError([error, cleanup], "workload startup and cleanup failed", {
                    cause: cleanup,
                });
            }
            throw error;
        }

        return generation;
    }

    /** Start the server of one implemented service, refusing a duplicate. */
    #serve(
        service: ServiceImplementation,
        options: WorkloadInstanceOptions,
        clock: AlarmClock | undefined,
    ): void {
        // refuse a cancelled start and a duplicate service
        this.#controller.signal.throwIfAborted();
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
            clock: options.clock,
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

    /** Report whether the start implements a service. */
    has(service: Service): boolean {
        return this.#services.has(keyOf(service));
    }

    /** What the start does with its held sockets' events, refusing a start holding none. */
    requireSocketHandler(): SocketHandler {
        if (this.#sockets === undefined) {
            throw new ServiceError("NOT_FOUND", { message: "the workload handles no sockets" });
        }

        return this.#sockets;
    }

    /** Abort the start's background work. */
    shutdown(): void {
        this.#controller.abort();
    }

    /** Run the servers' controllers due by a deadline. */
    async alarm(deadline: number): Promise<void> {
        await Promise.all(
            [...this.#services.values()].map((served) => served.server.idle(deadline)),
        );
    }

    /** Dispatch a request to a service. */
    fetch(service: Service, request: Request, caller?: Caller): Promise<Response> {
        // require the service
        const server = this.#services.get(keyOf(service))?.server;
        if (!server) {
            throw new ServiceError("NOT_FOUND", {
                message: `unknown workload service: ${keyOf(service)}`,
            });
        }

        return server.fetch(request, caller);
    }

    /** Drain the servers and release the resources once. */
    close(): Promise<void> {
        this.shutdown();
        this.#closing ??= this.#close();

        return this.#closing;
    }

    /** Close every server and resource, reporting all failures. */
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
    /** The audit history the workload's outboxes deliver to. */
    readonly history?: AuditHistory;
    /** The installation the workload runs as, absent outside a space. */
    readonly installation?: InstallationContext;
    /** The machine recording the installation's runs, absent outside a space. */
    readonly runs?: RunClient;
    /** Read the key the workload's journals fingerprint sensitive inputs under. */
    readonly callKey: CallKey;
    /** Keep a wake-up for the services' earliest due controller key. */
    readonly alarm?: Alarm;
    /** Where the instance keeps its ephemeral objects, absent outside a runner. */
    readonly ephemeral?: EphemeralHost;
    /** The WebSockets the host keeps open for the instance, absent where the host keeps none. */
    readonly hibernation?: Hibernation;
    /** Report a failure of the workload's background work to its host. */
    report(error: unknown): void;
    /** The clock the workload runs at. */
    readonly clock: Clock;
    /** Select the options of one service. */
    service(
        service: Service,
    ): Omit<ServerOptions, keyof ServiceImplementation | "health" | "resources" | "clock">;
}

/** Key a declaration by its package and name. */
function keyOf(declaration: Service | Trigger): string {
    const { packageId, name } = DeclarationReference.of(declaration);

    return `${packageId}/${name}`;
}
