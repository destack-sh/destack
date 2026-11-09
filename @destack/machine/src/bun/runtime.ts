import type { Program } from "@destack/package/bun";
import { RequestBody, SERVER_EXPORT } from "@destack/object";
import { Authentication } from "@destack/access";
import { dirname, join } from "node:path";
import { type Environment, Pipe } from "@destack/environment";
import { DependencyTree, type LocalBuildStore } from "@destack/build";
import { BuildStore } from "@destack/build/store";
import { type BuildReader, MANIFEST_PATH, PackageManifest } from "@destack/package/manifest";
import { Sandbox, type SandboxOptions } from "@destack/sandbox";
import { type Digest, type Identifier, JsonValue, present } from "@destack/schema";
import type { Deriver } from "@destack/identity";
import { WORKLOAD_EXPORT, WorkloadReady, type WorkloadStart } from "@destack/object/workload";
import type { BunWorkload } from "@destack/stack/bun";
import {
    type InstanceExit,
    InstanceSpec,
    InstanceRegistry,
    type Runtime,
    WorkloadRequest,
} from "@destack/stack/runtime";
import { Failure } from "@destack/error";
import { type BunInstanceLayout, BunWorkloadSandbox } from "./sandbox.ts";
import type {} from "@destack/package/import-meta";
import { MachineError } from "../error/index.ts";
import { PackageImplementation } from "@destack/object/server";

/** The server runtime this runtime runs instances on. */
const BUN_RUNTIME = "bun";

/** The directory below the runtime's with the builds its instances run, one per manifest digest. */
const BUILDS_DIRECTORY = "builds";

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
    /** Resolve the share of traces an installation samples. */
    readonly #sampling: (
        scope: Identifier<"space">,
        installation: Identifier<"installation">,
    ) => Promise<number>;
    /** Resolve the public URL an installation's instances serve at. */
    readonly #url: (
        scope: Identifier<"space">,
        installation: Identifier<"installation">,
    ) => Promise<string>;
    /** Record a line a runner writes to standard error. */
    readonly #output: (spec: InstanceSpec, line: string) => void;
    /** Report a failure of the runtime's own work around a runner. */
    readonly #report: (error: unknown) => void;
    /** Read the identity of an instance's space, which its installation's root derives from. */
    readonly #identity: (scope: Identifier<"space">) => Promise<Pick<Deriver, "derive">>;
    /** The runners starting and running. */
    readonly #registry = new InstanceRegistry<Runner>();
    /** Each build's directory and closure roots by digest, shared by its starts and calls. */
    readonly #layouts = new Map<Digest, Promise<Pick<BunInstanceLayout, "build" | "roots">>>();
    /** The host the runners run on: its files, processes and clock. */
    readonly #environment: Environment<"files" | "process" | "host">;
    /** How the runners are isolated. */
    readonly #isolation: BunIsolation;

    /** Run instances below a directory, calling addresses through the machine's egress. */
    constructor(options: {
        /** The directory of each instance's files. */
        readonly directory: string;
        /** The machine's store of builds, with every build's closure. */
        readonly packages: LocalBuildStore;
        /** The modules this process shares with the servers it loads: the builtins. */
        readonly program: Program;
        /** The machine's egress of a space's instances, below which they call addresses. */
        readonly egress: (scope: Identifier<"space">) => string;
        /** Resolve the share of traces an installation samples, as its space's settings set it. */
        readonly sampling: (
            scope: Identifier<"space">,
            installation: Identifier<"installation">,
        ) => Promise<number>;
        /** Resolve the public URL an installation's instances serve at. */
        readonly url: (
            scope: Identifier<"space">,
            installation: Identifier<"installation">,
        ) => Promise<string>;
        /** Record a line a runner writes to standard error. */
        readonly output: (spec: InstanceSpec, line: string) => void;
        /** Report a failure of the runtime's own work around a runner, such as a stop. */
        readonly report: (error: unknown) => void;
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
        this.#program = options.program;
        this.#egress = options.egress;
        this.#sampling = options.sampling;
        this.#url = options.url;
        this.#report = options.report;
        this.#output = options.output;
        this.#identity = options.identity;
        this.#environment = options.environment;
        this.#isolation = options.isolation;
    }

    /** Load the server a build exports into this process, absent for a build exporting none. */
    async server(manifest: Digest): Promise<PackageImplementation | undefined> {
        // take none of a build without a server export
        const build = await this.#packages.contents(manifest);
        const output = Object.values(build.manifest.outputs).find(
            (each) => each.runtime === BUN_RUNTIME,
        );
        if (output?.exports[SERVER_EXPORT] === undefined) {
            return undefined;
        }

        // import it, refusing a module exporting no package implementation
        const module = await this.import(manifest, SERVER_EXPORT);
        const exported =
            typeof module === "object" && module !== null && "default" in module
                ? module.default
                : undefined;
        if (!PackageImplementation.is(exported)) {
            throw MachineError.of(
                "SERVER_INVALID",
                `${build.manifest.package.name} exports no package implementation as its server`,
            );
        }

        return exported;
    }

    /** Import an export of a build's Bun output, its imports of builtins this process's own. */
    async import(manifest: Digest, specifier: string): Promise<unknown> {
        // find the export in the build's Bun output
        const build = await this.#packages.contents(manifest);
        const output = present(
            Object.values(build.manifest.outputs).find((each) => each.runtime === BUN_RUNTIME),
            `the ${BUN_RUNTIME} output of ${build.manifest.package.name}`,
        );
        const exported = present(
            output.exports[specifier],
            `the ${specifier} export of ${build.manifest.package.name}`,
        );

        // import a builtin's export as this process's own instance, never a second copy from its build
        const { name } = build.manifest.package;
        const shared = specifier === "." ? name : `${name}${specifier.slice(1)}`;
        if (this.#program.shares(shared)) {
            return await this.#program.module(shared);
        }

        // place the build, link the builtin modules its closure may load to this process's own, and import the export
        const { build: placed } = await this.#write(build);
        const imports = await this.#imports(build, manifest);

        // load the modules as one real operation, completed in order whether the host loaded them before or not
        return await this.#environment.scheduler.run(async () => {
            await this.#program.link(imports);
            const module: unknown = await import(join(placed, exported.path));

            return module;
        });
    }

    /** List the modules a build's closure may load: its imports and each build's Bun exports. */
    async #imports(build: BuildReader, manifest: Digest): Promise<string[]> {
        // read each build of the closure once, as placing it did
        const closure = await this.#packages.closure(
            manifest,
            PackageManifest.package(build.manifest).id,
            BUN_RUNTIME,
            (needed) => this.#packages.get(needed),
            "export",
        );
        const manifests = await Promise.all(
            [...closure].map(async (each) =>
                present(await this.#packages.get(each), `build ${each} of the closure`),
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
    async start(spec: InstanceSpec, exited: (exit: InstanceExit) => Promise<void>): Promise<void> {
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
    async stop(instanceId: Identifier<"instance">): Promise<void> {
        // drain the child after any start in progress, killing it once the drain runs out
        await this.#registry.stop(instanceId, (child) => child.stop());
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
    ): Promise<Response> {
        // forward the request at its path below the installation with the verified caller
        const child = this.#registry.require(instanceId);
        const { secret } = child;

        return child.send(
            WorkloadRequest.forward(RUNNER_ORIGIN, path, request, secret, authentication),
        );
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

    /** Call an export of a build's module once, sandboxed to the build, else in this process. */
    async call(
        build: BuildReader,
        path: string,
        name: string,
        input: readonly JsonValue[],
    ): Promise<JsonValue> {
        // write the build as its instances run it, then call the export in the module
        const written = await this.#write(build);
        const module = join(written.build, path);

        return this.#isolation === "sandbox"
            ? await this.#callSandboxed(written, module, name, input)
            : await this.#callHosted(module, name, input);
    }

    /** Call an export in a sandboxed process confined to the build, its closure and no network. */
    async #callSandboxed(
        written: Pick<BunInstanceLayout, "build" | "roots">,
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
                directory: written.build,
                environment: {},
                read: [written.build, ...written.roots],
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
            () => sandbox.stop(0).catch((error: unknown) => this.#report(error)),
            () => undefined,
        );
        const [output, errors, exit] = await Promise.all([
            RequestBody.text(new Response(sandbox.stdout), "a sandboxed call's output"),
            RequestBody.text(new Response(sandbox.stderr), "a sandboxed call's errors"),
            sandbox.exited,
        ]).finally(() => deadline.abort());
        if (exit.code !== 0) {
            throw MachineError.of(
                "RUNTIME_ACTION_FAILED",
                `call ${name} of ${module} failed with ${exit.signal ?? exit.code}: ${errors.trim()}`,
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
        // import the module as an operation and call the export, refusing a module without the function
        const imported: unknown = await this.#environment.scheduler.run(() => import(module));
        const exported: unknown =
            typeof imported === "object" && imported !== null
                ? Reflect.get(imported, name)
                : undefined;
        if (typeof exported !== "function") {
            throw MachineError.of("RUNTIME_ACTION_FAILED", `${module} exports no function ${name}`);
        }

        return JsonValue.of((await Reflect.apply(exported, undefined, [...input])) ?? null);
    }

    /** Extract an instance's build, start its runner, and wait until it serves. */
    async #spawn(
        spec: InstanceSpec,
        exited: (exit: InstanceExit) => Promise<void>,
        identify: (secret: string) => void,
    ): Promise<Runner> {
        // write the build's files and describe its start
        const directories = await this.#directories(spec);
        const start = await this.#start(spec);
        identify(start.secret);

        // run the runner in a sandboxed process, else in this process
        return this.#isolation === "sandbox"
            ? this.#sandboxed(spec, exited, directories, start)
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
            .catch((error: unknown) => this.#report(error));

        return child;
    }

    /** Start a runner in this process from its build's entry, on the runtime's environment. */
    async #hosted(
        spec: InstanceSpec,
        exited: (exit: InstanceExit) => Promise<void>,
        directories: BunInstanceLayout,
        start: WorkloadStart,
    ): Promise<Runner> {
        // import the entry's hosted workload as an operation, and start it with the build's own runner
        const entry = join(directories.build, InstanceSpec.entry(spec).path);
        const module: unknown = await this.#environment.scheduler.run(() => import(entry));
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
                    this.#report(error),
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
            throw MachineError.of("RUNNER_NOT_SERVING", `${entry} exports no workload`);
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
        void this.#capture(spec, sandbox.stderr).catch((error: unknown) => this.#report(error));
        const input = sandbox.stdin.getWriter();
        await input.write(new TextEncoder().encode(`${JSON.stringify(start)}\n`));
        input.releaseLock();
        await BunRuntime.#serving(sandbox, this.#environment);

        return sandbox;
    }

    /** Create an instance's build directory and its installation's data and cache directories. */
    async #directories(spec: InstanceSpec): Promise<BunInstanceLayout> {
        // write the build once, beside the installation's data and cache directories
        const written = await this.#write(spec.build);
        const root = dirname(dirname(written.build));
        const data = join(root, spec.installationId, "data");
        const cache = join(root, spec.installationId, "cache");

        // create the installation's directories
        const { files } = this.#environment;
        for (const directory of [data, cache]) {
            await files.mkdir(directory, { recursive: true });
        }

        return { ...written, data, cache };
    }

    /** Write a build's Bun output and closure once, shared by its instances and calls. */
    async #write(build: BuildReader): Promise<Pick<BunInstanceLayout, "build" | "roots">> {
        // share the layout of each build, forgetting one that failed
        const digest = await BuildStore.digest(build.manifest, this.#environment.crypto);
        const known = this.#layouts.get(digest);
        if (known !== undefined) {
            return await known;
        }
        const placing = this.#place(digest);
        this.#layouts.set(digest, placing);
        placing.catch(() => this.#layouts.delete(digest));

        return await placing;
    }

    /** Place a build's Bun output in the runtime's directory, its closure in the store's trees. */
    async #place(digest: Digest): Promise<Pick<BunInstanceLayout, "build" | "roots">> {
        // run a layered build in place, its run tree linked beside it
        const { files } = this.#environment;
        const layered = this.#packages.layered(digest);
        if (layered !== undefined && (await files.exists(join(layered, "node_modules")))) {
            const build = await files.realPath(layered);

            return { build, roots: await DependencyTree.linkedRoots(build, files) };
        }

        // place the closure's Bun outputs in the store's trees, shared with every build reaching them
        const build = await this.#packages.contents(digest);
        const tree = await this.#packages.runTree(build, digest, BUN_RUNTIME);
        const roots = await tree.roots(files);

        // leave a build written before, whose manifest a write publishes last
        await files.mkdir(this.#directory, { recursive: true });
        const written = join(await files.realPath(this.#directory), BUILDS_DIRECTORY, digest);
        if (await files.exists(join(written, MANIFEST_PATH))) {
            return { build: written, roots };
        }

        // write the build's Bun output with its maps, its manifest and its closure's links in a staging directory beside its place
        await files.mkdir(dirname(written), { recursive: true });
        const staging = await files.makeTemporaryDirectory(`${digest}.`, {
            directory: dirname(written),
        });
        const staged = join(staging, "build");
        const [name] = present(
            Object.entries(build.manifest.outputs).find(([, each]) => each.runtime === BUN_RUNTIME),
            `the ${BUN_RUNTIME} output of ${build.manifest.package.name}`,
        );
        await this.#packages.place(
            (await build.outputFiles(name)).map((file) => ({
                file,
                destination: join(staged, file.path),
            })),
        );
        await files.writeFile(
            join(staged, MANIFEST_PATH),
            PackageManifest.serialize(build.manifest),
        );
        await tree.link(join(staged, "node_modules"), files);

        // move it in, leaving a build another process moved in first
        try {
            await files.rename(staged, written);
        } catch (error) {
            if (!(await files.exists(join(written, MANIFEST_PATH)))) {
                throw error;
            }
        } finally {
            await files.remove(staging, { recursive: true });
        }

        return { build: written, roots };
    }

    /** Describe a runner's start with a fresh secret. */
    async #start(spec: InstanceSpec): Promise<WorkloadStart> {
        return InstanceSpec.start(spec, {
            egress: this.#egress(spec.scope),
            url: await this.#url(spec.scope, spec.installationId),
            sampling: await this.#sampling(spec.scope, spec.installationId),
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
                : new AggregateError([error, stopped], "a runner failed to start and to stop");
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
                    "RUNNER_NOT_SERVING",
                    "the workload runner did not start serving",
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
