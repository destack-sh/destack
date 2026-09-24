import { spawn, type ChildProcess } from "node:child_process";
import { dirname, isAbsolute, join } from "node:path";
import { fileURLToPath } from "node:url";
import { tmpdir } from "node:os";
import type { Readable } from "node:stream";
import { SandboxError } from "../error/index.ts";

/** Maximum time to prepare OS restrictions and launch a process, in milliseconds. */
const START_TIMEOUT_MS = 10000;
/** Maximum time to release a launcher after failed startup, in milliseconds. */
const CLEANUP_TIMEOUT_MS = 1000;
/** Default graceful workload shutdown duration, in milliseconds. */
const STOP_TIMEOUT_MS = 5000;

/** A process running under independent filesystem and network restrictions. */
export class Sandbox implements AsyncDisposable {
    /** Workload standard output. */
    readonly stdout: Readable;
    /** Workload standard error. */
    readonly stderr: Readable;
    /** Workload termination after manager cleanup; rejects on launcher or cleanup failure. */
    readonly exited: Promise<SandboxExit>;
    /** The launcher process outside the workload sandbox. */
    readonly #launcher: ChildProcess;
    /** Shared completion of the first shutdown request. */
    #stopping: Promise<SandboxExit> | undefined;

    /** Retain one launcher until its process and proxies close. */
    private constructor(launcher: ChildProcess, exited: Promise<SandboxExit>) {
        // retain the launcher and its output streams
        this.#launcher = launcher;
        this.stdout = launcher.stdout!;
        this.stderr = launcher.stderr!;
        this.exited = exited;
    }

    /** Spawn a restricted command; application readiness is reported by the application. */
    static async start(options: SandboxOptions): Promise<Sandbox> {
        // refuse platforms whose enforcement has not been verified
        if (process.platform !== "darwin") {
            throw new SandboxError(
                "UNSUPPORTED",
                "sandbox enforcement is not verified on this platform",
            );
        }
        const paths = [
            options.executable,
            options.directory,
            ...options.read,
            ...options.write,
            ...(options.sockets ?? []),
        ];
        if (!paths.every(isAbsolute)) {
            throw new SandboxError("START_FAILED", "sandbox paths must be absolute");
        }

        // prevent literal host paths from becoming upstream glob permissions
        if (paths.some((path) => /[\x00*?[\]{}]/.test(path))) {
            throw new SandboxError("START_FAILED", "sandbox paths cannot contain glob characters");
        }

        // isolate the upstream manager and omit host credentials from its environment
        const executable = Bun.isStandaloneExecutable
            ? join(dirname(process.execPath), "destack-sandbox")
            : process.execPath;
        const arguments_ = Bun.isStandaloneExecutable
            ? []
            : ["run", "--no-env-file", fileURLToPath(new URL("../main.ts", import.meta.url))];
        const launcher = spawn(executable, arguments_, {
            cwd: options.directory,
            env: {
                PATH: "/usr/bin:/bin:/usr/sbin:/sbin",
                HOME: options.directory,
                TMPDIR: tmpdir(),
            },
            stdio: ["ignore", "pipe", "pipe", "ipc"],
        });
        const ready = Promise.withResolvers<void>();
        const closed = Promise.withResolvers<void>();
        let result: SandboxExit | undefined;
        let failure: SandboxError | undefined;
        let isStarted = false;

        // retain the workload result separately from launcher failures
        launcher.on("message", (message: LauncherMessage) => {
            if (message.type === "ready") {
                isStarted = true;
                ready.resolve();
            } else if (message.type === "exit") {
                result = message.exit;
            } else if (message.type === "error") {
                failure = new SandboxError(
                    isStarted ? "STOP_FAILED" : "START_FAILED",
                    message.message,
                );
                ready.reject(failure);
            }
        });
        launcher.once("error", (cause) => {
            failure = new SandboxError(
                isStarted ? "STOP_FAILED" : "START_FAILED",
                "sandbox launcher failed",
                { cause },
            );
            ready.reject(failure);
        });
        launcher.once("close", (code, signal) => {
            // require the workload result and successful manager cleanup
            if (!failure && (code !== 0 || signal !== null || !result)) {
                failure = new SandboxError(
                    isStarted ? "STOP_FAILED" : "START_FAILED",
                    "sandbox launcher terminated without successful cleanup",
                );
            }
            ready.reject(
                failure ?? new SandboxError("START_FAILED", "sandbox exited before startup"),
            );
            closed.resolve();
        });

        // bound startup and terminate failed launchers before returning the error
        const timeout = setTimeout(
            () => ready.reject(new SandboxError("START_FAILED", "sandbox startup timed out")),
            START_TIMEOUT_MS,
        );
        try {
            launcher.send({ type: "start", options }, (error) => {
                if (error) {
                    ready.reject(
                        new SandboxError("START_FAILED", "cannot configure sandbox", {
                            cause: error,
                        }),
                    );
                }
            });
            await ready.promise;

            // expose failures after startup through the public completion promise
            const exited = closed.promise.then(() => {
                if (failure) {
                    throw failure;
                }

                return result!;
            });

            return new Sandbox(launcher, exited);
        } catch (error) {
            if (launcher.connected) {
                launcher.disconnect();
            }
            const timeout = setTimeout(() => launcher.kill("SIGKILL"), CLEANUP_TIMEOUT_MS);
            try {
                await closed.promise;
            } finally {
                clearTimeout(timeout);
            }
            throw error;
        } finally {
            clearTimeout(timeout);
        }
    }

    /** Stop the workload's process group and release its proxies; detached children may survive. */
    stop(gracePeriodMs = STOP_TIMEOUT_MS): Promise<SandboxExit> {
        if (!Number.isSafeInteger(gracePeriodMs) || gracePeriodMs < 0 || gracePeriodMs > 300000) {
            throw new SandboxError(
                "STOP_FAILED",
                "grace period must be between 0 and 300000 milliseconds",
            );
        }

        // share one shutdown request and deadline across callers
        this.#stopping ??= this.#stop(gracePeriodMs);

        return this.#stopping;
    }

    /** Request shutdown once and wait for manager cleanup. */
    async #stop(gracePeriodMs: number): Promise<SandboxExit> {
        if (this.#launcher.connected) {
            await new Promise<void>((resolve, reject) => {
                this.#launcher.send({ type: "stop", gracePeriodMs }, (cause) => {
                    if (cause) {
                        reject(new SandboxError("STOP_FAILED", "cannot stop sandbox", { cause }));
                    } else {
                        resolve();
                    }
                });
            });
        }

        return await this.exited;
    }

    /** Stop execution before releasing the sandbox. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.stop();
    }
}

/** Explicit process inputs and host-approved access. */
export interface SandboxOptions {
    /** Absolute executable path. */
    executable: string;
    /** Arguments passed unchanged to the executable. */
    arguments: string[];
    /** Absolute working directory. */
    directory: string;
    /** Environment supplied to the workload, excluding inherited host variables. */
    environment: Record<string, string>;
    /** Absolute readable paths, in addition to the executable and required OS runtime files. */
    read: string[];
    /** Absolute writable files and directories, also readable. */
    write: string[];
    /** Allowed outgoing domains through the supplied proxies, optionally qualified by port. */
    network: string[];
    /** Unix sockets available for host-mediated service connections. */
    sockets?: string[];
}

/** Process termination reported after sandbox cleanup. */
export interface SandboxExit {
    /** Exit code, absent when a signal terminates the process. */
    code: number | null;
    /** Terminating signal, absent after a normal exit. */
    signal: NodeJS.Signals | null;
}

/** Launcher lifecycle messages over its private parent connection. */
type LauncherMessage =
    | {
          /** The command that applies OS restrictions has spawned. */
          type: "ready";
      }
    | {
          /** The workload has terminated. */
          type: "exit";
          /** Process termination. */
          exit: SandboxExit;
      }
    | {
          /** Startup or cleanup failed. */
          type: "error";
          /** Failure description. */
          message: string;
      };
