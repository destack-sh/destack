import { present } from "@destack/schema";
import { OriginPath } from "@destack/package";
import { CallFrame } from "../replica/frame.ts";
import { Caller } from "@destack/access";
import { ObjectError } from "../error/error.ts";
import type { Deriver } from "@destack/identity";
import { InstallationRoot } from "./root.ts";
import { Symbolicator } from "@destack/package/stack";
import { copyRequest, Environment } from "@destack/environment";
import { LineExporter } from "@destack/environment/line";
import type { ObjectRemote } from "../replica/remote.ts";
import type { Connector, ResourceDeclaration } from "@destack/resource";
import { ResourceContext } from "@destack/resource/context";
import { TelemetryScope } from "@destack/telemetry";
import type { Telemetry, TelemetryOptions } from "@destack/telemetry/sdk";
import { OtlpExporter } from "@destack/telemetry/otlp";
import type {} from "@destack/package/import-meta";
import { BuildLayer, type BuildReader } from "@destack/package/manifest";
import type { Package } from "../declare/package.ts";

import { refusal } from "../error/report.ts";
import { Egress } from "../declare/egress.ts";
import { PackageMount } from "../declare/mount.ts";
import { WorkloadInstance } from "./instance.ts";
import { HOOK_PATH, PARTITION_HEADER, PARTITION_PARAMETER, type WorkloadStart } from "./start.ts";
import {
    Hibernation,
    SOCKET_HEADER,
    SOCKET_PATH,
    SocketDescription,
    type SocketHost,
} from "./socket.ts";
import type { Alarm } from "@destack/db/database";
import type { EphemeralHost, ServerContext, Workload } from "./workload.ts";
import { CallKey } from "../audit/key.ts";

/** How long a stopping runner drains its requests: below the host's fifteen-second stop timeout. */
const DRAIN_MILLISECONDS = 10_000;

/** The address of the stack package of a workload's holder, relaying the space's access. */
const STACK_ADDRESS = "@destack/stack";

/** What a runner runs: one workload, its package's resources, and the clients of its space. */
export interface WorkloadRunnerOptions {
    /** The workload, its packages each served at its mount, and the name for its telemetry. */
    readonly workload: Workload;
    /** The package's resource declarations by name. */
    readonly resources: Readonly<Record<string, ResourceDeclaration<unknown>>>;
    /** Reach the remote publishing an installation's replicas. */
    readonly remote: (url: string, secret: string) => ObjectRemote;
    /** Set a wake-up for the workload's earliest due controller key. */
    readonly alarm?: Alarm;
    /** Where the instance connects to the sockets its host hibernates. */
    readonly socketHost?: SocketHost;
    /** Where the runtime stores the instance's ephemeral objects. */
    readonly ephemeral: EphemeralHost;
    /** The connectors the runtime supplies by provider code, ahead of the declared connectors. */
    readonly connectors?: Readonly<Record<string, Connector>>;
    /** The environment: the clock, the network and the randomness. */
    readonly environment: Environment;
    /** The host's own log, which the workload prints its records to. */
    readonly log: LineExporter;
}

/** The telemetry a runner exports through, flushed after its work and shut down at its end. */
export type RunnerTelemetry = Environment["telemetry"] & Pick<Telemetry, "shutdown" | "flush">;

/** A workload instance a host started, on any runtime. */
export class WorkloadRunner implements AsyncDisposable {
    /** The host's start message. */
    readonly start: WorkloadStart;
    /** The running instance. */
    readonly instance: WorkloadInstance;
    /** The workload's served packages, by package id. */
    readonly #packages: ReadonlyMap<string, Package>;
    /** The bound resources' clients, disposed once the instance drains. */
    readonly #connections: AsyncDisposableStack;
    /** The WebSockets the host hibernates for the instance. */
    readonly #hibernation: Hibernation;
    /** The telemetry that exports to the host, which relays it to observability. */
    readonly #telemetry: RunnerTelemetry;
    /** The scheduler each export runs under as tracked work. */
    readonly #scheduler: Environment["scheduler"];
    /** Report a failed export of the runner's telemetry, which it cannot record itself. */
    readonly #report: (error: unknown) => void;
    /** Whether the runner stops, its telemetry exporting what remains as it shuts down. */
    #isClosing = false;
    /** The workload's bounded attribute on its spans, durations and logs. */
    readonly #attributes: { readonly "destack.workload.name": string };
    /** Start a partition's runner in this process, absent for a partition's own runner. */
    readonly #startPartition: ((partition: string) => Promise<WorkloadRunner>) | undefined;
    /** The runners of the partitions below this root, started on their first call. */
    readonly #partitions = new Map<string, Promise<WorkloadRunner>>();

    /** Create the runner with a started instance and its telemetry. */
    private constructor(
        start: WorkloadStart,
        instance: WorkloadInstance,
        packages: ReadonlyMap<string, Package>,
        running: RunnerTelemetry,
        connections: AsyncDisposableStack,
        hibernation: Hibernation,
        scheduler: Environment["scheduler"],
        report: (error: unknown) => void,
        attributes: { readonly "destack.workload.name": string },
        startPartition: ((partition: string) => Promise<WorkloadRunner>) | undefined,
    ) {
        // set the instance and what it runs with
        this.start = start;
        this.#attributes = attributes;
        this.#startPartition = startPartition;
        this.instance = instance;
        this.#packages = packages;
        this.#telemetry = running;
        this.#connections = connections;
        this.#hibernation = hibernation;
        this.#scheduler = scheduler;
        this.#report = report;
    }

    /** Start a workload of a build as a host's start asks. */
    static async start(
        runner: WorkloadRunnerOptions,
        start: WorkloadStart,
        build: BuildReader,
        startTelemetry: (
            options: TelemetryOptions,
            environment: Environment,
        ) => Promise<RunnerTelemetry>,
        report: (error: unknown) => void,
    ): Promise<WorkloadRunner> {
        // export the package's telemetry to its host, the instance's environment emitting through it
        const running = await startTelemetry(
            WorkloadRunner.#telemetryOptions(runner, start, build, report),
            runner.environment,
        );
        const environment = Environment.instrument(runner.environment, running);

        // start the instance on the bound resources
        const connections = new AsyncDisposableStack();
        const scope = TelemetryScope.of(import.meta.destack.package, running);
        const attributes = { "destack.workload.name": runner.workload.name };
        try {
            // start the instance in the start's span
            const { instance, hibernation } = await scope.span(
                "destack.workload.start",
                attributes,
                () => WorkloadRunner.#instance(runner, start, build, environment, connections),
                { duration: { attributes } },
            );

            // index the served packages
            const packages = new Map(instance.packages.map((handle) => [handle.id, handle]));

            // set the instance
            scope.log.info("destack.workload.started", attributes);

            // export the start's traces and logs
            const started = new WorkloadRunner(
                start,
                instance,
                packages,
                running,
                connections,
                hibernation,
                runner.environment.scheduler,
                report,
                attributes,
                start.partition === undefined
                    ? (partition) =>
                          WorkloadRunner.start(
                              runner,
                              { ...start, partition },
                              build,
                              startTelemetry,
                              report,
                          )
                    : undefined,
            );
            started.#export();

            return started;
        } catch (error) {
            await connections.disposeAsync();
            await running.shutdown();
            throw error;
        }
    }

    /** Connect a workload's resources and start its instance, with the hibernation it sleeps in. */
    static async #instance(
        runner: WorkloadRunnerOptions,
        start: WorkloadStart,
        build: BuildReader,
        environment: Environment,
        connections: AsyncDisposableStack,
    ): Promise<{
        readonly instance: WorkloadInstance;
        readonly hibernation: Hibernation;
    }> {
        // connect the resources, open the keys, and start the instance
        const resources = await WorkloadRunner.#connect(runner, start, connections);
        const keys = await InstallationRoot.open(
            Uint8Array.fromHex(start.root),
            runner.environment.crypto,
        );
        const callKey = CallKey.of(keys, runner.environment.crypto);
        const stack = Egress.url(start.egress, STACK_ADDRESS);
        const host = runner.socketHost ?? {
            url: Egress.beside(start.egress, SOCKET_PATH),
            fetch: (request: Request) => runner.environment.network.fetch(request),
        };
        const hibernation = new Hibernation(
            host,
            { secret: start.secret },
            environment.telemetry,
            start.partition,
        );
        const instance = await WorkloadInstance.start(runner.workload, {
            resources,
            server: WorkloadRunner.#server(runner, start, { build, stack, resources, keys }),
            callKey: () => callKey,
            environment,
            ...(runner.alarm === undefined ? {} : { alarm: runner.alarm }),
            ephemeral: runner.ephemeral,
            hibernation,
            serverOptions: () => WorkloadRunner.#serve(runner, start),
        });

        return { instance, hibernation };
    }

    /** Export a workload's telemetry to its host, symbolicated by its build. */
    static #telemetryOptions(
        runner: WorkloadRunnerOptions,
        start: WorkloadStart,
        build: BuildReader,
        report: (error: unknown) => void,
    ): TelemetryOptions {
        // post to the host's OTLP path with the runner's secret, resolving frames through the build
        const endpoint = Egress.beside(start.egress, OriginPath.telemetry);
        const headers = () => ({ authorization: `Bearer ${start.secret}` });
        const fetch = (request: Request) => runner.environment.network.fetch(request);
        const exporter = OtlpExporter.http(
            { endpoint, headers, fetch },
            report,
            new Symbolicator(build),
        );

        return exporter.options(runner.workload.package, {
            ratio: start.telemetry.sampling,
            local: runner.log,
            delay: start.telemetry.delay,
        });
    }

    /** Build the server context of a started workload. */
    static #server(
        runner: WorkloadRunnerOptions,
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
            scope: start.scope,
            ...(start.partition === undefined ? {} : { partition: start.partition }),
            build,
            distribution: BuildLayer.empty(),
            settings: start.settings,
            capabilities: start.capabilities,
            resources,
            keys,
            environment: runner.environment,
            remote: runner.remote(stack, start.secret),
            connect: (address, packageId) =>
                runner.remote(
                    `${Egress.url(start.egress, address)}${PackageMount.path(packageId)}`,
                    start.secret,
                ),
            fetch: (request) => runner.environment.network.fetch(request),
            url: start.url,
            at: (address) => ({
                url: Egress.url(start.egress, address),
                fetch: (request) => {
                    const headers = new Headers(request.headers);
                    headers.set("authorization", `Bearer ${start.secret}`);

                    return runner.environment.network.fetch(copyRequest(request, { headers }));
                },
            }),
        };
    }

    /** Aborts once the instance shuts down. */
    get signal(): AbortSignal {
        return this.instance.signal;
    }

    /** Serve a forwarded request, exporting its traces and logs once it is answered. */
    fetch(request: Request): Promise<Response> {
        // hand a call or a socket event of a partition below this root to the partition's runner
        const partition = this.#partitionOf(request);
        if (partition !== undefined) {
            return this.#partition(partition).then((runner) => runner.fetch(request));
        }

        // serve it, then export once it is answered or refused
        const answered = this.#serveRequest(request);
        const exported = () => this.#export();
        void answered.then(exported, exported);

        return answered;
    }

    /** Serve a forwarded request below the mount of one of the workload's packages. */
    #serveRequest(request: Request): Promise<Response> {
        // refuse a request without the host's secret before anything serves it
        if (request.headers.get("authorization") !== `Bearer ${this.start.secret}`) {
            return Promise.resolve(
                refusal(
                    ObjectError.of(
                        "HOST_SECRET_UNAUTHENTICATED",
                        "authentication required: host secret does not match",
                    ),
                    this.#telemetry,
                ),
            );
        }

        // hand a hibernated socket's message or close to the workload
        const { pathname } = new URL(request.url);
        const mounted = PackageMount.of(request);
        const served = mounted === undefined ? undefined : this.#packages.get(mounted.packageId);
        if (pathname.startsWith(`${SOCKET_PATH}/`)) {
            return this.#socket(request);
        }
        // hand the store's request to run a hook to the workload
        else if (pathname === HOOK_PATH && request.method === "POST") {
            return this.#hook(request);
        }
        // serve the package the mount refers to
        else if (mounted !== undefined && served !== undefined) {
            return this.instance.fetch(served, mounted.request);
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

    /** Hand the workload a hibernated socket's message or close, as the host describes it. */
    async #socket(request: Request): Promise<Response> {
        try {
            // hand the event over once the workload serves, its replicas ready as its requests wait for
            await this.instance.ready();
            await this.#hibernation.receive(request, this.instance.sockets);

            return new Response(null, { status: 204 });
        }
        // answer a refusal as it is
        catch (error) {
            return refusal(error, this.#telemetry);
        }
    }

    /** Run the controllers at the host's wake-up, exporting their traces and logs once they ran. */
    alarm(deadline: number): Promise<void> {
        // run them, then export once they ran or failed
        const ran = this.instance.alarm(deadline);
        const exported = () => this.#export();
        void ran.then(exported, exported);

        return ran;
    }

    /** Export the buffered spans and log records as tracked work, as serverless runtimes do. */
    #export(): void {
        // leave what remains to the shutdown once the runner stops
        if (this.#isClosing) {
            return;
        }
        void this.#scheduler.task(() =>
            this.#telemetry.flush(["traces", "logs"]).catch(this.#report),
        );
    }

    /** Hand the workload its store's request to run a hook, answering a refusal as it is. */
    async #hook(request: Request): Promise<Response> {
        try {
            return await this.instance.requireHookHandler().fetch(request);
        } catch (error) {
            return refusal(error, this.#telemetry);
        }
    }

    /** Read the partition below this root a request goes to, absent for the root's own. */
    #partitionOf(request: Request): string | undefined {
        // take the partition a call names, else the one that accepted a socket an event concerns
        const url = new URL(request.url);
        const socket = request.headers.get(SOCKET_HEADER);
        const named =
            request.headers.get(PARTITION_HEADER) ??
            url.searchParams.get(PARTITION_PARAMETER) ??
            (socket === null ? undefined : SocketDescription.parse(JSON.parse(socket)).partition);

        // leave a call for the root, its scope, or for a partition's own runner
        const isRoot = named === undefined || named === this.start.scope;

        return isRoot || this.#startPartition === undefined ? undefined : named;
    }

    /** Start a partition's runner below this root once, forgetting a failed start. */
    #partition(partition: string): Promise<WorkloadRunner> {
        // join a start in progress or done
        const known = this.#partitions.get(partition);
        if (known !== undefined) {
            return known;
        }

        // start it in this process, as the cloud starts one object per partition
        const started = present(this.#startPartition, "the root's partition start")(partition);
        this.#partitions.set(partition, started);
        started.catch(() => this.#partitions.delete(partition));

        return started;
    }

    /** Drain and stop the instance with its resources and its remaining telemetry. */
    async close(): Promise<void> {
        // stop the instance and its resources in the stop's span, then log the stop and flush
        const scope = TelemetryScope.of(import.meta.destack.package, this.#telemetry);
        const attributes = this.#attributes;
        try {
            await scope.span(
                "destack.workload.stop",
                attributes,
                async () => {
                    // stop the root, then each partition started below it
                    await this.instance.close();
                    await this.#connections.disposeAsync();
                    const started = await Promise.allSettled(this.#partitions.values());
                    for (const result of started) {
                        if (result.status === "fulfilled") {
                            await result.value.close();
                        }
                    }
                },
                { duration: { attributes } },
            );
            scope.log.info("destack.workload.stopped", attributes);
        } finally {
            this.#isClosing = true;
            await this.#telemetry.shutdown();
        }
    }

    /** Close the runner at the end of its scope. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.close();
    }

    /** Connect each bound resource through its declaration's connector for its provider. */
    static async #connect(
        runner: WorkloadRunnerOptions,
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
                declaration.kind.name !== binding.kind ||
                connector === undefined
            ) {
                throw ObjectError.of(
                    "BINDING_NOT_FOUND",
                    `${binding.kind} '${name}' connecting to provider '${binding.provider}' not found`,
                );
            }

            // connect to the bound resource
            const client = connections.use(
                await connector.connect(binding, declaration, runner.environment),
            );
            resources.bind(declaration, client);
        }

        return resources;
    }

    /** Serve the workload's packages to the callers the host forwards. */
    static #serve(runner: WorkloadRunnerOptions, start: WorkloadStart) {
        return {
            clock: runner.environment.clock,
            scope: start.scope,
            instance: start.instance,
            drainTimeout: DRAIN_MILLISECONDS,
            authenticate: async (request: Request) => {
                // answer a call its host refused as that refusal, else read the caller it forwarded
                const refused = CallFrame.refusal(request);
                if (refused !== undefined) {
                    throw refused;
                }

                return Caller.forwarded(request);
            },
            authorizeMachine: async ({
                access,
            }: {
                readonly access: { readonly authentication: string };
            }) => {
                if (access.authentication === "machine") {
                    throw ObjectError.of(
                        "MACHINE_PROCEDURE_DENIED",
                        "machine procedure is forbidden: a workload cannot serve machine procedures",
                    );
                }
            },
        };
    }
}
