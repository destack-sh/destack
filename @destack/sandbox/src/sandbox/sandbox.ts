import type { FamilyError } from "@destack/error";
import type { Environment, ProcessChannel, Scheduler, Subprocess } from "@destack/environment";
import { isAbsolute, join } from "node:path";
import { SANDBOX_TOOLS, SandboxTool } from "./tools.ts";
import { launcherCommand } from "../bun/command.ts";
import { SandboxError, type SandboxReason } from "../error/index.ts";
import { LauncherMessage, SandboxExit, SandboxOptions } from "./message.ts";
import { SERVER_SOCKET_VARIABLE } from "../workload/index.ts";

/** Maximum time to prepare OS restrictions and launch a process, in milliseconds. */
const START_TIMEOUT_MILLISECONDS = 10000;
/** Maximum time to release a launcher after failed startup, in milliseconds. */
const CLEANUP_TIMEOUT_MILLISECONDS = 1000;
/** Default graceful workload shutdown duration, in milliseconds. */
const STOP_TIMEOUT_MILLISECONDS = 5000;
/** The longest graceful shutdown a caller may request, five minutes, in milliseconds. */
const MAX_STOP_TIMEOUT_MILLISECONDS = 300000;

/** The socket a workload serves its host on, in its temporary folder. */
const SERVER_SOCKET = "server.sock";

/** The search path of the launcher, with the system's tools only. */
const LAUNCHER_PATH = "/usr/bin:/bin:/usr/sbin:/sbin";

/** The launcher process, connected to its parent by a channel. */
type Launcher = Subprocess & { readonly channel: ProcessChannel };

/** A process running under independent filesystem and network restrictions. */
export class Sandbox implements AsyncDisposable {
    /** Workload standard input. */
    readonly stdin: WritableStream<Uint8Array>;
    /** Workload standard output. */
    readonly stdout: ReadableStream<Uint8Array>;
    /** Workload standard error. */
    readonly stderr: ReadableStream<Uint8Array>;
    /** The workload's exit after manager cleanup, rejecting on a launcher or cleanup failure. */
    readonly exited: Promise<SandboxExit>;
    /** The socket the workload serves its host on, in its temporary folder. */
    readonly socket: string;
    /** The launcher process outside the workload sandbox. */
    readonly #launcher: Launcher;
    /** The launcher's watched channel and termination. */
    readonly #watched: WatchedLauncher;
    /** Shared completion of the first shutdown request. */
    #stopping: Promise<SandboxExit> | undefined;
    /** The scheduler a stop's wait for the launcher's exit waits on, as a long operation. */
    readonly #scheduler: Scheduler;

    /** Retain one launcher and the workload's standard streams it passes through. */
    private constructor(
        launcher: Launcher,
        watched: WatchedLauncher,
        exited: Promise<SandboxExit>,
        scheduler: Scheduler,
        socket: string,
    ) {
        // retain the launcher, its watched channel, the streams it passes through and the scheduler
        this.#launcher = launcher;
        this.#watched = watched;
        this.#scheduler = scheduler;
        this.stdin = launcher.stdin;
        this.stdout = launcher.stdout;
        this.stderr = launcher.stderr;
        this.exited = exited;
        this.socket = socket;
    }

    /** Spawn a restricted command, whose application reports its own readiness. */
    static async start(
        options: SandboxOptions,
        environment: Environment<"files" | "process" | "host">,
    ): Promise<Sandbox> {
        // refuse unsupported hosts and unsafe paths before spawning
        await requireSupport(environment);
        requirePaths(options);

        // give the launcher a fresh temporary directory, removed once it closes
        const { files } = environment;
        const temporary = await files.realPath(
            await files.makeTemporaryDirectory("destack-sandbox-"),
        );
        // name the socket the workload serves its host on, in its temporary folder
        const socket = join(temporary, SERVER_SOCKET);
        const launched = {
            ...options,
            environment: { ...options.environment, [SERVER_SOCKET_VARIABLE]: socket },
        };

        // remove the temporary directory once the launcher closes
        const launcher = spawnLauncher(options.directory, temporary, environment);
        const watched = watchLauncher(launcher);
        const closed = watched.closed.then(() =>
            files.remove(temporary, { recursive: true }).catch((cause: unknown) => {
                const message = "cannot remove the sandbox's temporary directory";
                throw SandboxError.of("STOP_FAILED", message, { cause });
            }),
        );

        // bound startup and terminate failed launchers before returning the error
        const deadline = new AbortController();
        const failure = SandboxError.of("START_FAILED", "sandbox startup timed out");
        void environment.clock.sleep(START_TIMEOUT_MILLISECONDS, deadline.signal).then(
            () => watched.ready.reject(failure),
            () => undefined,
        );
        try {
            configure(launcher, launched, watched.ready);
            await environment.scheduler.run(() => watched.ready.promise);

            // expose failures after startup through the public completion promise
            return new Sandbox(
                launcher,
                watched,
                closed.then(watched.outcome),
                environment.scheduler,
                socket,
            );
        } catch (error) {
            // release the launcher, then fail with the errors it wrote
            await release(launcher, watched, environment);
            await closed;
            const errors = (await new Response(launcher.stderr).text()).trim();
            throw errors === ""
                ? error
                : SandboxError.of("START_FAILED", `sandbox failed to start: ${errors}`, {
                      cause: error,
                  });
        } finally {
            deadline.abort();
        }
    }

    /** Stop the workload's process group and release its proxies. */
    stop(gracePeriod = STOP_TIMEOUT_MILLISECONDS): Promise<SandboxExit> {
        // require a bounded grace period
        if (
            !Number.isSafeInteger(gracePeriod) ||
            gracePeriod < 0 ||
            gracePeriod > MAX_STOP_TIMEOUT_MILLISECONDS
        ) {
            throw SandboxError.of(
                "GRACE_PERIOD_INVALID",
                `grace period must be between 0 and ${MAX_STOP_TIMEOUT_MILLISECONDS} milliseconds`,
            );
        }

        // share one shutdown request and deadline across callers
        this.#stopping ??= this.#stop(gracePeriod);

        return this.#stopping;
    }

    /** Request shutdown once and wait for manager cleanup. */
    async #stop(gracePeriod: number): Promise<SandboxExit> {
        if (this.#watched.isConnected()) {
            try {
                this.#launcher.channel.send({ type: "stop", gracePeriod });
            } catch (cause) {
                throw SandboxError.of("STOP_FAILED", "cannot stop sandbox", { cause });
            }
        }

        return await this.#scheduler.waitUntil(this.exited);
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
    /** Settles once the launcher process exits. */
    readonly closed: Promise<void>;
    /** Report whether the channel to the launcher is still open. */
    readonly isConnected: () => boolean;
    /** Read the workload result, throwing the launcher's failure. */
    readonly outcome: () => SandboxExit;
}

/** Refuse platforms without runtime paths and hosts missing the enforcement tools. */
async function requireSupport(
    environment: Environment<"host" | "process" | "files">,
): Promise<void> {
    // load the upstream manager on first start as an operation, since it costs hundreds of milliseconds to import
    const [{ SandboxManager }, { RUNTIME_PATHS }] = await environment.scheduler.run(() =>
        Promise.all([import("@anthropic-ai/sandbox-runtime"), import("../launcher/launcher.ts")]),
    );

    // refuse an unsupported platform
    const { os } = environment.host;
    if (!SandboxManager.isSupportedPlatform() || !RUNTIME_PATHS[os]) {
        throw SandboxError.of("UNSUPPORTED", `sandboxing is unsupported on ${os}`);
    }

    // require a release's own tools, as the launcher's manager checks bubblewrap
    const shipped = SANDBOX_TOOLS.flatMap(
        (tool) => SandboxTool.path(tool, environment.process, os) ?? [],
    );
    if (shipped.length > 0) {
        for (const path of shipped) {
            if (!(await environment.files.exists(path))) {
                throw SandboxError.of("UNSUPPORTED", `the release lacks its sandbox tool ${path}`);
            }
        }
    }
    // require the system's tools otherwise
    else {
        const dependencies = SandboxManager.checkDependencies();
        if (dependencies.errors.length > 0) {
            throw SandboxError.of("UNSUPPORTED", dependencies.errors.join("; "));
        }
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
        ...options.sockets,
    ];
    if (!paths.every(isAbsolute)) {
        throw SandboxError.of("START_FAILED", "sandbox paths must be absolute");
    }

    // prevent literal host paths from becoming upstream glob permissions
    if (paths.some((path) => /[\x00*?[\]{}]/u.test(path))) {
        throw SandboxError.of("START_FAILED", "sandbox paths cannot contain glob characters");
    }
}

/** Spawn the launcher in a directory, omitting host credentials from its environment. */
function spawnLauncher(
    directory: string,
    temporary: string,
    environment: Environment<"process">,
): Launcher {
    const command = launcherCommand(environment.process);

    return environment.process.fork([command.executable, ...command.arguments], {
        isService: true,
        workingDirectory: directory,
        variables: {
            PATH: LAUNCHER_PATH,
            HOME: temporary,
            TMPDIR: temporary,
            CLAUDE_CODE_TMPDIR: temporary,
        },
        stdin: "pipe",
        stdout: "pipe",
        stderr: "pipe",
    });
}

/** Watch a launcher's messages and exit, keeping the workload result apart from failures. */
function watchLauncher(launcher: Launcher): WatchedLauncher {
    // fail startup or shutdown by whether the workload has started
    const ready = Promise.withResolvers<void>();
    let result: SandboxExit | undefined;
    let failure: FamilyError<SandboxReason> | undefined;
    let isStarted = false;
    let isConnected = true;
    const fail = (message: string) => {
        failure = SandboxError.of(isStarted ? "STOP_FAILED" : "START_FAILED", message);
        ready.reject(failure);
    };

    // retain the workload result separately from launcher failures
    launcher.channel.on((received) => {
        // fail the sandbox on a message the launcher protocol has no shape for
        const parsed = LauncherMessage.safeParse(received);
        if (!parsed.success) {
            fail("invalid launcher message");

            return;
        }
        const message = parsed.data;
        if (message.type === "ready") {
            isStarted = true;
            ready.resolve();
        } else if (message.type === "exit") {
            result = message.exit;
        } else if (message.type === "error") {
            fail(message.message);
        }
    });
    void launcher.channel.disconnected.then(() => {
        isConnected = false;
    });
    const closed = launcher.exited.then((code) => {
        // require the workload result and successful manager cleanup
        if (!failure && (code !== 0 || launcher.signal !== undefined || !result)) {
            failure = SandboxError.of(
                isStarted ? "STOP_FAILED" : "START_FAILED",
                "sandbox launcher terminated without successful cleanup",
            );
        }
        ready.reject(failure ?? SandboxError.of("START_FAILED", "sandbox exited before startup"));
    });

    return {
        ready,
        closed,
        isConnected: () => isConnected,
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
    launcher: Launcher,
    options: SandboxOptions,
    ready: PromiseWithResolvers<void>,
): void {
    try {
        launcher.channel.send({ type: "start", options });
    } catch (cause) {
        ready.reject(SandboxError.of("START_FAILED", "cannot configure sandbox", { cause }));
    }
}

/** Disconnect a failed launcher and wait for it to exit, killing it after a grace period. */
async function release(
    launcher: Launcher,
    watched: WatchedLauncher,
    environment: Pick<Environment, "clock" | "scheduler">,
): Promise<void> {
    // disconnect, and kill the launcher once the grace period passes
    if (watched.isConnected()) {
        launcher.channel.disconnect();
    }
    const cleanup = new AbortController();
    void environment.clock.sleep(CLEANUP_TIMEOUT_MILLISECONDS, cleanup.signal).then(
        () => launcher.kill("SIGKILL"),
        () => undefined,
    );
    try {
        await environment.scheduler.waitUntil(watched.closed);
    } finally {
        cleanup.abort();
    }
}
