import { type ChildProcessWithoutNullStreams, spawn } from "node:child_process";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { BuildError } from "../error/index.ts";
import type { BuildOptions } from "./build.ts";
import { PackageBuild } from "./build.ts";
import {
    readMessages,
    writeMessage,
    type BuildRequest,
    type BuildResponse,
    type BuildResult,
} from "./message.ts";
import type { InspectOptions } from "../inspect/inspection.ts";
import type { PackageInspection } from "@destack/package/inspect";

/** Environment variables required by the local toolchain. */
const TOOLCHAIN_ENVIRONMENT = [
    "PATH",
    "HOME",
    "USERPROFILE",
    "SYSTEMROOT",
    "WINDIR",
    "COMSPEC",
    "PATHEXT",
    "BUN_INSTALL_CACHE_DIR",
    "XDG_CACHE_HOME",
] as const;

/** An isolated compiler retained across builds of one source checkout. */
export class PackageBuilder implements AsyncDisposable {
    /** The source package directory. */
    readonly directory: string;
    /** Temporary compiler files removed on shutdown. */
    readonly #temporary: string;
    /** The isolated production compiler process. */
    readonly #child: ChildProcessWithoutNullStreams;
    /** Process completion, including native compiler shutdown. */
    readonly #closed: Promise<void>;
    /** The build currently awaiting a response. */
    #pending?: { resolve: (result: BuildResponse) => void; reject: (error: unknown) => void };
    /** A terminal process failure. */
    #failure?: Error;
    /** Whether process-tree termination has already been requested. */
    #stopping = false;
    /** Whether the caller has closed this compiler. */
    #disposed = false;
    /** Shared shutdown, including removal of temporary files. */
    #closing?: Promise<void>;
    /** Exit status reported by the compiler process. */
    #exitCode: number | null = null;
    /** Compiler diagnostics retained until process exit. */
    #stderr = "";

    /** Start the isolated process and receive build results. */
    private constructor(directory: string, temporary: string) {
        // start the compiler process with a production environment
        this.directory = resolve(directory);
        this.#temporary = temporary;
        const environment: Record<string, string> = {
            NODE_ENV: "production",
            TMPDIR: temporary,
            TMP: temporary,
            TEMP: temporary,
            NO_COLOR: "1",
        };
        for (const key of TOOLCHAIN_ENVIRONMENT) {
            if (process.env[key] !== undefined) {
                environment[key] = process.env[key];
            }
        }

        // retain one production environment across sequential build requests
        this.#child = spawn(
            process.execPath,
            [
                "run",
                "--preload",
                fileURLToPath(import.meta.resolve("@destack/package/transform/preload")),
                "--no-env-file",
                fileURLToPath(new URL("./worker.ts", import.meta.url)),
                this.directory,
            ],
            {
                env: environment,
                detached: process.platform !== "win32",
                stdio: ["pipe", "pipe", "pipe"],
            },
        );
        this.#closed = new Promise((resolve) => {
            this.#child.once("close", (code) => {
                // record the exit and fail unexpected exits
                this.#exitCode = code;
                if (code !== 0 || !this.#disposed) {
                    this.#failure ??= new BuildError(
                        "BUILD_FAILED",
                        `Compiler exited (${code}): ${this.#stderr.trim()}`,
                    );
                }
                this.#pending?.reject(this.#failure);
                resolve();
            });
        });
        this.#child.on("error", (error) => this.#fail(error));
        this.#child.stdin.on("error", (error) => this.#fail(error));

        // bound diagnostics retained from compiler tools
        this.#child.stderr.setEncoding("utf8");
        this.#child.stderr.on("data", (chunk: string) => {
            this.#stderr += chunk;
            if (this.#stderr.length > 1_048_576) {
                this.#fail(new BuildError("BUILD_FAILED", "Build diagnostics exceeded 1 MiB."));
            }
        });
        const stdout = this.#child.stdout;
        void (async () => {
            for await (const response of readMessages(stdout)) {
                if (!this.#pending) {
                    throw new BuildError("BUILD_FAILED", "Unexpected compiler response.");
                }
                this.#pending.resolve(response as BuildResponse);
            }
        })().catch((error) => this.#fail(error));
    }

    /** Start a reusable compiler for one source package. */
    static async start(directory: string): Promise<PackageBuilder> {
        const temporary = await mkdtemp(join(tmpdir(), "destack-compile-"));

        try {
            return new PackageBuilder(directory, temporary);
        } catch (error) {
            await rm(temporary, { recursive: true });
            throw error;
        }
    }

    /** Build current source using retained compiler state. */
    async build(options: Omit<BuildOptions, "directory">): Promise<PackageBuild> {
        const { signal, timeout, ...request } = options;
        const destination = await mkdtemp(join(tmpdir(), "destack-build-"));
        try {
            const response = await this.#request(
                { kind: "build", destination, options: request },
                signal,
                timeout,
            );
            if (response.kind !== "build") {
                throw new BuildError("BUILD_FAILED", "unexpected compiler result");
            }

            return new PackageBuild(response.manifest, destination, "temporary");
        } catch (error) {
            if (this.#stopping) {
                await this.#closed;
            }
            await rm(destination, { recursive: true, force: true });
            throw error;
        }
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
        if (this.#failure) {
            throw this.#failure;
        }
        if (this.#pending) {
            throw new BuildError("BUILD_FAILED", "A build is already running.");
        }
        if (signal?.aborted) {
            throw new BuildError("BUILD_FAILED", "Build cancelled.", { cause: signal.reason });
        }
        const abort = () =>
            this.#fail(
                new BuildError("BUILD_FAILED", "Build cancelled.", { cause: signal?.reason }),
            );
        const timer = setTimeout(
            () => this.#fail(new BuildError("BUILD_FAILED", `Build exceeded ${timeout} ms.`)),
            timeout,
        );
        signal?.addEventListener("abort", abort, { once: true });

        // send the request and wait for its response
        try {
            this.#stderr = "";
            const response = await new Promise<BuildResponse>((resolve, reject) => {
                this.#pending = { resolve, reject };
                void writeMessage(this.#child.stdin, request).catch((error) => this.#fail(error));
            });
            if (response.error) {
                const failure = new BuildError(response.error.code, response.error.message, {
                    cause: response.error.cause,
                });
                if (response.error.stack) {
                    failure.stack = response.error.stack;
                }
                throw failure;
            }

            return response.result;
        } finally {
            clearTimeout(timer);
            signal?.removeEventListener("abort", abort);
            this.#pending = undefined;
        }
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
            this.#fail(new BuildError("BUILD_FAILED", "Compiler closed during a build."));
        }
        this.#child.stdin.end();
        let timeout: BuildError | undefined;
        const timer = setTimeout(() => {
            timeout = new BuildError("BUILD_FAILED", "compiler shutdown timed out");
            this.#fail(timeout);
        }, 5000);
        try {
            await this.#closed;
            if (timeout) {
                throw timeout;
            }
            if (this.#exitCode !== 0 && !this.#stopping) {
                throw this.#failure;
            }
        } finally {
            clearTimeout(timer);
            await rm(this.#temporary, { recursive: true });
        }
    }

    /** Terminate the compiler and its native children after a terminal failure. */
    #fail(error: Error): void {
        // reject the pending build once and stop the compiler
        this.#failure ??= error;
        this.#pending?.reject(this.#failure);
        if (this.#stopping) {
            return;
        }
        const pid = this.#child.pid;
        if (pid === undefined || this.#child.exitCode !== null || this.#child.signalCode !== null) {
            return;
        }
        this.#stopping = true;

        // kill the process tree
        if (process.platform === "win32") {
            const killer = spawn("taskkill", ["/pid", String(pid), "/T", "/F"]);
            killer.on("error", (failure) => {
                this.#failure = failure;
            });
        } else {
            try {
                process.kill(-pid, "SIGKILL");
            } catch (failure) {
                if (!(failure instanceof Error && "code" in failure && failure.code === "ESRCH")) {
                    throw failure;
                }
            }
        }
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
