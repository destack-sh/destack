import type { Program } from "@destack/package/bun";
import type { PackageRelease } from "@destack/package";
import { RequestBody, SERVER_EXPORT } from "@destack/object";
import { Authentication } from "@destack/access";
import { join } from "node:path";
import { type Environment, Pipe } from "@destack/environment";
import { BuildTree, type LocalBuildStore } from "@destack/build";
import { PackageFile } from "@destack/package/file";
import { BuildImage, PackageManifest } from "@destack/package/manifest";
import { Sandbox, type SandboxOptions } from "@destack/sandbox";
import { type Digest, type Identifier, JsonValue, present } from "@destack/schema";
import type { Deriver } from "@destack/identity";
import {
    PARTITION_HEADER,
    WORKLOAD_EXPORT,
    WorkloadReady,
    type WorkloadStart,
    type WorkloadTelemetry,
} from "@destack/object/workload";
import type { BunWorkload } from "@destack/object/bun";
import {
    type InstanceExit,
    InstanceSpec,
    InstanceRegistry,
    type Runtime,
    WorkloadRequest,
} from "../runtime/runtime.ts";
import { Failure } from "@destack/error";
import { type BunInstanceLayout, BunWorkloadSandbox } from "./sandbox.ts";
import type {} from "@destack/package/import-meta";
import { TelemetryScope } from "@destack/telemetry";
import { MachineError } from "../error/index.ts";
import { PackageImplementation } from "@destack/object/server";

/** The server runtime this runtime runs instances on. */
const BUN_RUNTIME = "bun";

/** How long a runner may take to serve, in milliseconds. */
const START_TIMEOUT_MILLISECONDS = 10_000;

/** The origin of the requests a runner serves on its Unix socket, which the socket alone routes. */
const RUNNER_ORIGIN = "http://localhost";

/** How long a stopping runner may drain before it is killed, in milliseconds. */
const STOP_TIMEOUT_MILLISECONDS = 15_000;

/** How long a call of a build's export may run, in milliseconds. */
const CALL_TIMEOUT_MILLISECONDS = 10_000;

/** The program a sandboxed call runs: import the module, call the export, print its JSON. */
const CALL_PROGRAM = `
const { module, name, input } = JSON.parse(await Bun.stdin.text());
const exported = (await import(module))[name];
if (typeof exported !== "function") {
    throw new TypeError(\`\${module} exports no function \${name}\`);
}
process.stdout.write(JSON.stringify((await exported(...input)) ?? null));
`;

/** The workload a Bun build's entry exports, which a runtime starts in this process. */
type HostedWorkload = Pick<BunWorkload, "start">;

/** How a runtime isolates its runners: in sandboxed processes, or in this process. */
export type BunIsolation = "sandbox" | "host";

/** A runner of one instance. */
interface Runner {
    /** The instance's spec. */
    readonly spec: InstanceSpec;
    /** Report an exit nobody asked for. */
    readonly exited: (exit: InstanceExit) => Promise<void>;
    /** The secret the machine and the runner prove each other's requests with. */
    readonly secret: string;
    /** Send a request to the runner. */
    readonly send: (request: Request) => Promise<Response>;
    /** Drain the runner, stopping it once the drain runs out. */
    readonly stop: () => Promise<void>;
}

/** Run instances on Bun on this machine, each sandboxed or in this process. */
export class BunRuntime implements Runtime {
    /** The server runtime. */
    readonly name = BUN_RUNTIME;
    /** The directory of each instance's files. */
    readonly #directory: string;
    /** The machine's store of builds, with every build's closure. */
    readonly #packages: LocalBuildStore;
    /** The modules this process shares with the servers it loads: the builtins. */
    readonly #program: Program;
    /** The machine's egress of a space's instances, below which they call addresses. */
    readonly #egress: (scope: Identifier<"space">) => string;
    /** Resolve how an installation samples and batches its telemetry. */
    readonly #workloadTelemetry: (
        scope: Identifier<"space">,
        installation: Identifier<"installation">,
    ) => Promise<WorkloadTelemetry>;
    /** Resolve the public URL an installation's instances serve at. */
    readonly #url: (
        scope: Identifier<"space">,
        installation: Identifier<"installation">,
    ) => Promise<string>;
    /** Record a line a runner writes to standard error. */
    readonly #output: (spec: InstanceSpec, line: string) => void;
    /** Read the identity of an instance's space, which its installation's root derives from. */
    readonly #identity: (scope: Identifier<"space">) => Promise<Pick<Deriver, "derive">>;
    /** The runners starting and running. */
    readonly #registry: InstanceRegistry<Runner>;
    /** Each image's tree, by digest, shared by its starts and calls. */
    readonly #trees = new Map<Digest, Promise<BuildTree>>();
    /** The builtin releases the runtime supplies. */
    readonly #builtins: readonly PackageRelease[];
    /** The host the runners run on: its files, processes and clock. */
    readonly #environment: Environment<"files" | "process" | "host">;
    /** How the runners are isolated. */
    readonly #isolation: BunIsolation;

    /** The runtime's spans and durations. */
    readonly #scope: TelemetryScope;

    /** Run instances below a directory, calling addresses through the machine's egress. */
    constructor(options: {
        /** The directory of each instance's files. */
        readonly directory: string;
        /** The machine's store of builds, with every build's closure. */
        readonly packages: LocalBuildStore;
        /** The builtin releases the runtime supplies. */
        readonly builtins: readonly PackageRelease[];
        /** The modules this process shares with the servers it loads: the builtins. */
        readonly program: Program;
        /** The machine's egress of a space's instances, below which they call addresses. */
        readonly egress: (scope: Identifier<"space">) => string;
        /** Resolve how an installation samples and batches its telemetry. */
        readonly workloadTelemetry: (
            scope: Identifier<"space">,
            installation: Identifier<"installation">,
        ) => Promise<WorkloadTelemetry>;
        /** Resolve the public URL an installation's instances serve at. */
        readonly url: (
            scope: Identifier<"space">,
            installation: Identifier<"installation">,
        ) => Promise<string>;
        /** Record a line a runner writes to standard error. */
        readonly output: (spec: InstanceSpec, line: string) => void;
        /** Read the identity of an instance's space, which its installation's root derives from. */
        readonly identity: (scope: Identifier<"space">) => Promise<Pick<Deriver, "derive">>;
        /** The host the runners run on: its files, processes and clock. */
        readonly environment: Environment<"files" | "process" | "host">;
        /** How the runners are isolated. */
        readonly isolation: BunIsolation;
    }) {
        // set the directory, the egress, the output and the spaces' identities
        this.#directory = options.directory;
        this.#packages = options.packages;
        this.#builtins = options.builtins;
        this.#program = options.program;
        this.#egress = options.egress;
        this.#workloadTelemetry = options.workloadTelemetry;
        this.#url = options.url;
        this.#output = options.output;
        this.#identity = options.identity;
        this.#environment = options.environment;
        this.#isolation = options.isolation;
        this.#scope = TelemetryScope.of(import.meta.destack.package, options.environment.telemetry);
        this.#registry = new InstanceRegistry(BUN_RUNTIME, options.environment.telemetry);
    }

    /** Load the server an image's root exports into this process, absent for none. */
    async server(image: Digest): Promise<PackageImplementation | undefined> {
        // take none of a root without a server export
        const build = await this.#packages.root(image);
        const output = Object.values(build.manifest.outputs).find(
            (each) => each.runtime === BUN_RUNTIME,
        );
        if (output?.exports[SERVER_EXPORT] === undefined) {
            return undefined;
        }

        // import it, refusing a module exporting no package implementation
        const module = await this.import(image, SERVER_EXPORT);
        const exported =
            typeof module === "object" && module !== null && "default" in module
                ? module.default
                : undefined;
        if (!PackageImplementation.is(exported)) {
            throw MachineError.of(
                "SERVER_BROKEN",
                `'${build.manifest.package.name}' exports no package implementation as its server`,
            );
        }

        return exported;
    }

    /** Import an export of an image's root's Bun output, with this process's builtins. */
    import(image: Digest, specifier: string): Promise<unknown> {
        return this.#scope.span("destack.runtime.import", {}, async () => {
            // find the export in the root's Bun output
            const read = await this.#packages.image(image);
            const build = await this.#packages.contents(read.root);
            const output = present(
                Object.values(build.manifest.outputs).find((each) => each.runtime === BUN_RUNTIME),
                `the ${BUN_RUNTIME} output of ${build.manifest.package.name}`,
            );
            const exported = present(
                output.exports[specifier],
                `the ${specifier} export of ${build.manifest.package.name}`,
            );

            // import a builtin's export as this process's own instance in one real operation, never a second copy
            const shared = BunRuntime.#shared(build.manifest.package.name, specifier);
            if (this.#program.shares(shared)) {
                return await this.#environment.scheduler.run(() => this.#program.module(shared));
            }

            // link the builtins to this process's own and import the export
            const directory = await this.#build(image);
            const imports = await this.#imports(read);

            // load the modules as one real operation, completed in order whether the host loaded them before or not
            return await this.#environment.scheduler.run(async () => {
                await this.#program.link(imports);
                const module: unknown = await import(join(directory, exported.path));

                return module;
            });
        });
    }

    /** List the modules the builds Bun loads of an image may load: imports and Bun exports. */
    async #imports(image: BuildImage): Promise<string[]> {
        // read each build Bun loads once, as placing it did
        const loaded = await BuildImage.loads(
            image,
            BUN_RUNTIME,
            (needed) => this.#packages.get(needed),
            "export",
        );
        const manifests = await Promise.all(
            [...loaded].map(async (each) =>
                present(await this.#packages.get(each), `build ${each} Bun loads`),
            ),
        );

        // take every module the outputs import, and each build's exports but its workloads' entries and tests
        return manifests.flatMap((built) => {
            // read the build's Bun outputs and the modules they import
            const { name } = built.package;
            const outputs = Object.values(built.outputs).filter(
                (output) => output.runtime === BUN_RUNTIME,
            );
            const imported = outputs.flatMap((output) =>
                Object.values(output.exports).flatMap((entry) => entry.imports),
            );

            // leave a workload's entry and a test subpath, which no placed copy answers
            const exported = outputs.flatMap((output) => {
                const entries = new Set(
                    Object.values(output.workloads).map((workload) => workload.entrypoint),
                );

                return Object.keys(output.exports)
                    .filter(
                        (subpath) => !entries.has(subpath) && !subpath.split("/").includes("test"),
                    )
                    .map((subpath) => `${name}${subpath.slice(1)}`);
            });

            return [...imported, ...exported];
        });
    }

    /** Start an instance's runner and resolve when it serves. */
    start(spec: InstanceSpec, exited: (exit: InstanceExit) => Promise<void>): Promise<void> {
        // spawn the runner or join a start in progress
        return this.#registry.start(spec, (identify) => this.#spawn(spec, exited, identify));
    }

    /** Report whether an instance's runner serves now. */
    isRunning(instanceId: Identifier<"instance">): boolean {
        return this.#registry.isRunning(instanceId);
    }

    /** Find the spec of the instance whose runner has a secret. */
    identify(secret: string): InstanceSpec | undefined {
        return this.#registry.identify(secret);
    }

    /** Stop an instance after any start in progress, draining its runner before killing it. */
    stop(instanceId: Identifier<"instance">): Promise<void> {
        // drain the child after any start in progress, killing it once the drain runs out
        return this.#registry.stop(instanceId, (child) => child.stop());
    }

    /** Stop every instance, as the machine stops. */
    async close(): Promise<void> {
        const running = this.#registry.list().map((child) => child.spec.instanceId);
        await Promise.all(running.map((instanceId) => this.stop(instanceId)));
    }

    /** Forward a request below an instance's service to its runner, as its caller or anonymous. */
    async fetch(
        instanceId: Identifier<"instance">,
        path: string,
        request: Request,
        authentication: Authentication | undefined,
        partition?: string,
    ): Promise<Response> {
        // forward the request at its path below the installation with the verified caller
        const child = this.#registry.require(instanceId);
        const { secret } = child;
        const forwarded = WorkloadRequest.forward(
            RUNNER_ORIGIN,
            path,
            request,
            secret,
            authentication,
        );

        // name the partition the call goes to, which the runner serves in its process
        if (partition !== undefined) {
            forwarded.headers.set(PARTITION_HEADER, partition);
        }

        return child.send(forwarded);
    }

    /** Hand an instance a request the machine makes at a path below its origin. */
    async deliver(
        instanceId: Identifier<"instance">,
        path: string,
        request: Request,
    ): Promise<Response> {
        // hand over the request at its path with the machine's secret
        const child = this.#registry.require(instanceId);

        return child.send(WorkloadRequest.forward(RUNNER_ORIGIN, path, request, child.secret));
    }

    /** Call an export of an image's root module once, sandboxed to the root, else in process. */
    async call(
        image: Digest,
        path: string,
        name: string,
        input: readonly JsonValue[],
    ): Promise<JsonValue> {
        // read the image's root build
        const build = await this.#packages.root(image);

        // call a builtin's export in this process's own instance, as importing it does
        const output = Object.values(build.manifest.outputs).find(
            (each) => each.runtime === BUN_RUNTIME,
        );
        const exported = Object.entries(output?.exports ?? {}).find(
            ([, each]) => each.path === path,
        );
        const shared =
            exported === undefined
                ? undefined
                : BunRuntime.#shared(build.manifest.package.name, exported[0]);
        if (shared !== undefined && this.#program.shares(shared)) {
            const module = await this.#environment.scheduler.run(() =>
                this.#program.module(shared),
            );

            return await BunRuntime.#apply(module, shared, name, input);
        }

        // find the module in the image's tree
        const tree = await this.#tree(image);
        const directory = await this.#build(image);
        const module = join(directory, path);
        if (this.#isolation === "sandbox") {
            await this.#requireNoAddon(await this.#packages.image(image));
        }

        return this.#isolation === "sandbox"
            ? await this.#callSandboxed(
                  { build: directory, roots: await tree.roots(this.#environment.files) },
                  module,
                  name,
                  input,
              )
            : await this.#callHosted(module, name, input);
    }

    /** Read the module specifier of a package's export, as the program shares it. */
    static #shared(name: string, specifier: string): string {
        return specifier === "." ? name : `${name}${specifier.slice(1)}`;
    }

    /** Call an export in a sandboxed process confined to the build, its closure and no network. */
    async #callSandboxed(
        layout: Pick<BunInstanceLayout, "build" | "roots">,
        module: string,
        name: string,
        input: readonly JsonValue[],
    ): Promise<JsonValue> {
        // start the call's program reading the build and its closure alone
        const { process, clock } = this.#environment;
        const sandbox = await Sandbox.start(
            {
                executable: process.executable,
                arguments: ["--no-env-file", "--eval", CALL_PROGRAM],
                directory: layout.build,
                environment: {},
                read: [layout.build, ...layout.roots],
                write: [],
                network: [],
                sockets: [],
                listening: "denied",
            },
            this.#environment,
        );

        // hand it the call, then read its result once it exits, killing one that runs too long
        const writer = sandbox.stdin.getWriter();
        await writer.write(new TextEncoder().encode(JSON.stringify({ module, name, input })));
        await writer.close();
        const deadline = new AbortController();
        void clock.sleep(CALL_TIMEOUT_MILLISECONDS, deadline.signal).then(
            () =>
                sandbox
                    .stop(0)
                    .catch((error: unknown) =>
                        this.#scope.captureException(error, { isEscaped: true }),
                    ),
            () => undefined,
        );
        const [output, errors, exit] = await Promise.all([
            RequestBody.text(new Response(sandbox.stdout), "a sandboxed call's output"),
            RequestBody.text(new Response(sandbox.stderr), "a sandboxed call's errors"),
            sandbox.exited,
        ]).finally(() => deadline.abort());
        if (exit.code !== 0) {
            throw MachineError.of(
                "RUNTIME_ACTION_UNREACHABLE",
                `could not call '${name}' of '${module}': exit ${exit.signal ?? exit.code} '${errors.trim()}'`,
            );
        }

        return JsonValue.of(JSON.parse(output));
    }

    /** Call an export in this process, as hosted runners run. */
    async #callHosted(
        module: string,
        name: string,
        input: readonly JsonValue[],
    ): Promise<JsonValue> {
        // import the module as an operation, then call the export
        const imported: unknown = await this.#environment.scheduler.run(() => import(module));

        return await BunRuntime.#apply(imported, module, name, input);
    }

    /** Call an imported module's export, refusing a module without the function. */
    static async #apply(
        imported: unknown,
        module: string,
        name: string,
        input: readonly JsonValue[],
    ): Promise<JsonValue> {
        const exported: unknown =
            typeof imported === "object" && imported !== null
                ? Reflect.get(imported, name)
                : undefined;
        if (typeof exported !== "function") {
            throw MachineError.of(
                "RUNTIME_ACTION_UNREACHABLE",
                `could not call '${name}': '${module}' exports no such function`,
            );
        }

        return JsonValue.of((await Reflect.apply(exported, undefined, [...input])) ?? null);
    }

    /** Extract an instance's build, start its runner, and wait until it serves. */
    async #spawn(
        spec: InstanceSpec,
        exited: (exit: InstanceExit) => Promise<void>,
        identify: (secret: string) => void,
    ): Promise<Runner> {
        // find the build and describe its start
        const directories = await this.#directories(spec);
        const start = await this.#start(spec);
        identify(start.secret);

        // run the runner in a sandboxed process reading its closure's roots, refusing an addon there, else in this process
        if (this.#isolation === "sandbox") {
            await this.#requireNoAddon(await this.#packages.image(spec.image));
        }

        return this.#isolation === "sandbox"
            ? this.#sandboxed(
                  spec,
                  exited,
                  {
                      ...directories,
                      roots: await (await this.#tree(spec.image)).roots(this.#environment.files),
                  },
                  start,
              )
            : this.#hosted(spec, exited, directories, start);
    }

    /** Start a runner in a sandboxed process confined to its build, resources and egress. */
    async #sandboxed(
        spec: InstanceSpec,
        exited: (exit: InstanceExit) => Promise<void>,
        directories: BunInstanceLayout,
        start: WorkloadStart,
    ): Promise<Runner> {
        // confine the runner to its build, its installation's directories, its resources and the egress
        const { files, process } = this.#environment;
        const runtime = await files.realPath(await files.runtimeDirectory());
        const options = new BunWorkloadSandbox({
            executable: process.executable,
            directories,
            runner: InstanceSpec.entry(spec).path,
            runtime,
            egress: this.#egress(spec.scope),
            environment: process.variables,
            which: (command) => process.which(command),
        }).options(spec);
        const sandbox = await this.#run(options, start, spec);

        // send on its socket, and report an unasked exit and a sandbox failure as an exit
        const child: Runner = {
            spec,
            exited,
            secret: start.secret,
            send: (request) => this.#environment.network.fetch(request, { unix: sandbox.socket }),
            stop: async () => {
                await sandbox.stop(STOP_TIMEOUT_MILLISECONDS);
            },
        };
        const removed = (exit: InstanceExit) => this.#exited(child, exit);
        void sandbox.exited
            .then(
                (exit) =>
                    removed(
                        exit.signal === null
                            ? { code: present(exit.code, "a runner's exit code or signal") }
                            : { signal: exit.signal },
                    ),
                (error: unknown) => removed({ failure: Failure.of(error).message }),
            )
            .catch((error: unknown) => this.#scope.captureException(error, { isEscaped: true }));

        return child;
    }

    /** Start a runner in this process from its build's entry, on the runtime's environment. */
    async #hosted(
        spec: InstanceSpec,
        exited: (exit: InstanceExit) => Promise<void>,
        directories: Pick<BunInstanceLayout, "build">,
        start: WorkloadStart,
    ): Promise<Runner> {
        // link the builtins to this process's own, then import the entry's hosted workload as an operation
        const entry = join(directories.build, InstanceSpec.entry(spec).path);
        const imports = await this.#imports(await this.#packages.image(spec.image));
        const module: unknown = await this.#environment.scheduler.run(async () => {
            await this.#program.link(imports);

            const imported: unknown = await import(entry);

            return imported;
        });
        const hosted = BunRuntime.#workload(module, entry);
        const runner = await hosted.start(this.#environment, start, spec.build, (error) =>
            this.#output(spec, `workload failed: ${String(error)}`),
        );

        // send to it directly, and report its end as an exit
        const child: Runner = {
            spec,
            exited,
            secret: start.secret,
            send: (request) => runner.fetch(request),
            stop: async () => {
                runner.shutdown();
                await runner.close();
            },
        };
        runner.signal.addEventListener(
            "abort",
            () =>
                void this.#exited(child, { code: 0 }).catch((error: unknown) =>
                    this.#scope.captureException(error, { isEscaped: true }),
                ),
            { once: true },
        );

        return child;
    }

    /** Read the workload an entry module exports, refusing a module without one. */
    static #workload(module: unknown, entry: string): HostedWorkload {
        // take the module's workload export, refusing a module without a startable one
        const exported =
            typeof module === "object" && module !== null && WORKLOAD_EXPORT in module
                ? module[WORKLOAD_EXPORT]
                : undefined;
        if (!BunRuntime.#isWorkload(exported)) {
            throw MachineError.of(
                "RUNNER_BROKEN",
                `workload runner '${entry}' exports no workload`,
            );
        }

        return exported;
    }

    /** Decide whether a value is a workload this process can start. */
    static #isWorkload(value: unknown): value is HostedWorkload {
        return (
            typeof value === "object" &&
            value !== null &&
            "start" in value &&
            typeof value.start === "function"
        );
    }

    /** Start a runner's sandbox with its input and wait until it serves on its socket. */
    async #run(
        options: SandboxOptions,
        start: WorkloadStart,
        spec: InstanceSpec,
    ): Promise<Sandbox> {
        // start the sandbox, write the start to its input and record what it reports
        const sandbox = await Sandbox.start(options, this.#environment);
        void this.#capture(spec, sandbox.stderr).catch((error: unknown) =>
            this.#scope.captureException(error, { isEscaped: true }),
        );
        const input = sandbox.stdin.getWriter();
        await input.write(new TextEncoder().encode(`${JSON.stringify(start)}\n`));
        input.releaseLock();
        await BunRuntime.#serving(sandbox, this.#environment);

        return sandbox;
    }

    /** Find an instance's build, and create its installation's data and cache directories. */
    async #directories(spec: InstanceSpec): Promise<Omit<BunInstanceLayout, "roots">> {
        // find the build and the installation's directories
        const build = await this.#build(spec.image);
        const { files } = this.#environment;
        await files.mkdir(this.#directory, { recursive: true });
        const root = await files.realPath(this.#directory);
        const data = join(root, spec.installationId, "data");
        const cache = join(root, spec.installationId, "cache");

        // create the installation's directories
        for (const directory of [data, cache]) {
            await files.mkdir(directory, { recursive: true });
        }

        return { build, data, cache };
    }

    /** Lay out an image's tree once, shared by its instances and calls. */
    #tree(digest: Digest): Promise<BuildTree> {
        // share each image's tree
        const known = this.#trees.get(digest);
        if (known !== undefined) {
            return known;
        }
        const tree = this.#layOut(digest);
        this.#trees.set(digest, tree);
        tree.catch(() => this.#trees.delete(digest));

        return tree;
    }

    /** Lay out an image's tree in the store. */
    async #layOut(digest: Digest): Promise<BuildTree> {
        // link the builtins for a sandbox only
        const image = await this.#packages.image(digest);
        const supplied = this.#isolation === "sandbox" ? this.#builtins : [];
        const builtins = await Promise.all(
            supplied.map(
                async (builtin) =>
                    [builtin.package.name, await this.#packages.image(builtin.image)] as const,
            ),
        );

        return await this.#packages.tree(image, BUN_RUNTIME, new Map(builtins));
    }

    /** Find the directory of an image's root build. */
    async #build(digest: Digest): Promise<string> {
        // refuse a release without a Bun output
        const image = await this.#packages.image(digest);
        const root = await this.#packages.contents(image.root);
        const { name, version } = PackageManifest.package(root.manifest);
        const isRunning = Object.values(root.manifest.outputs).some(
            (each) => each.runtime === BUN_RUNTIME,
        );
        if (!isRunning) {
            throw MachineError.of(
                "RELEASE_INVALID",
                `release '${name}@${version}' is invalid: it has no ${BUN_RUNTIME} output`,
            );
        }

        // find the root build's directory in the tree
        const tree = await this.#tree(digest);

        return present(tree.directories[name], `the directory of ${name}`);
    }

    /** Refuse an image whose builds run a native addon on Bun. */
    async #requireNoAddon(image: BuildImage): Promise<void> {
        for (const build of BuildImage.builds(image)) {
            const reader = await this.#packages.contents(build);
            const addon = (await reader.runFiles([BUN_RUNTIME])).find((file) =>
                PackageFile.isAddon(file.path),
            );
            if (addon !== undefined) {
                const { name, version } = PackageManifest.package(reader.manifest);
                throw MachineError.of(
                    "RELEASE_INVALID",
                    `release '${name}@${version}' is invalid: it runs the native addon '${addon.path}', which only host packages may`,
                );
            }
        }
    }

    /** Describe a runner's start with a fresh secret. */
    async #start(spec: InstanceSpec): Promise<WorkloadStart> {
        return InstanceSpec.start(spec, {
            egress: this.#egress(spec.scope),
            url: await this.#url(spec.scope, spec.installationId),
            telemetry: await this.#workloadTelemetry(spec.scope, spec.installationId),
            identity: await this.#identity(spec.scope),
            random: this.#environment.random,
        });
    }

    /** Wait until a started runner serves, killing a runner that exits or stays silent. */
    static async #serving(
        sandbox: Sandbox,
        environment: Pick<Environment, "clock" | "scheduler">,
    ): Promise<void> {
        try {
            await BunRuntime.#ready(sandbox.stdout, environment);
        } catch (error) {
            // kill the runner
            const stopped = await sandbox.stop(0).then(
                () => undefined,
                (failure: unknown) => failure,
            );
            throw stopped === undefined
                ? error
                : new AggregateError([error, stopped], "a runner did not start, then did not stop");
        }
    }

    /** Report an exited runner's exit unless asked for. */
    async #exited(child: Runner, exit: InstanceExit): Promise<void> {
        // wait for the start recording the child, then report the exit of a child still recorded
        await this.#registry.settle(child.spec.instanceId);
        if (this.#registry.forget(child)) {
            await child.exited(exit);
        }
    }

    /** Record each line a runner writes to standard error until the stream ends. */
    async #capture(spec: InstanceSpec, stream: AsyncIterable<Uint8Array>): Promise<void> {
        // record each complete line and buffer the partial rest
        const decoder = new TextDecoder();
        let rest = "";
        for await (const chunk of stream) {
            const text = rest + decoder.decode(chunk, { stream: true });
            const end = text.lastIndexOf("\n");
            const lines = end === -1 ? [] : text.slice(0, end).split("\n");
            rest = text.slice(end + 1);
            for (const line of lines) {
                this.#output(spec, line);
            }
        }

        // record a last line without a line break
        if (rest !== "") {
            this.#output(spec, rest);
        }
    }

    /** Read a runner's first output line once it serves, discarding what it writes after. */
    static async #ready(
        stdout: ReadableStream<Uint8Array>,
        environment: Pick<Environment, "clock" | "scheduler">,
    ): Promise<void> {
        // read the first line as an operation until it ends, the process exits or the timeout passes
        const timer = new AbortController();
        const deadline = environment.clock
            .sleep(START_TIMEOUT_MILLISECONDS, timer.signal)
            .then(() => undefined)
            .catch(() => undefined);
        try {
            const first = await Promise.race([
                environment.scheduler.run(() => BunRuntime.#firstLine(stdout), { isLong: true }),
                deadline,
            ]);
            if (first === undefined) {
                throw MachineError.of(
                    "RUNNER_UNAVAILABLE",
                    "the workload runner is unavailable: it did not start serving",
                );
            }

            WorkloadReady.parse(JSON.parse(first));
        } finally {
            timer.abort();
        }
    }

    /** Read the first line of a stream, absent once it ends without one, then discard the rest. */
    static async #firstLine(stdout: ReadableStream<Uint8Array>): Promise<string | undefined> {
        // read chunks until one ends the first line
        const reader = stdout.getReader();
        const decoder = new TextDecoder();
        let text = "";
        let line: string | undefined;
        for (let read = await reader.read(); !read.done; read = await reader.read()) {
            text += decoder.decode(read.value, { stream: true });
            const end = text.indexOf("\n");
            if (end !== -1) {
                line = text.slice(0, end);
                break;
            }
        }

        // discard what it writes after, so its writes never block on a full pipe
        reader.releaseLock();
        void Pipe.to(stdout, new WritableStream()).catch(() => undefined);

        return line;
    }
}
