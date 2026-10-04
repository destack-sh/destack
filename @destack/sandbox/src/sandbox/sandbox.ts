import { spawn, type ChildProcess } from "node:child_process";
import { isAbsolute } from "node:path";
import { tmpdir } from "node:os";
import type { Readable, Writable } from "node:stream";
import { launcherCommand } from "../bun/command.ts";
import { SandboxError } from "../error/index.ts";
import { LauncherMessage, SandboxExit, SandboxOptions } from "./message.ts";

/** Maximum time to prepare OS restrictions and launch a process, in milliseconds. */
const START_TIMEOUT_MILLISECONDS = 10000;
/** Maximum time to release a launcher after failed startup, in milliseconds. */
const CLEANUP_TIMEOUT_MILLISECONDS = 1000;
/** Default graceful workload shutdown duration, in milliseconds. */
const STOP_TIMEOUT_MILLISECONDS = 5000;
/** The longest graceful shutdown a caller may request, five minutes, in milliseconds. */
const MAX_STOP_TIMEOUT_MILLISECONDS = 300000;

/** A process running under independent filesystem and network restrictions. */
export class Sandbox implements AsyncDisposable {
    /** Workload standard input. */
    readonly stdin: Writable;
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

    /** Retain one launcher and the workload's standard streams it passes through. */
    private constructor(launcher: ChildProcess, exited: Promise<SandboxExit>) {
        // retain the launcher and its output streams
        const { stdin, stdout, stderr } = launcher;
        if (stdin === null || stdout === null || stderr === null) {
            throw new TypeError("sandbox launcher has no piped input and output");
        }
        this.#launcher = launcher;
        this.stdin = stdin;
        this.stdout = stdout;
        this.stderr = stderr;
        this.exited = exited;
    }

    /** Spawn a restricted command; application readiness is reported by the application. */
    static async start(options: SandboxOptions): Promise<Sandbox> {
        // refuse unsupported hosts and unsafe paths before spawning
        await requireSupport();
        requirePaths(options);

        // spawn the launcher and watch its messages and termination
        const launcher = spawnLauncher(options.directory);
        const watched = watchLauncher(launcher);

        // bound startup and terminate failed launchers before returning the error
        const timeout = setTimeout(
            () =>
                watched.ready.reject(new SandboxError("START_FAILED", "sandbox startup timed out")),
            START_TIMEOUT_MILLISECONDS,
        );
        try {
            configure(launcher, options, watched.ready);
            await watched.ready.promise;

            // expose failures after startup through the public completion promise
            return new Sandbox(launcher, watched.closed.then(watched.outcome));
        } catch (error) {
            await release(launcher, watched.closed);
            throw error;
        } finally {
            clearTimeout(timeout);
        }
    }

    /** Stop the workload's process group and release its proxies; detached children may survive. */
    stop(gracePeriodMs = STOP_TIMEOUT_MILLISECONDS): Promise<SandboxExit> {
        // require a bounded grace period
        if (
            !Number.isSafeInteger(gracePeriodMs) ||
            gracePeriodMs < 0 ||
            gracePeriodMs > MAX_STOP_TIMEOUT_MILLISECONDS
        ) {
            throw new RangeError(
                `grace period must be between 0 and ${MAX_STOP_TIMEOUT_MILLISECONDS} milliseconds`,
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

/** The launcher's startup signal, termination and workload outcome. */
interface WatchedLauncher {
    /** Settles once the workload starts, rejecting on any startup failure. */
    readonly ready: PromiseWithResolvers<void>;
    /** Settles once the launcher process closes. */
    readonly closed: Promise<void>;
    /** Read the workload result, throwing the launcher's failure. */
    readonly outcome: () => SandboxExit;
}

/** Refuse platforms without runtime paths and hosts missing the enforcement tools. */
async function requireSupport(): Promise<void> {
    // load the upstream manager on first start, since it costs hundreds of milliseconds to import
    const [{ SandboxManager }, { RUNTIME_PATHS }] = await Promise.all([
        import("@anthropic-ai/sandbox-runtime"),
        import("../launcher/launcher.ts"),
    ]);

    // refuse an unsupported platform or missing tools
    if (!SandboxManager.isSupportedPlatform() || !RUNTIME_PATHS[process.platform]) {
        throw new SandboxError("UNSUPPORTED", `sandboxing is unsupported on ${process.platform}`);
    }
    const dependencies = SandboxManager.checkDependencies();
    if (dependencies.errors.length > 0) {
        throw new SandboxError("UNSUPPORTED", dependencies.errors.join("; "));
    }
}

/** Require absolute, glob-free paths for every permission. */
function requirePaths(options: SandboxOptions): void {
    // require absolute paths
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
    if (paths.some((path) => /[\x00*?[\]{}]/u.test(path))) {
        throw new SandboxError("START_FAILED", "sandbox paths cannot contain glob characters");
    }
}

/** Spawn the launcher in a directory, omitting host credentials from its environment. */
function spawnLauncher(directory: string): ChildProcess {
    const command = launcherCommand();

    return spawn(command.executable, command.arguments, {
        cwd: directory,
        env: {
            PATH: "/usr/bin:/bin:/usr/sbin:/sbin",
            HOME: tmpdir(),
            TMPDIR: tmpdir(),
        },
        stdio: ["pipe", "pipe", "pipe", "ipc"],
    });
}

/** Watch a launcher's messages and termination, retaining the workload result apart from failures. */
function watchLauncher(launcher: ChildProcess): WatchedLauncher {
    // fail startup or shutdown by whether the workload has started
    const ready = Promise.withResolvers<void>();
    const closed = Promise.withResolvers<void>();
    let result: SandboxExit | undefined;
    let failure: SandboxError | undefined;
    let isStarted = false;
    const fail = (message: string, options?: ErrorOptions) => {
        failure = new SandboxError(isStarted ? "STOP_FAILED" : "START_FAILED", message, options);
        ready.reject(failure);
    };

    // retain the workload result separately from launcher failures
    launcher.on("message", (received: unknown) => {
        const message = LauncherMessage.parse(received);
        if (message.type === "ready") {
            isStarted = true;
            ready.resolve();
        } else if (message.type === "exit") {
            result = message.exit;
        } else if (message.type === "error") {
            fail(message.message);
        }
    });
    launcher.once("error", (cause) => fail("sandbox launcher failed", { cause }));
    launcher.once("close", (code, signal) => {
        // require the workload result and successful manager cleanup
        if (!failure && (code !== 0 || signal !== null || !result)) {
            failure = new SandboxError(
                isStarted ? "STOP_FAILED" : "START_FAILED",
                "sandbox launcher terminated without successful cleanup",
            );
        }
        ready.reject(failure ?? new SandboxError("START_FAILED", "sandbox exited before startup"));
        closed.resolve();
    });

    return {
        ready,
        closed: closed.promise,
        outcome: () => {
            // throw the launcher's failure or a missing result
            if (failure) {
                throw failure;
            } else if (result === undefined) {
                throw new TypeError("sandbox launcher closed without a workload result");
            }

            return result;
        },
    };
}

/** Send the sandbox options to the launcher, failing startup when they cannot be sent. */
function configure(
    launcher: ChildProcess,
    options: SandboxOptions,
    ready: PromiseWithResolvers<void>,
): void {
    launcher.send({ type: "start", options }, (error) => {
        if (error) {
            ready.reject(
                new SandboxError("START_FAILED", "cannot configure sandbox", { cause: error }),
            );
        }
    });
}

/** Disconnect a failed launcher and wait for it to close, killing it after a grace period. */
async function release(launcher: ChildProcess, closed: Promise<void>): Promise<void> {
    if (launcher.connected) {
        launcher.disconnect();
    }
    const cleanup = setTimeout(() => launcher.kill("SIGKILL"), CLEANUP_TIMEOUT_MILLISECONDS);
    try {
        await closed;
    } finally {
        clearTimeout(cleanup);
    }
}
