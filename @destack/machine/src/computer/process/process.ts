import { TelemetryScope } from "@destack/telemetry";
import { Bookmark } from "@destack/object/client";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import type { CommandHost } from "./command.ts";
import { Release } from "@destack/update/release";
import { StageHome } from "./home.ts";
import { type LoopbackEndpoint, ENDPOINT_FILE, LOCK_FILE, Loopback } from "./endpoint.ts";
import { MachineError } from "../../error/index.ts";
import { Autostart } from "../autostart/index.ts";

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
    "DESTACK_STAGE",
    "DESTACK_UNIVERSE",
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

/** How long a machine the OS supervisor starts may take to answer: a launch plus scheduling. */
const SUPERVISOR_TIMEOUT_MILLISECONDS = 10000;

/** The machine process of a state directory: its launch, its loopback and its stop. */
export class MachineProcess {
    /** The machine's private state directory. */
    readonly directory: string;
    /** The host starting and stopping the machine. */
    readonly environment: CommandHost;
    /** The owner's loopback connection to the running machine. */
    readonly loopback: Loopback;

    /** The telemetry of the launches and shutdowns. */
    readonly #scope: TelemetryScope;

    /** Address the machine process of a state directory. */
    constructor(directory: string, environment: CommandHost) {
        // take the directory and the environment, with the telemetry and the loopback on them
        this.directory = directory;
        this.environment = environment;
        this.#scope = TelemetryScope.of(import.meta.destack.package, environment.telemetry);
        this.loopback = new Loopback(directory, environment, new Bookmark());
    }

    /** Address the machine process of the current release home. */
    static current(environment: CommandHost): MachineProcess {
        return new MachineProcess(StageHome.current(environment).directory, environment);
    }

    /** Locate the machine executable beside this standalone executable, else this runtime. */
    static executable(environment: CommandHost): string {
        const { executable } = environment.process;

        return environment.process.runtime.isStandalone
            ? join(dirname(executable), Release.executable("destack-machine", environment.host.os))
            : executable;
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
        executable = MachineProcess.executable(this.environment),
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
                await autostart.start();
                await this.#started(SUPERVISOR_TIMEOUT_MILLISECONDS);
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
                ? []
                : [
                      "run",
                      "--preload",
                      fileURLToPath(import.meta.resolve("@destack/package/bun/preload")),
                      "--no-env-file",
                      fileURLToPath(entry),
                  ];

        // write the machine's output to its log, independently of the launching terminal
        const log = { file: join(this.directory, "machine.log") };
        const child = process.spawn([executable, ...runArguments], {
            isDetached: true,
            stdin: "ignore",
            stdout: log,
            stderr: log,
            variables,
        });

        // bound startup while the child publishes its endpoint
        const exited = new AbortController();
        void child.exited.then(
            () => exited.abort(),
            (error: unknown) => exited.abort(error),
        );
        try {
            await this.#started(LAUNCH_TIMEOUT_MILLISECONDS, exited.signal);
        } catch (error) {
            child.kill("SIGTERM");
            throw error;
        }
    }

    /** Wait until a starting machine answers, failing once it exits or times out. */
    async #started(
        timeout: number,
        exited: AbortSignal = new AbortController().signal,
    ): Promise<void> {
        // watch for the endpoint before asking a machine that may have published it already
        const stopping = AbortSignal.any([this.environment.clock.timeout(timeout), exited]);
        const changes = await this.environment.files.watch(this.directory, { signal: stopping });
        try {
            if (await this.#answers()) {
                return;
            }

            // ask again each time the endpoint is published
            for await (const change of changes) {
                if (change.path === ENDPOINT_FILE && (await this.#answers())) {
                    return;
                }
            }
        } catch (error) {
            if (!stopping.aborted) {
                throw error;
            }
        }

        // report a machine that exited or did not answer in time
        throw MachineError.of(
            "START_BROKEN",
            `machine did not start: read '${this.directory}'/machine.log`,
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
