import { Caller } from "@destack/access";
import type { Deriver } from "@destack/identity";
import { InstallationRoot } from "./root.ts";
import { Symbolicator } from "@destack/package/stack";
import { copyRequest } from "../request/request.ts";
import type { Uplink } from "@destack/sync";
import type { Connector, ResourceDeclaration } from "@destack/resource";
import { ResourceContext } from "@destack/resource/context";
import { telemetry } from "@destack/telemetry";
import type { Telemetry, TelemetryOptions } from "@destack/telemetry/sdk";
import { OTLP_ORIGIN_PATH, OtlpExporter } from "@destack/telemetry/otlp";
import type {} from "@destack/package/import-meta";
import type { BuildReader } from "@destack/package/manifest";
import type { PackageHandle } from "../declare/package.ts";
import { CallError } from "../error/index.ts";
import { Egress } from "../runtime/egress.ts";
import { PackageMount } from "../runtime/mount.ts";
import { WorkloadInstance } from "./instance.ts";
import { HOOK_PATH, ROUTE_PATH, WEBHOOK_PATH, type WorkloadStart } from "./start.ts";
import {
    SocketDescription,
    Hibernation,
    SOCKET_HEADER,
    SOCKET_PATH,
    type SocketHost,
} from "./socket.ts";
import type { PackageId } from "@destack/package";
import { refusal } from "../server/error.ts";
import type { Alarm } from "../control/index.ts";
import type { EphemeralHost, ServerContext, Workload } from "./workload.ts";
import { CallKey } from "../request/index.ts";
import { Clock } from "@destack/environment";

/** How long a stopping runner drains its requests: below the host's fifteen-second stop timeout. */
const DRAIN_MILLISECONDS = 10_000;

/** The address of the stack package of a workload's holder, relaying the space's access. */
const STACK_ADDRESS = "@destack/stack";

/** The runner's log records. */
const { log } = telemetry.scope(import.meta.destack.package);

/** What a runner runs: one workload, its package's resources, and the clients of its space. */
export interface RunnerOptions {
    /** The workload, its packages each served at its mount, and the name for its telemetry. */
    readonly workload: Workload;
    /** The package's resource declarations by name. */
    readonly resources: Readonly<Record<string, ResourceDeclaration<unknown>>>;
    /** Connect to a publisher of an installation's replicas. */
    readonly publisher: (url: string, secret: string) => Uplink;
    /** Set a wake-up for the workload's earliest due controller key. */
    readonly alarm?: Alarm;
    /** Where the instance connects to the sockets its host hibernates. */
    readonly socketHost?: SocketHost;
    /** Where the runtime stores the instance's ephemeral objects. */
    readonly ephemeral: EphemeralHost;
    /** The connectors the runtime supplies by provider code, ahead of the declarations' own. */
    readonly connectors?: Readonly<Record<string, Connector>>;
    /** The clock. */
    readonly clock: Clock;
}

/** A workload instance a host started, on any runtime. */
export class WorkloadRunner implements AsyncDisposable {
    /** The host's start message. */
    readonly start: WorkloadStart;
    /** The running instance. */
    readonly instance: WorkloadInstance;
    /** The workload's served packages, by package id. */
    readonly #packages: ReadonlyMap<string, PackageHandle>;
    /** The running package, which routed requests reach. */
    readonly #packageId: PackageId;
    /** The bound resources' clients, disposed once the instance drains. */
    readonly #connections: AsyncDisposableStack;
    /** The WebSockets the host hibernates for the instance. */
    readonly #hibernation: Hibernation;
    /** The telemetry that exports to the host, which relays it to observability. */
    readonly #telemetry: Pick<Telemetry, "shutdown">;
    /** Create the runner with a started instance and its telemetry. */
    private constructor(
        start: WorkloadStart,
        instance: WorkloadInstance,
        packages: ReadonlyMap<string, PackageHandle>,
        running: Pick<Telemetry, "shutdown">,
        packageId: PackageId,
        connections: AsyncDisposableStack,
        hibernation: Hibernation,
    ) {
        // set the instance and what it runs with
        this.start = start;
        this.instance = instance;
        this.#packages = packages;
        this.#packageId = packageId;
        this.#telemetry = running;
        this.#connections = connections;
        this.#hibernation = hibernation;
    }

    /** Start a workload of a build as a host's start asks. */
    static async start(
        runner: RunnerOptions,
        start: WorkloadStart,
        build: BuildReader,
        startTelemetry: (options: TelemetryOptions) => Promise<Pick<Telemetry, "shutdown">>,
        report: (error: unknown) => void,
    ): Promise<WorkloadRunner> {
        // export the package's telemetry to its host
        const running = await startTelemetry(
            WorkloadRunner.#telemetryOptions(runner, start, build, report),
        );

        // start the instance on the bound resources
        const connections = new AsyncDisposableStack();
        try {
            const resources = await WorkloadRunner.#connect(runner, start, connections);
            const keys = await InstallationRoot.open(Uint8Array.fromHex(start.root));
            const callKey = CallKey.of(keys);
            const stack = Egress.url(start.egress, STACK_ADDRESS);
            const host = runner.socketHost ?? {
                url: Egress.beside(start.egress, SOCKET_PATH),
                fetch: (request: Request) => fetch(request),
            };
            const hibernation = new Hibernation(host, start.secret, report);
            const instance = await WorkloadInstance.start(runner.workload, {
                resources,
                server: WorkloadRunner.#server(runner, start, { build, stack, resources, keys }),
                callKey: () => callKey,
                clock: runner.clock,
                report,
                ...(runner.alarm === undefined ? {} : { alarm: runner.alarm }),
                ephemeral: runner.ephemeral,
                hibernation,
                serverOptions: () => WorkloadRunner.#serve(runner, start),
            });

            // index the served packages
            const packages = new Map(instance.packages.map((handle) => [handle.id, handle]));

            // set the instance
            log.info("workload.started", {
                workload: runner.workload.name,
                instance: start.instance,
            });

            return new WorkloadRunner(
                start,
                instance,
                packages,
                running,
                runner.workload.package.id,
                connections,
                hibernation,
            );
        } catch (error) {
            await connections.disposeAsync();
            await running.shutdown();
            throw error;
        }
    }

    /** Export a workload's telemetry to its host, symbolicated by its build. */
    static #telemetryOptions(
        runner: RunnerOptions,
        start: WorkloadStart,
        build: BuildReader,
        report: (error: unknown) => void,
    ): TelemetryOptions {
        // post to the host's OTLP path with the runner's secret, resolving frames through the build
        const endpoint = Egress.beside(start.egress, OTLP_ORIGIN_PATH);
        const headers = () => ({ authorization: `Bearer ${start.secret}` });
        const exporter = OtlpExporter.http({ endpoint, headers }, report, new Symbolicator(build));

        return exporter.options(runner.workload.package, { ratio: start.sampling });
    }

    /** Build the server context of a started workload. */
    static #server(
        runner: RunnerOptions,
        start: WorkloadStart,
        bound: {
            readonly build: BuildReader;
            readonly stack: string;
            readonly resources: ResourceContext;
            readonly keys: Deriver;
        },
    ): ServerContext {
        const { build, stack, resources, keys } = bound;

        return {
            installation: start.installation,
            name: start.name,
            space: start.space,
            ...(start.partition === undefined ? {} : { partition: start.partition }),
            build,
            settings: start.settings,
            capabilities: start.capabilities,
            resources,
            keys,
            publisher: runner.publisher(stack, start.secret),
            publisherAt: (address, packageId) =>
                runner.publisher(
                    `${Egress.url(start.egress, address)}${PackageMount.path(packageId)}`,
                    start.secret,
                ),
            fetch: (request) => fetch(request),
            url: start.url,
            at: (address) => ({
                url: Egress.url(start.egress, address),
                fetch: (request) => {
                    const headers = new Headers(request.headers);
                    headers.set("authorization", `Bearer ${start.secret}`);

                    return fetch(copyRequest(request, { headers }));
                },
            }),
        };
    }

    /** Aborts once the instance shuts down. */
    get signal(): AbortSignal {
        return this.instance.signal;
    }

    /** Serve a forwarded request below the mount of one of the workload's packages. */
    fetch(request: Request): Promise<Response> {
        // refuse a request without the host's secret before anything serves it
        if (request.headers.get("authorization") !== `Bearer ${this.start.secret}`) {
            return Promise.resolve(
                refusal(new CallError("UNAUTHORIZED", { message: "invalid host secret" })),
            );
        }

        // hand a webhook delivery to the workload
        const { pathname } = new URL(request.url);
        const routed = PackageMount.of(request);
        const served = routed === undefined ? undefined : this.#packages.get(routed.packageId);
        if (pathname.startsWith(`${WEBHOOK_PATH}/`) && request.method === "POST") {
            return this.#receive(request, pathname.slice(WEBHOOK_PATH.length));
        }
        // hand a hibernated socket's message or close to the workload
        else if (pathname.startsWith(`${SOCKET_PATH}/`)) {
            return this.#socket(request);
        }
        // hand the store's request to run a hook to the workload
        else if (pathname === HOOK_PATH && request.method === "POST") {
            return this.#hook(request);
        }
        // hand a routed request to the workload's own package
        else if (pathname === ROUTE_PATH) {
            return this.#route(request);
        }
        // serve the package the mount refers to
        else if (routed !== undefined && served !== undefined) {
            return this.instance.fetch(served, routed.request);
        }
        // answer nothing for any other path
        else {
            return Promise.resolve(new Response(null, { status: 404 }));
        }
    }

    /** Request shutdown. */
    shutdown(): void {
        this.instance.shutdown();
    }

    /** Hand a routed request to the workload's own package at the route path. */
    #route(request: Request): Promise<Response> {
        // require the workload to serve its own package
        const served = this.#packages.get(this.#packageId);
        if (served === undefined) {
            return Promise.resolve(
                refusal(new CallError("NOT_FOUND", { message: "no package serves routes here" })),
            );
        }

        return this.instance.fetch(served, request);
    }

    /** Hand the workload a hibernated socket's message or close, as the host describes it. */
    async #socket(request: Request): Promise<Response> {
        try {
            // read the socket the host describes
            const header = request.headers.get(SOCKET_HEADER);
            const description = SocketDescription.parse(JSON.parse(header ?? "null"));
            const socket = this.#hibernation.socket(description);
            const handler = this.instance.requireSocketHandler();

            // hand over its close or its message
            if (request.method === "DELETE") {
                await handler.close(socket);
            } else {
                await handler.message(socket, new Uint8Array(await request.arrayBuffer()));
            }

            // answer once the sends the event caused reached the host
            await this.#hibernation.settled(description.id);

            return new Response(null, { status: 204 });
        }
        // answer a refusal as it is
        catch (error) {
            return refusal(error);
        }
    }

    /** Run the controllers at the host's wake-up. */
    alarm(deadline: number): Promise<void> {
        return this.instance.alarm(deadline);
    }

    /** Hand the workload its store's request to run a hook, answering a refusal as it is. */
    async #hook(request: Request): Promise<Response> {
        try {
            return await this.instance.requireHookHandler().fetch(request);
        } catch (error) {
            return refusal(error);
        }
    }

    /** Hand the workload a webhook delivery at its path below the webhook path. */
    async #receive(request: Request, below: string): Promise<Response> {
        try {
            const url = new URL(request.url);
            url.pathname = below;

            return await this.instance
                .requireWebhookHandler()
                .fetch(new Request(url.href, request));
        } catch (error) {
            return refusal(error);
        }
    }

    /** Drain and stop the instance with its resources and its remaining telemetry. */
    async close(): Promise<void> {
        try {
            await this.instance.close();
            await this.#connections.disposeAsync();
        } finally {
            await this.#telemetry.shutdown();
        }
    }

    /** Close the runner at the end of its scope. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.close();
    }

    /** Connect each bound resource through its declaration's connector for its provider. */
    static async #connect(
        runner: RunnerOptions,
        start: WorkloadStart,
        connections: AsyncDisposableStack,
    ): Promise<ResourceContext> {
        // connect each binding the start lists
        const resources = new ResourceContext();
        for (const [name, binding] of Object.entries(start.bindings)) {
            // require the declaration of the binding's kind and its connector for the provider
            const declaration = runner.resources[name];
            const connector =
                runner.connectors?.[binding.provider] ?? declaration?.connectors[binding.provider];
            if (
                declaration === undefined ||
                declaration.kind !== binding.kind ||
                connector === undefined
            ) {
                throw new CallError("NOT_FOUND", {
                    message: `no ${binding.kind} ${name} connecting to provider ${binding.provider}`,
                });
            }

            // connect to the bound resource
            const client = connections.use(await connector.connect(binding, declaration));
            resources.bind(declaration, client);
        }

        return resources;
    }

    /** Serve the workload's packages to the callers the host forwards. */
    static #serve(runner: RunnerOptions, start: WorkloadStart) {
        return {
            clock: runner.clock,
            scope: start.space,
            instance: start.instance,
            drainTimeout: DRAIN_MILLISECONDS,
            authenticate: async (request: Request) => Caller.forwarded(request),
            authorizeMachine: async ({
                access,
            }: {
                readonly access: { readonly authentication: string };
            }) => {
                if (access.authentication === "machine") {
                    throw new CallError("FORBIDDEN", {
                        message: "a workload cannot serve machine procedures",
                    });
                }
            },
        };
    }
}
