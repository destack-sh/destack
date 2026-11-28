import { TelemetryScope } from "@destack/telemetry";
import { untilAborted, type Environment } from "@destack/environment";
import { Failure } from "@destack/error";
import { present, Uuid } from "@destack/schema";
import { ObjectError } from "../error/error.ts";
import type { ResourceContext } from "@destack/resource/context";
import { Server, type ServerOptions, type PackageHandler, type Voucher } from "../server/index.ts";
import type { Package } from "../declare/package.ts";
import { type Alarm, AlarmClock, AlarmStorage, type ControllerSet } from "@destack/db/database";
import { Health } from "../server/health.ts";

import type {
    EphemeralHost,
    ServerContext,
    Workload,
    WorkloadContext,
    WorkloadImplementation,
} from "./workload.ts";
import { CallKey } from "../audit/key.ts";
import type { Hibernation, Socket, SocketHandler } from "./socket.ts";
import { ObjectConnectionAttachment } from "../server/connection.ts";

import type {} from "@destack/package/import-meta";

/** A running workload instance, serving its current start. */
export class WorkloadInstance implements AsyncDisposable {
    /** The workload the instance runs. */
    readonly #workload: Workload;
    /** The hosting configuration of each start. */
    readonly #options: WorkloadInstanceOptions;
    /** The shutdown controller. */
    readonly #controller: AbortController;
    /** The clock placing the servers' alarms on the host's one wake-up, absent without one. */
    readonly #clock: AlarmClock | undefined;
    /** The start serving requests. */
    #current: WorkloadStart;
    /** The start in progress, absent while none runs. */
    #starting: Promise<void> | undefined;
    /** The one start a reload during the running start folds into, absent while none waits. */
    #next: Promise<void> | undefined;
    /** The retired starts still draining beside the current one. */
    readonly #draining = new Set<Promise<void>>();
    /** The shutdown. */
    #closing?: Promise<void>;
    /** The instance's captures of failures no caller awaits, on its environment's telemetry. */
    readonly #instruments: TelemetryScope;

    /** Create the instance with its workload, hosting, shutdown, clock and first start. */
    private constructor(
        workload: Workload,
        options: WorkloadInstanceOptions,
        controller: AbortController,
        clock: AlarmClock | undefined,
        current: WorkloadStart,
    ) {
        // set what each further start needs, and the start serving now
        this.#workload = workload;
        this.#controller = controller;
        this.#options = options;
        this.#clock = clock;
        this.#current = current;
        this.#instruments = TelemetryScope.of(
            import.meta.destack.package,
            options.environment.telemetry,
        );
    }

    /** Start a workload. */
    static async start(
        workload: Workload,
        options: WorkloadInstanceOptions,
    ): Promise<WorkloadInstance> {
        // start the workload once, reloading through the instance once it exists
        const clock =
            options.alarm === undefined
                ? undefined
                : new AlarmClock(options.alarm, AlarmStorage.memory());
        const controller = new AbortController();
        let instance: WorkloadInstance | undefined;
        const first = await WorkloadStart.start(
            workload,
            options,
            clock,
            {
                signal: controller.signal,
                reload: () => present(instance, "the started instance").reload(),
                shutdown: () => controller.abort(),
            },
            Promise.resolve(),
        );
        instance = new WorkloadInstance(workload, options, controller, clock, first);

        return instance;
    }

    /** Start the workload again, folding reloads during a start into one start after it. */
    reload(): Promise<void> {
        // start at once, else once the running start ends, once for every reload meanwhile
        if (this.#starting === undefined) {
            return this.#start();
        }
        this.#next ??= this.#starting
            .catch(() => undefined)
            .then(() => {
                this.#next = undefined;

                return this.#start();
            });

        return this.#next;
    }

    /** Start the workload again, serving the next start and draining the previous one beside it. */
    #start(): Promise<void> {
        const starting = (async () => {
            // go on serving the current start while the next one starts with its controllers idle
            this.signal.throwIfAborted();
            const activation = Promise.withResolvers<void>();
            const next = await WorkloadStart.start(
                this.#workload,
                this.#options,
                this.#clock,
                {
                    signal: this.signal,
                    reload: () => this.reload(),
                    shutdown: () => this.shutdown(),
                },
                activation.promise,
            );

            // switch to the next start, running one start's controllers at a time
            const previous = this.#current;
            this.#current = next;
            this.#instruments.log.info("destack.workload.reloaded", {
                "destack.workload.name": this.#workload.name,
            });
            await previous.stopControllers();
            activation.resolve();

            // drain the previous start, capturing one that fails to drain
            const draining = previous
                .close()
                .catch((error: unknown) =>
                    this.#instruments.captureException(error, { isEscaped: true }),
                );
            this.#draining.add(draining);
            void draining.then(() => this.#draining.delete(draining));
        })();
        this.#starting = starting.finally(() => {
            this.#starting = undefined;
        });

        return this.#starting;
    }

    /** The shutdown signal. */
    get signal(): AbortSignal {
        return this.#controller.signal;
    }

    /** The served packages, in workload order. */
    get packages(): readonly Package[] {
        return this.#current.packages;
    }

    /** Report whether the workload serves a package. */
    has(handle: Package): boolean {
        return this.#current.has(handle);
    }

    /** Report whether a package implements a request's route of another package's types. */
    implements(handle: Package, protocol: string, request: Request): boolean {
        return this.#current.implements(handle, protocol, request);
    }

    /** What the workload does with its hibernated sockets' events. */
    get sockets(): SocketHandler {
        return this.#current.sockets;
    }

    /** Settle once every package the workload serves serves, its readiness met. */
    ready(): Promise<void> {
        return this.#current.ready();
    }

    /** Read the workload's handler of a kind of host request, refusing a kind it has none for. */
    requireHandler(kind: "hook" | "view"): NonNullable<WorkloadImplementation["hook"]> {
        return this.#current.requireHandler(kind);
    }

    /** Request shutdown. */
    shutdown(): void {
        this.#controller.abort();
    }

    /** Stop the current start's controllers at once, its servers serving until they drain. */
    stopControllers(): Promise<void> {
        return this.#current.stopControllers();
    }

    /** Run every server's due controllers at the host's wake-up, which set their next wake-ups. */
    async alarm(deadline: number): Promise<void> {
        // run them without a clock, else as the clock fires, since its wake-ups live in memory
        const run = () => this.#current.alarm(deadline);
        await (this.#clock === undefined
            ? run()
            : this.#clock.fire(this.#options.environment.clock.now(), run));
    }

    /** Dispatch a request to a package the current start serves. */
    fetch(handle: Package, request: Request, voucher?: Voucher): Promise<Response> {
        return this.#current.fetch(handle, request, voucher);
    }

    /** Close the workload. */
    close(): Promise<void> {
        this.shutdown();
        this.#closing ??= (async () => {
            // close once the starts in progress ended and the retired starts drained
            await (this.#next ?? this.#starting)?.catch(() => undefined);
            await Promise.all(this.#draining);
            await this.#current.close();
        })();

        return this.#closing;
    }

    /** Close the workload. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.close();
    }
}

/** One start of a workload: its servers and handlers, and the resources it releases. */
class WorkloadStart {
    /** The served packages and their servers by package id. */
    readonly #packages = new Map<
        string,
        {
            readonly handle: Package;
            readonly handler: PackageHandler;
            readonly server: Server;
            readonly protocols: ReadonlyMap<string, readonly RegExp[]>;
        }
    >();
    /** How the start serves: its workload, the instance's options, alarm clock and activation. */
    readonly #serving: {
        readonly workload: Workload;
        readonly options: WorkloadInstanceOptions;
        readonly clock: AlarmClock | undefined;
        readonly activation: Promise<void>;
    };
    /** The controller aborting the start's background work at its retirement. */
    readonly #controller = new AbortController();
    /** The servers a serve replaced, draining beside the start. */
    readonly #draining = new Set<Promise<void>>();
    /** The resources released after draining. */
    readonly #cleanup = new AsyncDisposableStack();
    /** The store whose jobs the start runs, absent for none. */
    #store: WorkloadImplementation["store"];
    /** Aborts the store's loop. */
    readonly #stopping = new AbortController();
    /** The store's running loop, settled once it stops. */
    #running: Promise<void> | undefined;
    /** What the start does with its hibernated sockets' events, absent for a start without them. */
    #sockets: SocketHandler | undefined;
    /** The start's handler of each kind of host request it serves. */
    #handlers: Pick<WorkloadImplementation, "hook" | "view"> = {};
    /** The shutdown. */
    #closing?: Promise<void>;
    /** The start's captures of failures no caller awaits, on its environment's telemetry. */
    readonly #instruments: TelemetryScope;

    /** Prepare a start of a workload with its instance's options, alarm clock and activation. */
    private constructor(serving: {
        readonly workload: Workload;
        readonly options: WorkloadInstanceOptions;
        readonly clock: AlarmClock | undefined;
        readonly activation: Promise<void>;
    }) {
        this.#serving = serving;
        this.#instruments = TelemetryScope.of(
            import.meta.destack.package,
            serving.options.environment.telemetry,
        );
    }

    /** Start a workload once, closing what it opened when the start fails. */
    static async start(
        workload: Workload,
        options: WorkloadInstanceOptions,
        clock: AlarmClock | undefined,
        host: Pick<WorkloadContext, "signal" | "reload" | "shutdown">,
        activation: Promise<void>,
    ): Promise<WorkloadStart> {
        // abort the start with its instance
        const start = new WorkloadStart({ workload, options, clock, activation });
        host.signal.addEventListener("abort", () => start.shutdown(), { once: true });
        if (host.signal.aborted) {
            start.shutdown();
        }
        try {
            // start the workload
            const implementation = await workload.start({
                resources: options.resources,
                ...(options.server === undefined ? {} : { server: options.server }),
                callKey: options.callKey,
                environment: options.environment,
                ...(options.ephemeral === undefined ? {} : { ephemeral: options.ephemeral }),
                ...(options.hibernation === undefined ? {} : { hibernation: options.hibernation }),
                signal: start.#controller.signal,
                shutdown: () => host.shutdown(),
                reload: () => host.reload(),
                serve: (packages) => start.serve(packages),
                defer: (dispose) => start.#cleanup.defer(dispose),
            });

            // start a server per package, and run the store's jobs once this start is current
            start.serve(implementation.packages);
            start.#store = implementation.store;
            start.#running = start.#run(implementation.store, implementation.control ?? (() => []));
            start.#sockets = implementation.sockets;
            start.#handlers = implementation;

            // reject a cancelled start
            start.#controller.signal.throwIfAborted();
        } catch (error) {
            // release a failed start
            try {
                await start.close();
            } catch (cleanup) {
                throw new AggregateError([error, cleanup], "workload startup and cleanup failed", {
                    cause: cleanup,
                });
            }
            throw error;
        }

        return start;
    }

    /** Run a store's jobs once the start is current, until stopped. */
    async #run(
        store: WorkloadImplementation["store"],
        sets: () => readonly ControllerSet[],
    ): Promise<void> {
        // wait for the start's turn
        const { options, clock, activation } = this.#serving;
        const signal = AbortSignal.any([this.#controller.signal, this.#stopping.signal]);
        await untilAborted(activation, signal);
        if (store === undefined || signal.aborted) {
            return;
        }

        // run the loop, capturing its failure and marking every server down
        const alarm = clock?.alarm(`store/${Uuid.v4(options.environment.random)}`);
        try {
            await store.run(signal, { sets, ...(alarm === undefined ? {} : { alarm }) });
        } catch (error) {
            this.#instruments.captureException(error, { isEscaped: true });

            // say why with the failure's causes, which a caller of a down server reads
            const causes = Failure.causes(error);
            const message =
                causes.length === 0 ? String(error) : causes.map((each) => each.message).join(": ");
            for (const { server } of this.#packages.values()) {
                server.health.set("down", `the controllers failed: ${message}`);
            }
        }
    }

    /** Serve a package: start its server, else update the server serving it in place. */
    #serve(served: PackageHandler): void {
        // refuse a cancelled start
        this.#controller.signal.throwIfAborted();
        const { options } = this.#serving;
        const handle = served.package;
        const running = this.#packages.get(handle.id)?.server;
        const serverOptions = {
            ...options.serverOptions(handle),
            ...served,
            ...(options.hibernation === undefined ? {} : { hibernation: options.hibernation }),
            health: running?.health ?? new Health(handle.name),
            resources: options.resources,
            environment: options.environment,
        };

        // update the running server, its connections calling the new handler, else start one
        if (running !== undefined) {
            running.update(serverOptions);
        }
        const server = running ?? Server.start(serverOptions);

        // match each protocol route's path and the paths below it, each parameter one segment
        const protocols = new Map(
            [...(served.protocols ?? [])].map(([protocol, routes]) => [
                protocol,
                [...routes].map(
                    (route) =>
                        new RegExp(`^${route.replaceAll(/\{[^}]+\}/gu, "[^/]+")}(?:[/:]|$)`, "u"),
                ),
            ]),
        );
        this.#packages.set(handle.id, { handle, handler: served, server, protocols });
    }

    /** Serve another set of packages, updating changed servers in place, removed ones draining. */
    serve(handlers: readonly PackageHandler[]): void {
        // refuse a package served twice
        const ids = handlers.map((handler) => handler.package.id);
        const twice = handlers.find((handler, index) => ids.indexOf(handler.package.id) !== index);
        if (twice !== undefined) {
            throw new TypeError(`duplicate workload package: '${twice.package.name}'`);
        }

        // take out the servers of packages no longer served
        const kept = new Set<string>(ids);
        const removed = [...this.#packages].filter(([packageId]) => !kept.has(packageId));
        for (const [packageId] of removed) {
            this.#packages.delete(packageId);
        }

        // serve the added and changed packages
        const changed = handlers.filter(
            (handler) => this.#packages.get(handler.package.id)?.handler !== handler,
        );
        for (const handler of changed) {
            this.#serve(handler);
        }

        // log the packages it serves anew and removes
        this.#instruments.log.info("destack.workload.served", {
            "destack.workload.name": this.#serving.workload.name,
            "destack.workload.changed": changed.map((handler) => handler.package.name),
            "destack.workload.removed": removed.map(([, served]) => served.handle.name),
        });

        // drain the removed servers beside the start, which may run the caller's own controller
        for (const [, served] of removed) {
            const draining = served.server
                .close()
                .catch((error: unknown) =>
                    this.#instruments.captureException(error, { isEscaped: true }),
                );
            this.#draining.add(draining);
            void draining.then(() => this.#draining.delete(draining));
        }
    }

    /** Report whether a package implements a request's route of another package's types. */
    implements(handle: Package, protocol: string, request: Request): boolean {
        const { pathname } = new URL(request.url);
        const routes = this.#packages.get(handle.id)?.protocols.get(protocol) ?? [];

        return routes.some((route) => route.test(pathname));
    }

    /** The served packages, in workload order. */
    get packages(): readonly Package[] {
        return [...this.#packages.values()].map((served) => served.handle);
    }

    /** Stop the store's loop at once, while the servers serve their requests until they drain. */
    async stopControllers(): Promise<void> {
        this.#stopping.abort();
        await this.#running;
    }

    /** Report whether the start serves a package. */
    has(handle: Package): boolean {
        return this.#packages.has(handle.id);
    }

    /** Settle once every package the start serves serves, its readiness met. */
    async ready(): Promise<void> {
        await Promise.all([...this.#packages.values()].map(({ server }) => server.ready()));
    }

    /** What the start does with its sockets' events: a package's connections, else its own. */
    get sockets(): SocketHandler {
        const handler = (socket: Socket): SocketHandler => {
            // hand a connection to the server of its package
            const connection = ObjectConnectionAttachment.safeParse(socket.attachment);
            if (connection.success) {
                const served = this.#packages.get(connection.data.connection);

                return present(served, `the server of package '${connection.data.connection}'`)
                    .server.sockets;
            }
            // refuse any other socket without the workload's handler
            else if (this.#sockets === undefined) {
                throw ObjectError.of(
                    "SOCKET_HANDLER_NOT_FOUND",
                    "workload socket handler not found",
                );
            }

            return this.#sockets;
        };

        return {
            open: async (socket) => handler(socket).open?.(socket),
            message: (socket, message) => handler(socket).message(socket, message),
            close: (socket) => handler(socket).close(socket),
        };
    }

    /** Read the start's handler of a kind of host request, refusing a kind it has none for. */
    requireHandler(kind: "hook" | "view"): NonNullable<WorkloadImplementation["hook"]> {
        const handler = this.#handlers[kind];
        if (handler === undefined) {
            throw ObjectError.of("HANDLER_NOT_FOUND", `workload ${kind} not found`);
        }

        return handler;
    }

    /** Abort the start's background work. */
    shutdown(): void {
        this.#controller.abort();
    }

    /** Run the store's keys due by a deadline. */
    async alarm(deadline: number): Promise<void> {
        await this.#store?.idle(deadline);
    }

    /** Dispatch a request to a served package. */
    fetch(handle: Package, request: Request, voucher?: Voucher): Promise<Response> {
        // require the package
        const server = this.#packages.get(handle.id)?.server;
        if (!server) {
            throw ObjectError.of(
                "PACKAGE_NOT_FOUND",
                `workload package '${handle.name}' not found`,
            );
        }

        return server.fetch(request, voucher);
    }

    /** Drain the servers and release the resources once. */
    close(): Promise<void> {
        this.shutdown();
        this.#closing ??= this.#close();

        return this.#closing;
    }

    /** Close every server and resource, reporting all failures. */
    async #close(): Promise<void> {
        // stop the store's loop, then drain each server and the replaced ones still draining
        await this.stopControllers();
        const servers = [...this.#packages.values()].map((served) => served.server);
        const results = await Promise.allSettled(servers.map((server) => server.close()));
        await Promise.all(this.#draining);
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
    /** The environment the workload runs in: its clock, randomness, crypto and network. */
    readonly environment: Environment;
    /** Select the server options of one served package. */
    readonly serverOptions: (
        handle: Package,
    ) => Omit<ServerOptions, keyof PackageHandler | "health" | "resources" | "environment">;
}
