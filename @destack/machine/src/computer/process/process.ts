import { TelemetryScope } from "@destack/telemetry";
import { Bookmark } from "@destack/object/client";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import type { Environment } from "@destack/environment";
import { Release } from "@destack/update/release";
import { UniverseHome } from "./home.ts";
import { waitAborted } from "@destack/environment";
import { type LoopbackEndpoint, ENDPOINT_FILE, LOCK_FILE, Loopback } from "./endpoint.ts";
import { MachineError } from "../../error/index.ts";
import { Autostart } from "../autostart/index.ts";

/** The host starting and stopping a machine: its files, processes, servers and platform. */
export type MachineHost = Environment<"files" | "process" | "host" | "server">;

/** The environment the machine runs with: system values and its configuration, no credentials. */
const ENVIRONMENT = [
    "HOME",
    "USERPROFILE",
    "PATH",
    "SystemRoot",
    "SYSTEMROOT",
    "WINDIR",
    "LOCALAPPDATA",
    "APPDATA",
    "TMPDIR",
    "TMP",
    "TEMP",
    "LANG",
    "LC_ALL",
    "DESTACK_PROFILE",
    "DESTACK_UNIVERSE_ORIGIN",
    "DESTACK_PORT",
    "DESTACK_DISTRIBUTION",
];

/** The permissions of the machine's private state directory: the person's alone. */
const PRIVATE_MODE = 0o700;

/** The permissions of the endpoint file, whose token only the person reads. */
export const ENDPOINT_MODE = 0o600;

/** How long a stop may take to drain and close storage and telemetry, in milliseconds. */
const STOP_TIMEOUT_MILLISECONDS = 15000;

/** How long a client waits for another client's launch to finish, in milliseconds: two launches. */
const START_LOCK_TIMEOUT_MILLISECONDS = 10000;

/** How long a launched machine may take to answer, in milliseconds. */
const LAUNCH_TIMEOUT_MILLISECONDS = 5000;

/** The socket a starting client listens at for the machine's notification that it serves. */
const READY_SOCKET = "ready.sock";

/** The file in the machine's directory its output goes to. */
const LOG_FILE = "machine.log";

/** How long a machine the OS supervisor starts may take to answer: a launch plus scheduling. */
const SUPERVISOR_TIMEOUT_MILLISECONDS = 10000;

/** The machine process of a state directory: its launch, its loopback and its stop. */
export class MachineProcess {
    /** The machine's private state directory. */
    readonly directory: string;
    /** The host starting and stopping the machine. */
    readonly environment: MachineHost;
    /** The owner's loopback connection to the running machine. */
    readonly loopback: Loopback;

    /** The telemetry of the launches and shutdowns. */
    readonly #scope: TelemetryScope;

    /** Address the machine process of a state directory. */
    constructor(directory: string, environment: MachineHost) {
        // take the directory and the environment, with the telemetry and the loopback on them
        this.directory = directory;
        this.environment = environment;
        this.#scope = TelemetryScope.of(import.meta.destack.package, environment.telemetry);
        this.loopback = new Loopback(directory, environment, new Bookmark());
    }

    /** Address the machine process of the current release home. */
    static current(environment: MachineHost): MachineProcess {
        return new MachineProcess(UniverseHome.current(environment).directory, environment);
    }

    /** Recognize connection failures retained by the service client's transport error. */
    static isUnavailable(error: unknown): boolean {
        // recognize a stopped machine
        if (MachineError.is(error)) {
            return error.reason === "MACHINE_UNAVAILABLE";
        }
        // look through the transport error's cause
        else if (error instanceof Error && error.cause !== undefined) {
            return MachineProcess.isUnavailable(error.cause);
        }
        // refuse anything else
        else {
            return false;
        }
    }

    /** Create the machine's private state directory before its process starts. */
    async createDirectory(): Promise<void> {
        const { files, host } = this.environment;
        await files.mkdir(this.directory, { recursive: true, mode: PRIVATE_MODE });
        if (host.os !== "win32") {
            await files.chmod(this.directory, PRIVATE_MODE);
        }
    }

    /** Start a detached machine and wait until it accepts requests. */
    start(
        entry: string,
        executable = this.environment.process.executable,
        isStandalone = this.environment.process.runtime.isStandalone,
    ): Promise<void> {
        return this.#scope.span("destack.machine.launch", {}, () =>
            this.#start(entry, executable, isStandalone),
        );
    }

    /** Start a detached machine unless one answers, one launch at a time. */
    async #start(entry: string, executable: string, isStandalone: boolean): Promise<void> {
        // reuse an authenticated machine that is already running
        if (await this.#answers()) {
            return;
        }

        // serialize concurrent client launches and recheck after acquiring ownership
        await this.createDirectory();
        const startup = await this.environment.files.lock(
            join(this.directory, "machine.start.lock"),
            "exclusive",
            { signal: this.environment.clock.timeout(START_LOCK_TIMEOUT_MILLISECONDS) },
        );
        try {
            // recheck once the startup lock is ours
            if (await this.#answers()) {
                return;
            }

            // let the OS supervisor start a registered machine
            const autostart = await Autostart.of(this.directory, this.environment);
            if (autostart.isSupervised && (await autostart.isRegistered())) {
                await this.#started(SUPERVISOR_TIMEOUT_MILLISECONDS, async () => {
                    await autostart.start();

                    return undefined;
                });
            }
            // launch one detached otherwise
            else {
                await this.#launch(isStandalone ? undefined : entry, executable);
            }
        } finally {
            await startup.close();
        }
    }

    /** Stop the machine and wait until it releases its database and process lock. */
    stop(): Promise<void> {
        return this.#scope.span("destack.machine.shutdown", {}, () => this.#stop());
    }

    /** Stop the machine through its supervisor or loopback, then wait for its lock. */
    async #stop(): Promise<void> {
        // stop OS supervision before waiting for the process to release its database
        const autostart = await Autostart.of(this.directory, this.environment);
        if (autostart.isSupervised && (await autostart.isRegistered())) {
            await autostart.stop();
            try {
                await this.loopback.stop();
            } catch (error) {
                if (!MachineProcess.isUnavailable(error)) {
                    throw error;
                }
            }
        }
        // stop the machine through its loopback otherwise
        else {
            await this.loopback.stop();
        }

        // observe completion without forcing a process to exit
        try {
            const lock = await this.environment.files.lock(
                join(this.directory, LOCK_FILE),
                "exclusive",
                { signal: this.environment.clock.timeout(STOP_TIMEOUT_MILLISECONDS) },
            );
            await lock.close();
        } catch (cause) {
            if (
                cause instanceof Error &&
                (cause.name === "TimeoutError" || cause.name === "AbortError")
            ) {
                const seconds = STOP_TIMEOUT_MILLISECONDS / 1000;
                throw MachineError.of(
                    "STOP_BROKEN",
                    `machine did not stop within ${seconds} seconds`,
                    { cause },
                );
            }
            throw cause;
        }
    }

    /** Write the endpoint file atomically, removing it once the cleanup closes. */
    async publish(cleanup: AsyncDisposableStack, published: LoopbackEndpoint): Promise<void> {
        // replace this process's file through a temporary file
        const { files } = this.environment;
        const endpoint = join(this.directory, ENDPOINT_FILE);
        const temporary = `${endpoint}.${this.environment.process.id}`;
        await files.writeTextFile(temporary, JSON.stringify(published), {
            mode: ENDPOINT_MODE,
            createNew: true,
        });
        cleanup.defer(() => files.remove(temporary, { force: true }));
        await files.rename(temporary, endpoint);
        cleanup.defer(() => files.remove(endpoint));

        // notify a starting client listening at the ready socket, which none may be
        const ready = join(this.directory, READY_SOCKET);
        if (await files.exists(ready)) {
            const socket = this.environment.network.connect({ unix: ready });
            await socket.opened;
            await socket.close();
        }
    }

    /** Launch one detached process while its caller has the startup lock. */
    async #launch(entry: string | undefined, executable: string): Promise<void> {
        // pass the system values and the configuration on, and no credentials
        const { process } = this.environment;
        const passed = ENVIRONMENT.flatMap((name) => {
            const value = process.variables[name];

            return value === undefined ? [] : [[name, value] as const];
        });
        const variables = { DESTACK_DIRECTORY: this.directory, ...Object.fromEntries(passed) };
        const runArguments =
            entry === undefined
                ? Release.arguments("machine")
                : [
                      "run",
                      "--preload",
                      fileURLToPath(import.meta.resolve("@destack/package/bun/preload")),
                      "--no-env-file",
                      fileURLToPath(entry),
                  ];

        // start the machine, stopping it when its start fails
        const log = { file: join(this.directory, LOG_FILE) };
        let child: ReturnType<typeof process.spawn> | undefined;
        try {
            await this.#started(LAUNCH_TIMEOUT_MILLISECONDS, async () => {
                // spawn it writing its output to its log, independently of the launching terminal
                const spawned = process.spawn([executable, ...runArguments], {
                    isDetached: true,
                    stdin: "ignore",
                    stdout: log,
                    stderr: log,
                    variables,
                });
                child = spawned;

                // end the wait once it exits
                const exited = new AbortController();
                void spawned.exited.then(
                    () => exited.abort(),
                    (error: unknown) => exited.abort(error),
                );

                return exited.signal;
            });
        } catch (error) {
            child?.kill("SIGTERM");
            throw error;
        }
    }

    /** Start a machine and wait for its ready notification, failing once it exits or times out. */
    async #started(timeout: number, start: () => Promise<AbortSignal | undefined>): Promise<void> {
        // listen at the ready socket before starting the machine, which notifies it once it serves
        const { files, server, clock } = this.environment;
        const path = join(this.directory, READY_SOCKET);
        await files.remove(path, { force: true });
        const ready = Promise.withResolvers<void>();
        const listener = await server.socket({
            unix: path,
            accept: (socket) => {
                ready.resolve();
                void socket.close();
            },
        });

        // start it, then wait for its notification, its exit or the timeout
        const exited = (await start()) ?? new AbortController().signal;
        const stopping = AbortSignal.any([clock.timeout(timeout), exited]);
        try {
            await Promise.race([ready.promise, waitAborted(stopping)]);
            if (!stopping.aborted) {
                return;
            }
        } finally {
            await listener.stop();
            await files.remove(path, { force: true });
        }

        // read the last line the machine logged
        const file = join(this.directory, LOG_FILE);
        const log = (await files.exists(file)) ? await files.readTextFile(file) : "";
        const last = log.trimEnd().split("\n").at(-1) ?? "";

        // report a machine that exited or did not answer in time
        throw MachineError.of(
            "START_BROKEN",
            exited.aborted
                ? `machine exited while starting: its log '${file}' ends with '${last}'`
                : `machine did not answer within ${timeout} ms: its log '${file}' ends with '${last}'`,
            { cause: stopping.reason },
        );
    }

    /** Report whether the machine answers, failing on errors other than a stopped machine. */
    async #answers(): Promise<boolean> {
        try {
            await this.loopback.computer();

            return true;
        } catch (error) {
            if (!MachineProcess.isUnavailable(error)) {
                throw error;
            }

            return false;
        }
    }
}
