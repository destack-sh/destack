import type { DeclarationDescription, graph } from "@destack/package";
import { dirname, join, relative, resolve } from "node:path";
import { type Environment, Telemetry } from "@destack/environment";
import { fileURLToPath } from "node:url";
import { Sandbox } from "@destack/sandbox";
import { Toolchain } from "@destack/check/toolchain";
import { Distribution } from "@destack/update";
import { Release } from "@destack/update/release";
import { FamilyError } from "@destack/error";
import type { EvaluatedDeclaration } from "../declaration/index.ts";
import { BuildError } from "../error/index.ts";
import { locateWorkspace } from "@destack/check/workspace";
import { Digest, present } from "@destack/schema";
import type { LocalBuildStore } from "../store/local.ts";
import type { PackageBuildOptions } from "./build.ts";
import { BuildTree } from "../dependency/tree.ts";
import {
    type BuildRequest,
    type BuildResponse,
    type BuildResult,
    type CacheRequest,
    CompilerMessage,
    readMessages,
    type RequestedBuild,
    writeMessage,
    type IndexedDeclarations,
} from "./message.ts";
import type { BuildReader } from "@destack/package/manifest";
import type { SourceOptions } from "../source/index.ts";
import type {} from "@destack/package/import-meta";
import { TelemetryScope } from "@destack/telemetry";
import { OtlpExport } from "@destack/telemetry/otlp";

/** The most diagnostics a compiler retains, 1 MiB. */
const MAX_DIAGNOSTICS = 1_048_576;

/** The diagnostics a failure over the limit quotes, 4 KiB: about 25 lines, the first errors. */
const DIAGNOSTICS_HEAD = 4096;

/** How a host starts the compiler: its executable, arguments, and tool directories. */
export interface CompilerCommand {
    /** The executable starting the compiler. */
    readonly executable: string;
    /** The arguments starting the compiler. */
    readonly arguments: readonly string[];
    /** The directories with the compiler and its tools. */
    readonly read: readonly string[];
    /** The variables the compiler starts with, such as the path of rolldown's native binding. */
    readonly variables: Readonly<Record<string, string>>;
}

/** The commands that start the compiler: a release's beside its executables, or from source. */
export const CompilerCommand = {
    /** Start a release's executable as its build process, running a toolchain. */
    beside(
        executable: string,
        toolchain: Toolchain,
        environment: Environment<"host">,
    ): CompilerCommand {
        // refuse a release's compiler without a toolchain directory, which it reads its tools from
        const { directory } = toolchain;
        if (directory === undefined) {
            throw BuildError.of("INVARIANT_BROKEN", "a release's compiler needs a toolchain");
        }

        // load rolldown's native binding from where the release lays it out in its resources
        const resources = Distribution.resources(executable);
        const platform = Toolchain.platform(environment.host);
        const binding = join(resources, "node_modules", "@rolldown", `binding-${platform}`);

        return {
            executable,
            arguments: Release.arguments("build"),
            // read the executable's, the resources' and the toolchain's directories
            read: [dirname(executable), resources, directory],
            variables: {
                NAPI_RS_NATIVE_LIBRARY_PATH: join(binding, `rolldown-binding.${platform}.node`),
                DESTACK_TOOLCHAIN: directory,
            },
        };
    },

    /** Find the compiler this process starts, running a toolchain. */
    async current(
        toolchain: Toolchain,
        environment: Environment<"files" | "process" | "host">,
    ): Promise<CompilerCommand> {
        // start the release's compiler from a standalone executable
        const { executable } = environment.process;
        if (environment.process.runtime.isStandalone) {
            return CompilerCommand.beside(executable, toolchain, environment);
        }

        // run the entry with Bun, reading the workspace that installed the tools and a named toolchain
        const entry = fileURLToPath(new URL("../main.ts", import.meta.url));
        const root = fileURLToPath(new URL("../..", import.meta.url));
        const workspace = await locateWorkspace(root, environment.files);
        const { directory } = toolchain;

        // stamp the compiler's sources at the release this process builds, as its own are
        const release = environment.process.variables["DESTACK_RELEASE_VERSION"];

        return {
            executable,
            arguments: ["run", "--no-env-file", entry],
            read: directory === undefined ? [workspace] : [workspace, directory],
            variables: {
                ...(directory === undefined ? {} : { DESTACK_TOOLCHAIN: directory }),
                ...(release === undefined ? {} : { DESTACK_RELEASE_VERSION: release }),
            },
        };
    },
};

/** How a compiler starts: its command, its toolchain, and the store it builds into. */
export interface CompilerOptions {
    /** The command starting it, absent for this process's compiler. */
    readonly command?: CompilerCommand;
    /** Read the toolchain the compiler runs, once it first needs it. */
    readonly toolchain: () => Promise<Toolchain>;
    /** The store whose cache its builds reuse and fill. */
    readonly store: LocalBuildStore;
}

/** A stored build with the declarations it compiled on. */
export interface CompiledBuild {
    /** The stored build. */
    readonly build: BuildReader;
    /** The declarations it compiled on. */
    readonly declarations: readonly EvaluatedDeclaration[];
}

/** A sandboxed compiler retained across builds of a workspace's packages. */
export class Compiler implements AsyncDisposable {
    /** The workspace whose packages it builds. */
    readonly workspace: string;
    /** The store whose cache its builds reuse and fill. */
    readonly store: LocalBuildStore;
    /** The compiler's files, its temporary files and the builds it writes, removed on shutdown. */
    readonly #temporary: string;
    /** The directories the compiler reads: the workspace, its tools and the store's trees. */
    readonly #read: readonly string[];
    /** How the compiler starts. */
    readonly #compiler: CompilerCommand;
    /** The sandboxed compiler, started again after a failed one. */
    #sandbox: Sandbox;
    /** The writer of the current compiler's input. */
    #input: WritableStreamDefaultWriter<Uint8Array>;
    /** Compiler completion, including native compiler shutdown. */
    #closed: Promise<void>;
    /** The tree the last request installed, keyed by its dependencies and installation. */
    #tree: { readonly key: Digest; readonly tree: BuildTree } | undefined;
    /** The request awaiting a response. */
    #pending:
        | {
              resolve: (result: BuildResponse) => void;
              reject: (error: unknown) => void;
          }
        | undefined;
    /** The failure that stopped the current compiler. */
    #failure: Error | undefined;
    /** Whether the compiler's termination has already been requested. */
    #stopping = false;
    /** Whether the caller has closed this compiler. */
    #disposed = false;
    /** Shared shutdown, including removal of temporary files. */
    #closing?: Promise<void>;
    /** Exit status reported by the compiler. */
    #exitCode: number | null = null;
    /** Compiler diagnostics retained until exit. */
    #stderr = "";
    /** The builds the compiler wrote, which numbers the directory of the next. */
    #builds = 0;
    /** The last request's settled work, since the compiler takes one request at a time. */
    #queue: Promise<void> = Promise.resolve();
    /** The spans and durations of the compiler's verbs. */
    readonly #scope: TelemetryScope;

    /** Refuse a response of another kind than its request's. */
    static mismatched(request: string, response: string): Error {
        return BuildError.of(
            "COMPILER_BROKEN",
            `compiler returned '${response}' for a '${request}' request`,
        );
    }

    /** Retain the workspace, store, files, read paths, command and compiler. */
    private constructor(
        workspace: string,
        store: LocalBuildStore,
        temporary: string,
        read: readonly string[],
        compiler: CompilerCommand,
        sandbox: Sandbox,
    ) {
        // store the workspace, the store and the compiler's files
        this.workspace = workspace;
        this.store = store;
        this.#temporary = temporary;
        this.#read = read;
        this.#compiler = compiler;
        this.#tree = undefined;
        this.#scope = TelemetryScope.of(import.meta.destack.package, store.environment.telemetry);

        // watch the started compiler
        this.#sandbox = sandbox;
        this.#input = sandbox.stdin.getWriter();
        this.#closed = this.#watch(sandbox, this.#input);
    }

    /** Start a reusable compiler for a directory's workspace, building into a store. */
    static start(directory: string, options: CompilerOptions): Promise<Compiler> {
        const { telemetry } = options.store.environment;
        return TelemetryScope.of(import.meta.destack.package, telemetry).span(
            "destack.compiler.start",
            {},
            () => Compiler.#start(directory, options),
        );
    }

    /** Spawn a sandboxed compiler reading the workspace, its tools and the store's trees. */
    static async #start(directory: string, options: CompilerOptions): Promise<Compiler> {
        // read the toolchain, the workspace, the compiler's tools and the store's trees
        const { store } = options;
        const { environment } = store;
        const { files } = environment;
        const started =
            options.command ??
            (await CompilerCommand.current(await options.toolchain(), environment));
        const workspace = await files.realPath(await locateWorkspace(resolve(directory), files));
        const paths = [
            workspace,
            ...started.read,
            store.trees,
            ...(await store.layerDirectories()),
        ];
        const read = [...new Set(await Promise.all(paths.map((path) => files.realPath(path))))];

        // start the compiler with a temporary directory
        const temporary = await files.realPath(await store.makeTemporaryDirectory("compile-"));
        try {
            const sandbox = await spawn(workspace, temporary, read, started, environment);

            return new Compiler(workspace, store, temporary, read, started, sandbox);
        } catch (error) {
            await files.remove(temporary, { recursive: true });
            throw error;
        }
    }

    /** Materialize a request's dependency builds in the store's trees, which the compiler reads. */
    async #install(
        options: Pick<PackageBuildOptions, "dependencies" | "links">,
    ): Promise<BuildTree> {
        // key the tree by the dependency builds, their links and the workspace's installation
        const { dependencies, links } = options;
        const { files, crypto } = this.store.environment;
        const manifests = Object.entries(dependencies).map(([name, each]) => [name, each.manifest]);
        const stamp = await BuildTree.stamp(this.workspace, files, crypto);
        const key = await Digest.json({ manifests, links, stamp }, crypto);

        // reuse the last tree while its key holds and its roots stand, using its builds as installing does
        const last = this.#tree;
        if (last?.key === key && (await last.tree.isPlaced(files))) {
            const used = [
                ...Object.values(dependencies).map((each) => each.manifest),
                ...Object.values(links).flatMap((each) => Object.values(each)),
            ];
            await this.store.use([...new Set(used)]);

            return last.tree;
        }
        const tree = await BuildTree.install(dependencies, links, this.workspace, this.store);
        this.#tree = { key, tree };

        return tree;
    }

    /** Start the compiler again after a failed one. */
    #restart(): Promise<void> {
        return this.#scope.span("destack.compiler.start", {}, () => this.#respawn());
    }

    /** Spawn the next compiler and watch it, forgetting the stopped one's failure. */
    async #respawn(): Promise<void> {
        // start the next compiler, recording the failure of the stopped one should it not start
        const sandbox = await spawn(
            this.workspace,
            this.#temporary,
            this.#read,
            this.#compiler,
            this.store.environment,
        );

        // reset the failure and diagnostics, then watch the next compiler
        this.#failure = undefined;
        this.#stopping = false;
        this.#exitCode = null;
        this.#stderr = "";
        this.#sandbox = sandbox;
        this.#input = sandbox.stdin.getWriter();
        this.#closed = this.#watch(sandbox, this.#input);
    }

    /** Watch a started compiler's exit, diagnostics and responses, returning its completion. */
    #watch(sandbox: Sandbox, input: WritableStreamDefaultWriter<Uint8Array>): Promise<void> {
        // fail the pending build of a compiler that exits unexpectedly or whose sandbox fails
        const closed = sandbox.exited.then(
            ({ code }) => {
                this.#exitCode = code;
                if (code !== 0 || !this.#disposed) {
                    this.#failure ??= BuildError.of(
                        "COMPILER_BROKEN",
                        `compiler exited ('${code}'): '${this.#stderr.trim()}'`,
                    );
                }
                this.#pending?.reject(this.#failure);
            },
            (cause: unknown) => {
                this.#failure ??= BuildError.of(
                    "COMPILER_BROKEN",
                    "the compiler's sandbox failed",
                    {
                        cause,
                    },
                );
                this.#pending?.reject(this.#failure);
            },
        );

        // leave a later compiler alone once this one was replaced
        const fail = (error: Error) => {
            if (this.#sandbox === sandbox) {
                this.#fail(error);
            }
        };
        input.closed.catch((cause: unknown) =>
            fail(BuildError.of("COMPILER_BROKEN", "the compiler's input failed", { cause })),
        );

        // bound diagnostics retained from compiler tools
        void this.#diagnose(sandbox, fail).catch(fail);

        // deliver each response to the pending build
        void this.#deliver(sandbox, input).catch(fail);

        return closed;
    }

    /** Retain the current compiler's diagnostics, failing it once and stopping past the bound. */
    async #diagnose(sandbox: Sandbox, fail: (error: Error) => void): Promise<void> {
        const decoder = new TextDecoder();
        let partial = "";
        for await (const chunk of sandbox.stderr) {
            // stop once a restarted compiler replaced this one
            if (this.#sandbox !== sandbox) {
                return;
            }

            // log each whole line as it arrives, as a panic it prints may never end the process
            const text = decoder.decode(chunk, { stream: true });
            const lines = `${partial}${text}`.split("\n");
            partial = lines.pop() ?? "";
            for (const line of lines.filter((each) => each.trim() !== "")) {
                this.#scope.log.debug("destack.compiler.stderr", {
                    "destack.compiler.stderr": line,
                });
            }

            // retain the text, failing the compiler once it passes the bound
            this.#stderr += text;
            if (this.#stderr.length > MAX_DIAGNOSTICS) {
                const head = this.#stderr.slice(0, DIAGNOSTICS_HEAD);
                fail(
                    BuildError.of(
                        "COMPILER_BROKEN",
                        `build diagnostics exceeded 1 MiB, starting: '${head}'`,
                    ),
                );

                return;
            }
        }
    }

    /** Deliver each compiler response to the build awaiting it, and answer its cache requests. */
    async #deliver(
        sandbox: Sandbox,
        input: WritableStreamDefaultWriter<Uint8Array>,
    ): Promise<void> {
        for await (const message of readMessages(sandbox.stdout, (value) =>
            CompilerMessage.parse(value),
        )) {
            // refuse a message no request awaits
            const pending = this.#pending;
            if (pending === undefined) {
                throw BuildError.of("COMPILER_BROKEN", "compiler sent a message without a request");
            }
            // deliver the response to its request
            else if (message.kind === "result") {
                pending.resolve(message);
            }
            // answer a transform's cache request from the pending build's store
            else {
                void this.#answer(input, message).catch((error: unknown) =>
                    this.#fail(
                        BuildError.of("COMPILER_BROKEN", "cannot respond to a cache request", {
                            cause: error,
                        }),
                    ),
                );
            }
        }
    }

    /** Read or write a transform's file in the store. */
    async #answer(
        input: WritableStreamDefaultWriter<Uint8Array>,
        request: CacheRequest,
    ): Promise<void> {
        // read the file a key caches, a miss without an entry, else store the file under its key
        const { cache } = this.store;
        let bytes: Uint8Array<ArrayBuffer> | null = null;
        if (request.kind === "read") {
            bytes = (await cache.read(request.key)) ?? null;
        } else {
            await cache.write(request.key, request.bytes);
        }

        // answer the request
        await writeMessage(input, { kind: "cached", id: request.id, bytes });
    }

    /** Build current source on declarations given, else evaluated here, storing the build. */
    build(
        options: PackageBuildOptions,
        declarations?: readonly EvaluatedDeclaration[],
    ): Promise<CompiledBuild> {
        return this.#scope.span(
            "destack.compiler.build",
            { "destack.package.directory": relative(this.workspace, options.directory) },
            () => this.#storeBuild(options, declarations),
        );
    }

    /** Compile after the requests before it, then move the written build into the store. */
    async #storeBuild(
        options: PackageBuildOptions,
        declarations: readonly EvaluatedDeclaration[] | undefined,
    ): Promise<CompiledBuild> {
        // build after the requests before it
        const { files } = this.store.environment;
        const built = await this.#serially(() => this.#build(options, declarations));
        const { destination } = built;
        try {
            const build = await this.store.contents(
                await this.store.move(destination, options.signal),
            );

            return { build, declarations: built.declarations };
        } catch (error) {
            await files.remove(destination, { recursive: true, force: true });
            throw error;
        }
    }

    /** Read a build's request at its real directory, its dependencies installed. */
    async #requested(options: PackageBuildOptions): Promise<RequestedBuild> {
        // resolve the directory and install the dependencies
        const { files } = this.store.environment;
        const { signal: _signal, timeout: _timeout, links: _links, ...given } = options;
        const directory = await files.realPath(resolve(given.directory));
        const tree = await this.#install(options);

        return { ...given, directory, tree: tree.directories };
    }

    /** Compile a package's current source with retained compiler state, evaluating unless given. */
    async #build(
        options: PackageBuildOptions,
        declarations: readonly EvaluatedDeclaration[] | undefined,
    ): Promise<{
        readonly destination: string;
        readonly declarations: readonly EvaluatedDeclaration[];
    }> {
        // build into temporary files moved into place once complete
        const { files } = this.store.environment;
        const { signal, timeout } = options;
        const request = await this.#requested(options);
        const written = join(this.#temporary, "builds", String(this.#builds++));
        const destination = await this.store.makeTemporaryDirectory("build-");
        try {
            const compiled = await this.#compile(request, declarations, written, signal, timeout);
            await files.rename(written, destination);

            return { destination, declarations: compiled };
        } catch (error) {
            if (this.#stopping) {
                await this.#closed;
            }
            await files.remove(written, { recursive: true, force: true });
            await files.remove(destination, { recursive: true, force: true });
            throw error;
        }
    }

    /** Compile current source into a directory, answering the declarations it compiled on. */
    async #compile(
        options: RequestedBuild,
        declarations: readonly EvaluatedDeclaration[] | undefined,
        destination: string,
        signal: AbortSignal | undefined,
        timeout: number | undefined,
    ): Promise<readonly EvaluatedDeclaration[]> {
        // compile into the directory
        await this.store.environment.files.mkdir(destination, { recursive: true });
        const request: BuildRequest =
            declarations === undefined
                ? { kind: "build", destination, options }
                : { kind: "compile", destination, options, declarations: [...declarations] };
        const response = await this.#request(request, signal, timeout);
        if (response.kind !== "build") {
            throw Compiler.mismatched(request.kind, response.kind);
        }

        return response.declarations;
    }

    /** Evaluate a package's declarations from source in the isolated compiler. */
    declarations(
        options: SourceOptions & Pick<PackageBuildOptions, "dependencies" | "links">,
        signal?: AbortSignal,
    ): Promise<readonly DeclarationDescription[]> {
        return this.#scope.span("destack.compiler.declarations", {}, () =>
            this.#serially(async () => {
                // send the request at the real directory, the dependencies materialized
                const request = {
                    kind: "declarations",
                    options: await this.#sourced(options),
                } as const;
                const response = await this.#request(request, signal);
                if (response.kind !== "declarations") {
                    throw Compiler.mismatched(request.kind, response.kind);
                }

                return response.declarations;
            }),
        );
    }

    /** Index source with its build's declarations in the isolated compiler. */
    index(
        options: SourceOptions & Pick<PackageBuildOptions, "dependencies" | "links">,
        declared: IndexedDeclarations,
        signal?: AbortSignal,
    ): Promise<graph.Module[]> {
        return this.#scope.span("destack.compiler.index", {}, () =>
            this.#serially(async () => {
                // send the index request at the real directory, the dependencies materialized
                const request = {
                    kind: "index",
                    options: await this.#sourced(options),
                    declared,
                } as const;
                const response = await this.#request(request, signal);
                if (response.kind !== "index") {
                    throw Compiler.mismatched(request.kind, response.kind);
                }

                return response.modules;
            }),
        );
    }

    /** Read source options at the real directory, the Destack dependencies materialized. */
    async #sourced(
        options: SourceOptions & Pick<PackageBuildOptions, "dependencies" | "links">,
    ): Promise<SourceOptions & Pick<RequestedBuild, "tree">> {
        // resolve the directory and materialize the dependencies' builds
        const { dependencies: _dependencies, links: _links, ...inspected } = options;
        const { files } = this.store.environment;
        const directory = await files.realPath(resolve(inspected.directory));
        const tree = await this.#install(options);

        return { ...inspected, directory, tree: tree.directories };
    }

    /** Run work after the compiler's earlier requests settled. */
    #serially<Result>(work: () => Promise<Result>): Promise<Result> {
        const running = this.#queue.then(work);
        this.#queue = running.then(
            () => {},
            () => {},
        );

        return running;
    }

    /** Send one request and retain its cancellation until the compiler responds. */
    async #request(
        request: BuildRequest,
        signal?: AbortSignal,
        timeout?: number,
    ): Promise<BuildResult> {
        // refuse a closed compiler, an invalid timeout and a cancelled request
        if (this.#disposed) {
            throw BuildError.of("INVARIANT_BROKEN", "compiler is closed");
        }
        if (
            timeout !== undefined &&
            (!Number.isSafeInteger(timeout) || timeout <= 0 || timeout > 2 ** 31 - 1)
        ) {
            throw BuildError.of(
                "TIMEOUT_INVALID",
                "compiler timeout is invalid: it must be a positive 32-bit integer of milliseconds",
            );
        }
        if (signal?.aborted === true) {
            throw BuildError.of("BUILD_CANCELLED", "build was cancelled", { cause: signal.reason });
        }

        // start a compiler again once a cancelled, late or crashed one has stopped
        if (this.#failure) {
            await this.#closed;
            await this.#restart();
        }
        const abort = () =>
            this.#fail(
                BuildError.of("BUILD_CANCELLED", "build was cancelled", { cause: signal?.reason }),
            );
        // fail a build past the caller's deadline, and leave one without a deadline to its signal
        const deadline = new AbortController();
        if (timeout !== undefined) {
            const late = BuildError.of(
                "COMPILER_UNREACHABLE",
                `could not finish the build within ${timeout} ms`,
            );
            void this.store.environment.clock.sleep(timeout, deadline.signal).then(
                () => this.#fail(late),
                (error: unknown) => {
                    if (!deadline.signal.aborted) {
                        throw error;
                    }
                },
            );
        }
        signal?.addEventListener("abort", abort, { once: true });

        // send the request and wait for its response, a peer the compiler answers
        try {
            const { scheduler, telemetry } = this.store.environment;
            const response = await scheduler.run(() => this.#send(request), { isLong: true });

            // take the spans the compiler ended into this telemetry
            telemetry.ingest(new OtlpExport("traces", response.traces).spans());

            return resultOf(response);
        } finally {
            deadline.abort();
            signal?.removeEventListener("abort", abort);
            this.#pending = undefined;
        }
    }

    /** Send a request to the compiler and await its response. */
    #send(request: BuildRequest): Promise<BuildResponse> {
        // clear the last request's diagnostics, and carry the active span's trace context
        this.#stderr = "";
        const trace = new Headers();
        Telemetry.inject(trace, this.store.environment.telemetry);
        const traced = { kind: "request", trace: Object.fromEntries(trace), request } as const;

        return new Promise<BuildResponse>((deliver, reject) => {
            this.#pending = { resolve: deliver, reject };
            void writeMessage(this.#input, traced).catch((error: unknown) =>
                this.#fail(
                    BuildError.of("COMPILER_BROKEN", "cannot send the compiler request", {
                        cause: error,
                    }),
                ),
            );
        });
    }

    /** Stop the compiler once its queued requests settled, and remove its temporary files. */
    [Symbol.asyncDispose](): Promise<void> {
        return (this.#closing ??= this.#serially(() => this.#close()));
    }

    /** Drain the compiler before releasing its temporary files. */
    async #close(): Promise<void> {
        // mark disposal, fail a pending build and end the compiler input
        this.#disposed = true;
        if (this.#pending) {
            this.#fail(BuildError.of("INVARIANT_BROKEN", "compiler closed during a build"));
        }
        void this.#input.close().catch((cause: unknown) => {
            this.#failure ??= BuildError.of("COMPILER_BROKEN", "cannot end the compiler's input", {
                cause,
            });
        });

        // wait for the compiler's exit as real work its world sees
        try {
            await this.store.environment.scheduler.waitUntil(this.#closed);
            if (this.#exitCode !== 0 && !this.#stopping) {
                throw present(this.#failure, "the failure of the exited compiler");
            }
        } finally {
            await this.store.environment.files.remove(this.#temporary, { recursive: true });
        }
    }

    /** Stop the compiler and its native children after a terminal failure. */
    #fail(error: Error): void {
        // reject the pending build once and stop the compiler once
        this.#failure ??= error;
        this.#pending?.reject(this.#failure);
        if (this.#stopping) {
            return;
        }
        this.#stopping = true;

        // stop the compiler's process group at once
        void this.#sandbox.stop(0).catch((cause: unknown) => {
            this.#failure ??= BuildError.of("COMPILER_BROKEN", "cannot stop the compiler", {
                cause,
            });
        });
    }
}

/** Start a sandboxed compiler in a workspace without network or host environment. */
function spawn(
    workspace: string,
    temporary: string,
    read: readonly string[],
    compiler: CompilerCommand,
    environment: Environment<"files" | "process" | "host">,
): Promise<Sandbox> {
    return Sandbox.start(
        {
            executable: compiler.executable,
            arguments: [...compiler.arguments],
            directory: workspace,
            environment: {
                NODE_ENV: "production",
                HOME: temporary,
                TMPDIR: temporary,
                NO_COLOR: "1",
                ...compiler.variables,
            },
            read: [...read],
            write: [temporary],
            network: [],
            sockets: [],
            listening: "denied",
        },
        environment,
    );
}

/** Read a compiler response's result, rethrowing the failure it reports. */
function resultOf(response: BuildResponse): BuildResult {
    if (response.error !== undefined) {
        const { failure: described, audience } = response.error;
        const { code, reason, domain, message, data } = described;
        const failure = new FamilyError({ domain, reason, code, data, audience }, message, {
            cause: response.error.cause,
        });
        if (response.error.stack !== undefined) {
            failure.stack = response.error.stack;
        }
        throw failure;
    }

    return response.result;
}
