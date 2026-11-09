import { waitAborted } from "@destack/environment";
import { present, Uuid } from "@destack/schema";
import { ObjectError } from "../error/error.ts";
import type { Authentication } from "@destack/access";
import type { ResourceContext } from "@destack/resource/context";
import { Server, type ServerOptions, type PackageHandler } from "../server/index.ts";
import type { PackageHandle } from "../declare/package.ts";
import { type Alarm, AlarmClock, AlarmStorage } from "@destack/db";
import { Health } from "../server/health.ts";

import type {
    EphemeralHost,
    HookHandler,
    InstalledServerContext,
    Workload,
    WorkloadContext,
} from "./workload.ts";
import { CallKey } from "../audit/key.ts";
import type { Hibernation, SocketHandler } from "./socket.ts";
import type { Environment } from "@destack/environment";

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
            await previous.stopControllers();
            activation.resolve();

            // drain the previous start, reporting one that fails to drain
            const draining = previous
                .close()
                .catch((error: unknown) => this.#options.report(error));
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
    get packages(): readonly PackageHandle[] {
        return this.#current.packages;
    }

    /** Report whether the workload serves a package. */
    has(handle: PackageHandle): boolean {
        return this.#current.has(handle);
    }

    /** Report whether a package implements a request's route of another package's types. */
    implements(handle: PackageHandle, protocol: string, request: Request): boolean {
        return this.#current.implements(handle, protocol, request);
    }

    /** What the workload does with its hibernated sockets' events, refused without them. */
    requireSocketHandler(): SocketHandler {
        return this.#current.requireSocketHandler();
    }

    /** Settle once every package the workload serves serves, its readiness met. */
    ready(): Promise<void> {
        return this.#current.ready();
    }

    /** What the workload does with its store's hook requests, refusing a workload serving none. */
    requireHookHandler(): HookHandler {
        return this.#current.requireHookHandler();
    }

    /** Request shutdown. */
    shutdown(): void {
        this.#controller.abort();
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
            readonly handle: PackageHandle;
            readonly handler: PackageHandler;
            readonly server: Server;
            readonly protocols: ReadonlyMap<string, readonly RegExp[]>;
        }
    >();
    /** How the start serves its packages: the instance's options, alarm clock and activation. */
    readonly #serving: {
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
    /** What the start does with its hibernated sockets' events, absent for a start without them. */
    #sockets: SocketHandler | undefined;
    /** What the start does with its store's hook requests, absent for a start serving none. */
    #hooks: HookHandler | undefined;
    /** The shutdown. */
    #closing?: Promise<void>;

    /** Prepare a start serving packages with its instance's options, alarm clock and activation. */
    private constructor(serving: {
        readonly options: WorkloadInstanceOptions;
        readonly clock: AlarmClock | undefined;
        readonly activation: Promise<void>;
    }) {
        this.#serving = serving;
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
        const start = new WorkloadStart({ options, clock, activation });
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
                report: (error) => options.report(error),
                defer: (dispose) => start.#cleanup.defer(dispose),
            });

            // start a server per package and take the handlers, replacing none in this start
            for (const served of implementation.packages) {
                start.#serve(served, Promise.resolve());
            }
            start.#sockets = implementation.sockets;
            start.#hooks = implementation.hooks;

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

    /** Start a served package's server once the servers it replaces stopped their controllers. */
    #serve(served: PackageHandler, handover: Promise<unknown>): void {
        // refuse a cancelled start and a package served twice
        this.#controller.signal.throwIfAborted();
        const { options, clock, activation } = this.#serving;
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
            environment: options.environment,
            report: (error) => options.report(error),
            // name each server's alarm uniquely, as a reload serves a package beside its draining server
            ...(clock === undefined
                ? {}
                : { alarm: clock.alarm(`${handle.id}/${Uuid.v4(options.environment.random)}`) }),
            activation: async (signal) => {
                // wait for this start's turn and the replaced controllers' stop, then the package's own activation
                await Promise.race([Promise.all([activation, handover]), waitAborted(signal)]);
                if (!signal.aborted) {
                    await served.activation?.(signal);
                }
            },
        });
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

    /** Serve another set of packages, keeping unchanged servers, the replaced ones draining. */
    serve(handlers: readonly PackageHandler[]): void {
        // take out the servers of packages no longer served or served by another handler
        const replaced = [...this.#packages].filter(
            ([, served]) => !handlers.includes(served.handler),
        );
        for (const [packageId] of replaced) {
            this.#packages.delete(packageId);
        }

        // serve the added and changed packages once the replaced controllers stopped, as one key has one writer
        const handover = Promise.all(replaced.map(([, served]) => served.server.stopControllers()));
        for (const handler of handlers.filter((each) => !this.#packages.has(each.package.id))) {
            this.#serve(handler, handover);
        }

        // drain the replaced servers beside the start, which may run the caller's own controller
        for (const [, served] of replaced) {
            const draining = served.server
                .close()
                .catch((error: unknown) => this.#serving.options.report(error));
            this.#draining.add(draining);
            void draining.then(() => this.#draining.delete(draining));
        }
    }

    /** Report whether a package implements a request's route of another package's types. */
    implements(handle: PackageHandle, protocol: string, request: Request): boolean {
        const { pathname } = new URL(request.url);
        const routes = this.#packages.get(handle.id)?.protocols.get(protocol) ?? [];

        return routes.some((route) => route.test(pathname));
    }

    /** The served packages, in workload order. */
    get packages(): readonly PackageHandle[] {
        return [...this.#packages.values()].map((served) => served.handle);
    }

    /** Stop every server's controllers at once, while it serves its requests until it drains. */
    async stopControllers(): Promise<void> {
        await Promise.all(
            [...this.#packages.values()].map(({ server }) => server.stopControllers()),
        );
    }

    /** Report whether the start serves a package. */
    has(handle: PackageHandle): boolean {
        return this.#packages.has(handle.id);
    }

    /** Settle once every package the start serves serves, its readiness met. */
    async ready(): Promise<void> {
        await Promise.all([...this.#packages.values()].map(({ server }) => server.ready()));
    }

    /** What the start does with its hibernated sockets' events, refusing a start without them. */
    requireSocketHandler(): SocketHandler {
        if (this.#sockets === undefined) {
            throw ObjectError.of("SOCKET_HANDLER_MISSING", "the workload has no socket handler");
        }

        return this.#sockets;
    }

    /** What the start does with its store's hook requests, refusing a start serving none. */
    requireHookHandler(): HookHandler {
        if (this.#hooks === undefined) {
            throw ObjectError.of("HOOKS_MISSING", "the workload has no hooks");
        }

        return this.#hooks;
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
            throw ObjectError.of("PACKAGE_UNKNOWN", `unknown workload package: ${handle.name}`);
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
        // drain each server, and the replaced ones still draining
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
    readonly server?: InstalledServerContext;
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
    /** The environment the workload runs in: its clock, randomness, crypto and network. */
    readonly environment: Environment;
    /** Select the server options of one served package. */
    readonly serverOptions: (
        handle: PackageHandle,
    ) => Omit<
        ServerOptions,
        keyof PackageHandler | "health" | "resources" | "environment" | "report"
    >;
}
