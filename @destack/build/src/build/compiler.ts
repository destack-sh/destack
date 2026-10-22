import type { graph } from "@destack/package";
import { type PackageInspection } from "@destack/package/code";
import { mkdir, mkdtemp, realpath, rename, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { Sandbox } from "@destack/sandbox";
import { readBuild } from "@destack/package/bun";
import { Toolchain } from "@destack/check/toolchain";
import { BunProcess } from "@destack/check/bun";
import { BuildError } from "../error/index.ts";
import { locateWorkspace } from "@destack/check/workspace";
import { present } from "@destack/schema";
import { PackageFile } from "@destack/package/file";
import type { OutputKeys, CacheEntry, CachedOutput } from "../cache/index.ts";
import type { PackageStore } from "../store/index.ts";
import type { LocalPackageStore } from "../store/local.ts";
import type { PackageBuildOptions } from "./build.ts";
import { DependencyTree } from "../dependency/tree.ts";
import {
    type NpmConversionRequest,
    type NpmConverter,
    type NpmDependency,
    NpmUnit,
} from "../dependency/npm.ts";
import { PackageBuild } from "./build.ts";
import {
    type BuildRequest,
    type BuildResponse,
    type BuildResult,
    type CacheRequest,
    CompilerMessage,
    readMessages,
    type RequestedBuild,
    writeMessage,
} from "./message.ts";
import type { InspectOptions } from "../inspection/inspection.ts";
import type { PackageManifest } from "@destack/package/manifest";

/** The compiler executable beside a standalone Destack executable. */
const EXECUTABLE = "destack-build";

/** The build store's directory of the files transforms derive. */
const TRANSFORM_DIRECTORY = "transforms";

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
}

/** The commands that start the compiler: a release's beside its executables, or from source. */
export const CompilerCommand = {
    /** Find the released `destack-build` beside an executable, reading its toolchain directory. */
    beside(executable: string): CompilerCommand {
        const compiler = join(dirname(executable), EXECUTABLE);

        return {
            executable: compiler,
            arguments: [],
            read: [compiler, Toolchain.of(executable)],
        };
    },

    /** Find the compiler this process starts. */
    async current(): Promise<CompilerCommand> {
        // start the release's compiler from a standalone executable
        if (BunProcess.isStandalone) {
            return CompilerCommand.beside(process.execPath);
        }

        // run the entry with Bun, reading the workspace that installed the tools
        const entry = fileURLToPath(new URL("../main.ts", import.meta.url));
        const workspace = await locateWorkspace(fileURLToPath(new URL("../..", import.meta.url)));

        return {
            executable: process.execPath,
            arguments: ["run", "--no-env-file", entry],
            read: [workspace],
        };
    },
};

/** How a compiler starts: its command, and the store it builds into. */
export interface CompilerOptions {
    /** The command starting it, absent for this process's compiler. */
    readonly command?: CompilerCommand;
    /** The store whose cache its builds reuse and fill. */
    readonly store: LocalPackageStore;
}

/** A sandboxed compiler retained across builds of a workspace's packages. */
export class Compiler implements AsyncDisposable {
    /** The workspace whose packages it builds. */
    readonly workspace: string;
    /** The store whose cache its builds reuse and fill. */
    readonly store: LocalPackageStore;
    /** The compiler's files, its temporary files and the builds it writes, removed on shutdown. */
    readonly #temporary: string;
    /** The directories the compiler reads: the workspace, its tools and the store's trees. */
    readonly #read: readonly string[];
    /** How the compiler starts. */
    readonly #compiler: CompilerCommand;
    /** The sandboxed compiler, started again after a failed one. */
    #sandbox: Sandbox;
    /** Compiler completion, including native compiler shutdown. */
    #closed: Promise<void>;
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

    /** Retain the workspace, store, compiler files, read paths, command and started compiler. */
    private constructor(
        workspace: string,
        store: LocalPackageStore,
        temporary: string,
        read: readonly string[],
        compiler: CompilerCommand,
        sandbox: Sandbox,
    ) {
        // keep the workspace, the store and the compiler's files
        this.workspace = workspace;
        this.store = store;
        this.#temporary = temporary;
        this.#read = read;
        this.#compiler = compiler;

        // watch the started compiler
        this.#sandbox = sandbox;
        this.#closed = this.#watch(sandbox);
    }

    /** Start a reusable compiler for a directory's workspace, building into a store. */
    static async start(directory: string, options: CompilerOptions): Promise<Compiler> {
        // read the workspace, the compiler's tools and the store's trees
        const { store } = options;
        const started = options.command ?? (await CompilerCommand.current());
        const workspace = await realpath(await locateWorkspace(resolve(directory)));
        const paths = [workspace, ...started.read, store.trees];
        const read = [...new Set(await Promise.all(paths.map((path) => realpath(path))))];

        // start the compiler with a temporary directory
        const temporary = await realpath(await mkdtemp(join(tmpdir(), "destack-compile-")));
        try {
            const sandbox = await spawn(workspace, temporary, read, started);

            return new Compiler(workspace, store, temporary, read, started, sandbox);
        } catch (error) {
            await rm(temporary, { recursive: true });
            throw error;
        }
    }

    /** Materialize a request's dependency builds in the store's trees, which the compiler reads. */
    async #install(dependencies: PackageBuildOptions["dependencies"]): Promise<DependencyTree> {
        return await DependencyTree.install(dependencies, this.workspace, this.store);
    }

    /** Start the compiler again after a failed one. */
    async #restart(): Promise<void> {
        // start the next compiler, keeping the failure of the stopped one should it not start
        const sandbox = await spawn(this.workspace, this.#temporary, this.#read, this.#compiler);

        // reset the failure and diagnostics, then watch the next compiler
        this.#failure = undefined;
        this.#stopping = false;
        this.#exitCode = null;
        this.#stderr = "";
        this.#sandbox = sandbox;
        this.#closed = this.#watch(sandbox);
    }

    /** Watch a started compiler's exit, diagnostics and responses, returning its completion. */
    #watch(sandbox: Sandbox): Promise<void> {
        // fail the pending build of a compiler that exits unexpectedly or whose sandbox fails
        const closed = sandbox.exited.then(
            ({ code }) => {
                this.#exitCode = code;
                if (code !== 0 || !this.#disposed) {
                    this.#failure ??= new BuildError(
                        "BUILD_FAILED",
                        `compiler exited (${code}): ${this.#stderr.trim()}`,
                    );
                }
                this.#pending?.reject(this.#failure);
            },
            (cause: unknown) => {
                this.#failure ??= new BuildError("BUILD_FAILED", "the compiler's sandbox failed", {
                    cause,
                });
                this.#pending?.reject(this.#failure);
            },
        );

        // leave a later compiler alone once this one was replaced
        const fail = (error: Error) => {
            if (this.#sandbox === sandbox) {
                this.#fail(error);
            }
        };
        sandbox.stdin.on("error", fail);

        // bound diagnostics retained from compiler tools
        sandbox.stderr.setEncoding("utf8");
        sandbox.stderr.on("data", (chunk: string) => {
            this.#stderr += chunk;
            if (this.#stderr.length > MAX_DIAGNOSTICS) {
                const head = this.#stderr.slice(0, DIAGNOSTICS_HEAD);
                fail(
                    new BuildError(
                        "BUILD_FAILED",
                        `build diagnostics exceeded 1 MiB, starting: ${head}`,
                    ),
                );
            }
        });

        // deliver each response to the pending build
        void this.#deliver(sandbox).catch(fail);

        return closed;
    }

    /** Deliver each compiler response to the build awaiting it, and answer its cache requests. */
    async #deliver(sandbox: Sandbox): Promise<void> {
        for await (const message of readMessages(sandbox.stdout, CompilerMessage)) {
            // refuse a message no request awaits
            const pending = this.#pending;
            if (pending === undefined) {
                throw new BuildError("BUILD_FAILED", "unexpected compiler message");
            }
            // deliver the response to its request
            else if (message.kind === "result") {
                pending.resolve(message);
            }
            // answer a transform's cache request from the pending build's store
            else {
                void this.#answer(sandbox, message).catch((error: unknown) =>
                    this.#fail(
                        new BuildError("BUILD_FAILED", "cannot answer a cache request", {
                            cause: error,
                        }),
                    ),
                );
            }
        }
    }

    /** Read or keep a transform's file in the store. */
    async #answer(sandbox: Sandbox, request: CacheRequest): Promise<void> {
        // read the file a key caches, a miss without an entry
        const { store } = this;
        let bytes: Uint8Array<ArrayBuffer> | null = null;
        if (request.kind === "read") {
            const entry = await store.cached(request.key);
            bytes = entry?.kind === "file" ? await store.file(entry.file) : null;
        }
        // keep the file under its key once stored
        else {
            const path = `${TRANSFORM_DIRECTORY}/${request.key}`;
            const file = await PackageFile.describe(
                path,
                "application/octet-stream",
                request.bytes,
            );
            await store.putFile(file, async () => request.bytes);
            await store.cache(request.key, { kind: "file", file });
        }

        // answer the request
        await writeMessage(sandbox.stdin, { kind: "cached", id: request.id, bytes });
    }

    /** Build a package's current source after the requests before it. */
    build(options: PackageBuildOptions): Promise<PackageBuild> {
        return this.#serially(() => this.#build(options));
    }

    /** Build a package's current source using retained compiler state. */
    async #build(options: PackageBuildOptions): Promise<PackageBuild> {
        // build the package at its real directory
        const { signal, timeout, ...given } = options;
        const requested = { ...given, directory: await realpath(resolve(given.directory)) };

        // install the dependencies, then build into temporary files moved into place once complete
        const tree = await this.#install(requested.dependencies);
        const request = { ...requested, tree: tree.directories };
        const written = join(this.#temporary, "builds", String(this.#builds++));
        const destination = await mkdtemp(join(tmpdir(), "destack-build-"));
        try {
            return await this.#buildCached(request, written, destination, signal, timeout);
        } catch (error) {
            if (this.#stopping) {
                await this.#closed;
            }
            await rm(written, { recursive: true, force: true });
            await rm(destination, { recursive: true, force: true });
            throw error;
        }
    }

    /** Compile the outputs the cache lacks, reusing the others' files, and fill the cache. */
    async #buildCached(
        request: RequestedBuild,
        written: string,
        destination: string,
        signal: AbortSignal | undefined,
        timeout: number | undefined,
    ): Promise<PackageBuild> {
        // compile the outputs the cache lacks beside the cached outputs' files
        const { store } = this;
        const keys = await this.#plan(request, signal, timeout);
        const reuse = await reuseOutputs(store, keys, written);
        const response = await this.#compile(request, reuse, written, signal, timeout);
        await rename(written, destination);
        const reused = Object.keys(reuse);
        const build = new PackageBuild(
            await readBuild(destination),
            destination,
            "temporary",
            reused,
        );

        // refuse an unloadable build, then store it and key each compiled output
        await Compiler.#requireLoadable(build.manifest, store);
        await store.put(build.reader, signal);
        for (const [name, output] of Object.entries(response.outputs)) {
            const key = present(keys[name], `the key of output ${name}`);
            await store.cache(key, { kind: "output", output });
        }

        return build;
    }

    /** Refuse an output importing an npm unit whose build lacks the output's runtime. */
    static async #requireLoadable(manifest: PackageManifest, store: PackageStore): Promise<void> {
        for (const output of Object.values(manifest.outputs)) {
            // take the npm units the output imports
            const units = Object.values(output.dependencies).filter(
                (dependency): dependency is NpmDependency => dependency.kind === "npm",
            );
            for (const unit of units) {
                // refuse a unit without the runtime: a native addon, or one importing such a unit
                const { outputs } = await store.get(unit.manifest);
                if (!Object.values(outputs).some((each) => each.runtime === output.runtime)) {
                    throw new BuildError(
                        "BUILD_FAILED",
                        `the ${output.runtime} output of ${manifest.package.name} needs ${unit.package.name}, which ${output.runtime} cannot load`,
                    );
                }
            }
        }
    }

    /** Derive a build's cache keys inside the compiler. */
    async #plan(
        options: RequestedBuild,
        signal: AbortSignal | undefined,
        timeout: number | undefined,
    ): Promise<OutputKeys> {
        const planned = await this.#request({ kind: "plan", options }, signal, timeout);
        if (planned.kind !== "plan") {
            throw new BuildError("BUILD_FAILED", "unexpected compiler result");
        }

        return planned.keys;
    }

    /** Compile current source into a temporary directory with the reused outputs' files. */
    async #compile(
        options: RequestedBuild,
        reuse: Readonly<Record<string, CachedOutput>>,
        destination: string,
        signal: AbortSignal | undefined,
        timeout: number | undefined,
    ): Promise<Extract<BuildResult, { kind: "build" }>> {
        // send the build with its destination, reused outputs and whether transforms have a cache
        await mkdir(destination, { recursive: true });
        const request: BuildRequest = {
            kind: "build",
            destination,
            options,
            reuse,
        };
        const response = await this.#request(request, signal, timeout);
        if (response.kind !== "build") {
            throw new BuildError("BUILD_FAILED", "unexpected compiler result");
        }

        return response;
    }

    /** Inspect source inside the isolated compiler process after the requests before it. */
    inspect(
        options: InspectOptions & Pick<PackageBuildOptions, "dependencies">,
        signal?: AbortSignal,
    ): Promise<PackageInspection> {
        return this.#serially(async () => {
            // send the inspection at the real directory, the dependencies materialized
            const request = { kind: "inspect", options: await this.#inspected(options) } as const;
            const response = await this.#request(request, signal);
            if (response.kind !== "inspect") {
                throw new BuildError("INSPECTION_FAILED", "unexpected compiler result");
            }

            return response.inspection;
        });
    }

    /** Index source inside the isolated compiler process after the requests before it. */
    index(
        options: InspectOptions & Pick<PackageBuildOptions, "dependencies">,
        signal?: AbortSignal,
    ): Promise<graph.Document[]> {
        return this.#serially(async () => {
            // send the index request at the real directory, the dependencies materialized
            const request = { kind: "index", options: await this.#inspected(options) } as const;
            const response = await this.#request(request, signal);
            if (response.kind !== "index") {
                throw new BuildError("INSPECTION_FAILED", "unexpected compiler result");
            }

            return response.code;
        });
    }

    /** Read inspection options at the real directory, the Destack dependencies materialized. */
    async #inspected(
        options: InspectOptions & Pick<PackageBuildOptions, "dependencies">,
    ): Promise<InspectOptions & Pick<RequestedBuild, "tree">> {
        // resolve the directory and materialize the dependencies' builds
        const { dependencies, ...inspected } = options;
        const directory = await realpath(resolve(inspected.directory));
        const tree = await this.#install(dependencies);

        return { ...inspected, directory, tree: tree.directories };
    }

    /** Convert an installed npm package for workerd inside the compiler, answering its files. */
    convert(request: NpmConversionRequest): ReturnType<NpmConverter> {
        return this.#serially(async () => {
            // convert into a directory of the compiler's own, then read the converted copy
            const destination = await mkdtemp(join(this.#temporary, "npm-"));
            try {
                const response = await this.#request({
                    kind: "convert",
                    source: request.source,
                    destination,
                    name: request.name,
                    dependencies: [...request.dependencies],
                    peers: [...request.peers],
                });
                if (response.kind !== "convert") {
                    throw new BuildError("BUILD_FAILED", "unexpected compiler result");
                }

                return {
                    files: await NpmUnit.files(destination),
                    imported: new Set(response.imported),
                };
            } finally {
                await rm(destination, { recursive: true });
            }
        });
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
            throw new BuildError("BUILD_FAILED", "compiler is closed");
        }
        if (
            timeout !== undefined &&
            (!Number.isSafeInteger(timeout) || timeout <= 0 || timeout > 2 ** 31 - 1)
        ) {
            throw new BuildError("BUILD_FAILED", "invalid compiler timeout");
        }
        if (signal?.aborted === true) {
            throw new BuildError("BUILD_FAILED", "build cancelled", { cause: signal.reason });
        }

        // start a compiler again once a cancelled, late or crashed one has stopped
        if (this.#failure) {
            await this.#closed;
            await this.#restart();
        }
        const abort = () =>
            this.#fail(
                new BuildError("BUILD_FAILED", "build cancelled", { cause: signal?.reason }),
            );
        // fail a build past the caller's deadline, and leave one without a deadline to its signal
        const timer =
            timeout === undefined
                ? undefined
                : setTimeout(
                      () =>
                          this.#fail(
                              new BuildError("BUILD_FAILED", `build exceeded ${timeout} ms`),
                          ),
                      timeout,
                  );
        signal?.addEventListener("abort", abort, { once: true });

        // send the request and wait for its response
        try {
            return resultOf(await this.#send(request));
        } finally {
            clearTimeout(timer);
            signal?.removeEventListener("abort", abort);
            this.#pending = undefined;
        }
    }

    /** Send a request to the compiler and await its response. */
    #send(request: BuildRequest): Promise<BuildResponse> {
        this.#stderr = "";

        return new Promise<BuildResponse>((deliver, reject) => {
            this.#pending = { resolve: deliver, reject };
            void writeMessage(this.#sandbox.stdin, request).catch((error: unknown) =>
                this.#fail(
                    new BuildError("BUILD_FAILED", "cannot send the compiler request", {
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
            this.#fail(new BuildError("BUILD_FAILED", "compiler closed during a build"));
        }
        this.#sandbox.stdin.end();

        // await the compiler's exit once it read the end of its input
        try {
            await this.#closed;
            if (this.#exitCode !== 0 && !this.#stopping) {
                throw present(this.#failure, "the failure of the exited compiler");
            }
        } finally {
            await rm(this.#temporary, { recursive: true });
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
            this.#failure ??= new BuildError("BUILD_FAILED", "cannot stop the compiler", { cause });
        });
    }
}

/** Start a sandboxed compiler in a workspace without network or host environment. */
function spawn(
    workspace: string,
    temporary: string,
    read: readonly string[],
    compiler: CompilerCommand,
): Promise<Sandbox> {
    return Sandbox.start({
        executable: compiler.executable,
        arguments: [...compiler.arguments],
        directory: workspace,
        environment: {
            NODE_ENV: "production",
            HOME: temporary,
            TMPDIR: temporary,
            NO_COLOR: "1",
        },
        read: [...read],
        write: [temporary],
        network: [],
    });
}

/** Read a compiler response's result, rethrowing the failure it reports. */
function resultOf(response: BuildResponse): BuildResult {
    if (response.error !== undefined) {
        const failure = new BuildError(response.error.code, response.error.message, {
            cause: response.error.cause,
        });
        if (response.error.stack !== undefined) {
            failure.stack = response.error.stack;
        }
        throw failure;
    }

    return response.result;
}

/** Write the files of each cached output into a build directory once, returning the outputs. */
async function reuseOutputs(
    store: LocalPackageStore,
    keys: OutputKeys,
    destination: string,
): Promise<Record<string, CachedOutput>> {
    // write each cached output's files no earlier output wrote
    const reuse: Record<string, CachedOutput> = {};
    const placed = new Set<string>();
    for (const [name, key] of Object.entries(keys)) {
        const entry = await store.cached(key);
        if (entry !== undefined) {
            const output = cachedOutput(entry);
            const files = output.files.filter((file) => !placed.has(file.path));
            for (const file of files) {
                await store.place(file, join(destination, file.path));
            }
            for (const file of files) {
                placed.add(file.path);
            }
            reuse[name] = output;
        }
    }

    return reuse;
}

/** Read the output a cached output names, refusing an entry of another kind. */
function cachedOutput(entry: CacheEntry): CachedOutput {
    if (entry.kind !== "output") {
        throw new BuildError("BUILD_FAILED", "an output's cache key names a build");
    }

    return entry.output;
}
