import { present } from "@destack/schema";
import type { Authentication } from "@destack/access";
import type { ResourceContext } from "@destack/resource/context";
import { Server, type ServerOptions, type PackageImplementation } from "../server/index.ts";
import type { PackageHandle } from "../declare/package.ts";
import { type Alarm, AlarmClock } from "../control/index.ts";
import { Health } from "../health/index.ts";
import { CallError } from "../error/index.ts";
import type {
    EphemeralHost,
    HookHandler,
    ServerContext,
    WebhookHandler,
    Workload,
    WorkloadContext,
} from "./workload.ts";
import { CallKey } from "../request/index.ts";
import type { Hibernation, SocketHandler } from "./socket.ts";
import type { Clock } from "@destack/environment";

/** A running workload instance, serving its current generation. */
export class WorkloadInstance implements AsyncDisposable {
    /** The workload the instance runs. */
    readonly #workload: Workload;
    /** The hosting configuration each generation starts with. */
    readonly #options: WorkloadInstanceOptions;
    /** The shutdown controller. */
    readonly #controller: AbortController;
    /** The clock placing the servers' alarms on the host's one wake-up, absent without one. */
    readonly #clock: AlarmClock | undefined;
    /** The generation serving requests. */
    #current: Generation;
    /** The reload in progress, which a further reload waits for. */
    #reloading: Promise<void> = Promise.resolve();
    /** The shutdown. */
    #closing?: Promise<void>;

    /** Create the instance with its workload, hosting, shutdown, clock and first generation. */
    private constructor(
        workload: Workload,
        options: WorkloadInstanceOptions,
        controller: AbortController,
        clock: AlarmClock | undefined,
        current: Generation,
    ) {
        // set what each further start needs, and the start serving now
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
            // go on serving the current generation while the next one starts
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

    /** The served packages, in workload order. */
    get packages(): readonly PackageHandle[] {
        return this.#current.packages;
    }

    /** Report whether the workload serves a package. */
    has(handle: PackageHandle): boolean {
        return this.#current.has(handle);
    }

    /** What the workload does with its hibernated sockets' events, refused without them. */
    requireSocketHandler(): SocketHandler {
        return this.#current.requireSocketHandler();
    }

    /** What the workload does with its store's hook requests, refusing a workload serving none. */
    requireHookHandler(): HookHandler {
        return this.#current.requireHookHandler();
    }

    /** What the workload does with its webhook deliveries, refusing a workload receiving none. */
    requireWebhookHandler(): WebhookHandler {
        return this.#current.requireWebhookHandler();
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

    /** Dispatch a request to a package the current generation serves. */
    fetch(
        handle: PackageHandle,
        request: Request,
        authentication?: Authentication,
    ): Promise<Response> {
        return this.#current.fetch(handle, request, authentication);
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

/** One start of a workload: its servers and handlers, and the resources it releases. */
class Generation {
    /** The served packages and their servers by package id. */
    readonly #packages = new Map<
        string,
        { readonly handle: PackageHandle; readonly server: Server }
    >();
    /** The controller aborting the start's background work at its retirement. */
    readonly #controller = new AbortController();
    /** The resources released after draining. */
    readonly #cleanup = new AsyncDisposableStack();
    /** What the start does with its hibernated sockets' events, absent for a start without them. */
    #sockets: SocketHandler | undefined;
    /** What the start does with its store's hook requests, absent for a start serving none. */
    #hooks: HookHandler | undefined;
    /** What the start does with its webhook deliveries, absent for a start receiving none. */
    #webhooks: WebhookHandler | undefined;
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
                ...(options.server === undefined ? {} : { server: options.server }),
                callKey: options.callKey,
                clock: options.clock,
                ...(options.ephemeral === undefined ? {} : { ephemeral: options.ephemeral }),
                ...(options.hibernation === undefined ? {} : { hibernation: options.hibernation }),
                signal: generation.#controller.signal,
                shutdown: () => host.shutdown(),
                reload: () => host.reload(),
                report: (error) => options.report(error),
                defer: (dispose) => generation.#cleanup.defer(dispose),
            });

            // start a server per package and take the handlers
            for (const served of implementation.packages) {
                generation.#serve(served, options, clock);
            }
            generation.#sockets = implementation.sockets;
            generation.#hooks = implementation.hooks;
            generation.#webhooks = implementation.webhooks;

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

    /** Start the server of one served package, refusing a duplicate. */
    #serve(
        served: PackageImplementation,
        options: WorkloadInstanceOptions,
        clock: AlarmClock | undefined,
    ): void {
        // refuse a cancelled start and a package served twice
        this.#controller.signal.throwIfAborted();
        const handle = served.package;
        if (this.#packages.has(handle.id)) {
            throw new TypeError(`duplicate workload package: ${handle.name}`);
        }

        // start the server, reporting its controllers' failures to the host
        const server = Server.start({
            ...options.serverOptions(handle),
            ...served,
            health: new Health(handle.name),
            resources: options.resources,
            clock: options.clock,
            report: (error) => options.report(error),
            ...(clock === undefined ? {} : { alarm: clock.alarm() }),
        });
        this.#packages.set(handle.id, { handle, server });
    }

    /** The served packages, in workload order. */
    get packages(): readonly PackageHandle[] {
        return [...this.#packages.values()].map((served) => served.handle);
    }

    /** Report whether the start serves a package. */
    has(handle: PackageHandle): boolean {
        return this.#packages.has(handle.id);
    }

    /** What the start does with its hibernated sockets' events, refusing a start without them. */
    requireSocketHandler(): SocketHandler {
        if (this.#sockets === undefined) {
            throw new CallError("NOT_FOUND", { message: "the workload has no socket handler" });
        }

        return this.#sockets;
    }

    /** What the start does with its store's hook requests, refusing a start serving none. */
    requireHookHandler(): HookHandler {
        if (this.#hooks === undefined) {
            throw new CallError("NOT_FOUND", { message: "the workload has no hooks" });
        }

        return this.#hooks;
    }

    /** What the start does with its webhook deliveries, refusing a start receiving none. */
    requireWebhookHandler(): WebhookHandler {
        if (this.#webhooks === undefined) {
            throw new CallError("NOT_FOUND", { message: "the workload receives no webhooks" });
        }

        return this.#webhooks;
    }

    /** Abort the start's background work. */
    shutdown(): void {
        this.#controller.abort();
    }

    /** Run the servers' controllers due by a deadline. */
    async alarm(deadline: number): Promise<void> {
        await Promise.all(
            [...this.#packages.values()].map((served) => served.server.idle(deadline)),
        );
    }

    /** Dispatch a request to a served package. */
    fetch(
        handle: PackageHandle,
        request: Request,
        authentication?: Authentication,
    ): Promise<Response> {
        // require the package
        const server = this.#packages.get(handle.id)?.server;
        if (!server) {
            throw new CallError("NOT_FOUND", {
                message: `unknown workload package: ${handle.name}`,
            });
        }

        return server.fetch(request, authentication);
    }

    /** Drain the servers and release the resources once. */
    close(): Promise<void> {
        this.shutdown();
        this.#closing ??= this.#close();

        return this.#closing;
    }

    /** Close every server and resource, reporting all failures. */
    async #close(): Promise<void> {
        // drain each server
        const servers = [...this.#packages.values()].map((served) => served.server);
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
    /** The server context of the installation the workload runs as, absent outside a space. */
    readonly server?: ServerContext;
    /** Read the key the workload's journals fingerprint sensitive inputs under. */
    readonly callKey: CallKey;
    /** Set a wake-up for the packages' earliest due controller key. */
    readonly alarm?: Alarm;
    /** Where the instance stores its ephemeral objects, absent outside a runner. */
    readonly ephemeral?: EphemeralHost;
    /** The WebSockets the host hibernates for the instance, absent where it hibernates none. */
    readonly hibernation?: Hibernation;
    /** Report a failure of the workload's background work to its host. */
    readonly report: (error: unknown) => void;
    /** The clock the workload runs at. */
    readonly clock: Clock;
    /** Select the server options of one served package. */
    readonly serverOptions: (
        handle: PackageHandle,
    ) => Omit<
        ServerOptions,
        keyof PackageImplementation | "health" | "resources" | "clock" | "report"
    >;
}
