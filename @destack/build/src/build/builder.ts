import { type PackageInspection } from "@destack/package/code";
import { mkdir, mkdtemp, realpath, rename, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { Sandbox } from "@destack/sandbox";
import { Toolchain } from "@destack/check/toolchain";
import { BunProcess } from "@destack/check/bun";
import { BuildError } from "../error/index.ts";
import { locateWorkspace } from "@destack/check/workspace";
import { present } from "@destack/schema";
import type { PackageFile } from "@destack/package/file";
import type { PackageManifest } from "@destack/package/manifest";
import type { BuildKeys, CacheEntry, CachedOutput } from "../cache/index.ts";
import type { PackageStore } from "../store/index.ts";
import type { BuildOptions } from "./build.ts";
import { PackageBuild } from "./build.ts";
import {
    BuildResponse,
    readMessages,
    writeMessage,
    type BuildRequest,
    type BuildResult,
    type RequestedBuild,
} from "./message.ts";
import type { InspectOptions } from "../inspect/inspection.ts";

/** The compiler executable beside a standalone Destack executable. */
const EXECUTABLE = "destack-build";

/** The most diagnostics a compiler retains, 1 MiB. */
const MAX_DIAGNOSTICS = 1_048_576;

/** How a host starts the compiler: its command, and the directories with the tools it reads. */
export interface Compiler {
    /** The executable starting the compiler. */
    readonly executable: string;
    /** The arguments before the package directory. */
    readonly arguments: readonly string[];
    /** The directories with the compiler and its tools. */
    readonly read: readonly string[];
}

/** The compilers a host starts: a release's beside its executables, or a workspace's from source. */
export const Compiler = {
    /** Name the released compiler beside an executable: `destack-build` reading the toolchain directory beside it. */
    beside(executable: string): Compiler {
        const compiler = join(dirname(executable), EXECUTABLE);

        return {
            executable: compiler,
            arguments: [],
            read: [compiler, Toolchain.beside(executable)],
        };
    },

    /** Find the compiler this process starts: the released one beside a standalone executable, else Bun running this package's entry from its workspace. */
    async current(): Promise<Compiler> {
        // start the release's compiler from a standalone executable
        if (BunProcess.isStandalone) {
            return Compiler.beside(process.execPath);
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

/** A sandboxed compiler retained across builds of one source checkout. */
export class PackageBuilder implements AsyncDisposable {
    /** The source package directory. */
    readonly directory: string;
    /** The compiler's files, its temporary files and the builds it writes, removed on shutdown. */
    readonly #temporary: string;
    /** The directories the compiler reads: the package's workspace with its installed dependencies, and the compiler's tools. */
    readonly #read: readonly string[];
    /** How the compiler starts. */
    readonly #compiler: Compiler;
    /** The sandboxed compiler, started again after a failed one. */
    #sandbox: Sandbox;
    /** Compiler completion, including native compiler shutdown. */
    #closed: Promise<void>;
    /** The build currently awaiting a response. */
    #pending:
        | { resolve: (result: BuildResponse) => void; reject: (error: unknown) => void }
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
    /** The builds the compiler wrote, naming the directory of the next. */
    #builds = 0;

    /** Retain the package directory, the compiler's files, what it reads, how it starts and the started compiler. */
    private constructor(
        directory: string,
        temporary: string,
        read: readonly string[],
        compiler: Compiler,
        sandbox: Sandbox,
    ) {
        // keep the package and the compiler's files
        this.directory = directory;
        this.#temporary = temporary;
        this.#read = read;
        this.#compiler = compiler;

        // watch the started compiler
        this.#sandbox = sandbox;
        this.#closed = this.#watch(sandbox);
    }

    /** Start a reusable compiler for one source package, the one this process starts unless given another. */
    static async start(directory: string, compiler?: Compiler): Promise<PackageBuilder> {
        // read the package's workspace with its installed dependencies, and the compiler's tools
        directory = await realpath(resolve(directory));
        const started = compiler ?? (await Compiler.current());
        const workspace = await locateWorkspace(directory);
        const paths = await Promise.all([workspace, ...started.read].map((path) => realpath(path)));
        const read = [...new Set(paths)];

        // start the compiler with a temporary directory
        const temporary = await realpath(await mkdtemp(join(tmpdir(), "destack-compile-")));
        try {
            const sandbox = await spawn(directory, temporary, read, started);

            return new PackageBuilder(directory, temporary, read, started, sandbox);
        } catch (error) {
            await rm(temporary, { recursive: true });
            throw error;
        }
    }

    /** Start the compiler again after a failed one. */
    async #restart(): Promise<void> {
        // reset the failure and diagnostics of the stopped compiler
        this.#failure = undefined;
        this.#stopping = false;
        this.#exitCode = null;
        this.#stderr = "";

        // start and watch the next compiler
        const sandbox = await spawn(this.directory, this.#temporary, this.#read, this.#compiler);
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
                fail(new BuildError("BUILD_FAILED", "build diagnostics exceeded 1 MiB"));
            }
        });

        // deliver each response to the pending build
        void this.#deliver(sandbox).catch(fail);

        return closed;
    }

    /** Deliver each compiler response to the build awaiting it. */
    async #deliver(sandbox: Sandbox): Promise<void> {
        for await (const response of readMessages(sandbox.stdout, BuildResponse)) {
            if (!this.#pending) {
                throw new BuildError("BUILD_FAILED", "unexpected compiler response");
            }
            this.#pending.resolve(response);
        }
    }

    /** Build current source using retained compiler state, reusing and filling the store's cache when given one. */
    async build(options: Omit<BuildOptions, "directory">): Promise<PackageBuild> {
        // write into the compiler's temporary files, moving the build to the destination once complete
        const { signal, timeout, store, ...request } = options;
        const destination = await mkdtemp(join(tmpdir(), "destack-build-"));
        const written = join(this.#temporary, "builds", String(this.#builds++));
        try {
            // build without a cache
            if (store === undefined) {
                const response = await this.#compile(request, {}, written, signal, timeout);
                await rename(written, destination);

                return new PackageBuild(response.manifest, destination, "temporary");
            }
            // build through the store's cache
            else {
                const call = { request, written, destination, signal, timeout };

                return await this.#buildCached(store, call);
            }
        } catch (error) {
            if (this.#stopping) {
                await this.#closed;
            }
            await rm(written, { recursive: true, force: true });
            await rm(destination, { recursive: true, force: true });
            throw error;
        }
    }

    /** Restore a whole build the cache names, else compile the outputs it lacks and fill it. */
    async #buildCached(
        store: PackageStore,
        call: {
            readonly request: RequestedBuild;
            readonly written: string;
            readonly destination: string;
            readonly signal: AbortSignal | undefined;
            readonly timeout: number | undefined;
        },
    ): Promise<PackageBuild> {
        // restore a whole build the cache names
        const { request, written, destination, signal, timeout } = call;
        const keys = await this.#plan(request, signal, timeout);
        const whole = await store.cached(keys.build);
        if (whole !== undefined) {
            const manifest = await restore(store, cachedManifest(whole), destination);
            const reused = Object.keys(keys.outputs);

            return new PackageBuild(manifest, destination, "temporary", reused);
        }

        // write the files of each output the cache names, then compile the others
        const reuse = await reuseOutputs(store, keys, written);
        const response = await this.#compile(request, reuse, written, signal, timeout);
        await rename(written, destination);
        const reused = Object.keys(reuse);
        const build = new PackageBuild(response.manifest, destination, "temporary", reused);

        // store the build, then name it and each compiled output by its key
        const manifest = await store.put(build.reader, signal);
        for (const [name, output] of Object.entries(response.outputs)) {
            const key = present(keys.outputs[name], `the key of output ${name}`);
            await store.cache(key, { kind: "output", output });
        }
        await store.cache(keys.build, { kind: "build", manifest });

        return build;
    }

    /** Derive a build's cache keys inside the compiler. */
    async #plan(
        options: RequestedBuild,
        signal: AbortSignal | undefined,
        timeout: number | undefined,
    ): Promise<BuildKeys> {
        const planned = await this.#request({ kind: "plan", options }, signal, timeout);
        if (planned.kind !== "plan") {
            throw new BuildError("BUILD_FAILED", "unexpected compiler result");
        }

        return planned.keys;
    }

    /** Compile current source into a directory among the compiler's temporary files, holding the files of the reused outputs. */
    async #compile(
        options: RequestedBuild,
        reuse: Readonly<Record<string, CachedOutput>>,
        destination: string,
        signal: AbortSignal | undefined,
        timeout: number | undefined,
    ): Promise<Extract<BuildResult, { kind: "build" }>> {
        // send the build with its destination and the reused outputs
        await mkdir(destination, { recursive: true });
        const response = await this.#request(
            { kind: "build", destination, options, reuse },
            signal,
            timeout,
        );
        if (response.kind !== "build") {
            throw new BuildError("BUILD_FAILED", "unexpected compiler result");
        }

        return response;
    }

    /** Inspect source inside the isolated compiler process. */
    async inspect(
        options: Omit<InspectOptions, "directory">,
        signal?: AbortSignal,
    ): Promise<PackageInspection> {
        const response = await this.#request({ kind: "inspect", options }, signal);
        if (response.kind !== "inspect") {
            throw new BuildError("INSPECTION_FAILED", "unexpected compiler result");
        }

        return response.inspection;
    }

    /** Send one request and retain its cancellation until the compiler responds. */
    async #request(
        request: BuildRequest,
        signal?: AbortSignal,
        timeout = 60_000,
    ): Promise<BuildResult> {
        // serialize builds within this checkout and keep cancellation explicit
        if (this.#disposed) {
            throw new BuildError("BUILD_FAILED", "compiler is closed");
        }
        if (!Number.isSafeInteger(timeout) || timeout <= 0 || timeout > 2 ** 31 - 1) {
            throw new BuildError("BUILD_FAILED", "invalid compiler timeout");
        }
        if (this.#pending) {
            throw new BuildError("BUILD_FAILED", "a build is already running");
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
        const timer = setTimeout(
            () => this.#fail(new BuildError("BUILD_FAILED", `build exceeded ${timeout} ms`)),
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

    /** Stop the compiler and remove its temporary files. */
    [Symbol.asyncDispose](): Promise<void> {
        return (this.#closing ??= this.#close());
    }

    /** Drain the compiler before releasing its temporary files. */
    async #close(): Promise<void> {
        // mark disposal, fail a pending build and end the compiler input
        this.#disposed = true;
        if (this.#pending) {
            this.#fail(new BuildError("BUILD_FAILED", "compiler closed during a build"));
        }
        this.#sandbox.stdin.end();

        // await the compiler's exit once it read the end: its startup, its native compilers' exits and its sandbox's release
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

/** Start a sandboxed compiler without network or host environment, writing only its temporary files. */
function spawn(
    directory: string,
    temporary: string,
    read: readonly string[],
    compiler: Compiler,
): Promise<Sandbox> {
    return Sandbox.start({
        executable: compiler.executable,
        arguments: [...compiler.arguments, directory],
        directory,
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

/** Write the files of each output the cache names into a build directory once, outputs sharing a file among them, returning the outputs to reuse. */
async function reuseOutputs(
    store: PackageStore,
    keys: BuildKeys,
    destination: string,
): Promise<Record<string, CachedOutput>> {
    // write each cached output's files no earlier output wrote
    const reuse: Record<string, CachedOutput> = {};
    const placed = new Set<string>();
    for (const [name, key] of Object.entries(keys.outputs)) {
        const entry = await store.cached(key);
        if (entry !== undefined) {
            const output = cachedOutput(entry);
            const files = output.files.filter((file) => !placed.has(file.path));
            await place(store, files, destination);
            for (const file of files) {
                placed.add(file.path);
            }
            reuse[name] = output;
        }
    }

    return reuse;
}

/** Read the manifest a cached build names, refusing an entry of another kind. */
function cachedManifest(entry: CacheEntry): string {
    if (entry.kind !== "build") {
        throw new BuildError("BUILD_FAILED", "a build's cache key names an output");
    }

    return entry.manifest;
}

/** Read the output a cached output names, refusing an entry of another kind. */
function cachedOutput(entry: CacheEntry): CachedOutput {
    if (entry.kind !== "output") {
        throw new BuildError("BUILD_FAILED", "an output's cache key names a build");
    }

    return entry.output;
}

/** Write a stored build's files and manifest into a directory, returning the manifest. */
async function restore(
    store: PackageStore,
    digest: string,
    destination: string,
): Promise<PackageManifest> {
    // write the files, then the manifest naming them
    const reader = await store.contents(digest);
    const { manifest } = reader;
    await place(store, await reader.distributed(), destination);
    await writeFile(join(destination, "manifest.json"), JSON.stringify(manifest), { flag: "wx" });

    return manifest;
}

/** Write stored files into a directory at their paths. */
async function place(
    store: PackageStore,
    files: readonly PackageFile[],
    destination: string,
): Promise<void> {
    for (const file of files) {
        const path = join(destination, file.path);
        await mkdir(dirname(path), { recursive: true });
        await writeFile(path, await store.file(file), { flag: "wx" });
    }
}

/** Compile once and close the isolated compiler. */
export async function buildPackage(options: BuildOptions): Promise<PackageBuild> {
    // reject a cancelled request before starting a compiler
    options.signal?.throwIfAborted();
    const { directory, ...request } = options;
    let result: PackageBuild | undefined;

    // transfer the result only after the compiler has closed successfully
    try {
        await using builder = await PackageBuilder.start(directory);
        result = await builder.build(request);
    } catch (error) {
        await result?.[Symbol.asyncDispose]();
        throw error;
    }

    return result;
}
