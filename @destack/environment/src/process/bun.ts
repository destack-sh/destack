import { ProcessError } from "./error.ts";
import { closeSync, openSync, writeSync } from "node:fs";
import type { BunPlugin, FileSink } from "bun";
import {
    type Input,
    type Output,
    type Process,
    type ProcessChannel,
    type ProcessUsage,
    type Runtime,
    type Signal,
    type SpawnOptions,
    type StandardStream,
    Subprocess,
} from "./process.ts";

/** The file descriptor of standard output, as POSIX fixes it. */
const STDOUT = 1;

/** The file descriptor of standard error, as POSIX fixes it. */
const STDERR = 2;

/** The mode of a file a child's stream creates: its owner reads and writes it. */
const PRIVATE_FILE_MODE = 0o600;

/** The microseconds of a second, the unit `process.cpuUsage` reports in. */
const MICROSECONDS_PER_SECOND = 1_000_000;

/** The process events reporting a failure nothing caught. */
type FailureEvent = "uncaughtException" | "unhandledRejection";

/** The process Bun runs, through Node's `process` and `Bun.spawn`. */
export class BunProcess implements Process {
    /** The standard output. */
    readonly stdout: Output = BunProcess.#output(process.stdout, STDOUT);
    /** The standard error. */
    readonly stderr: Output = BunProcess.#output(process.stderr, STDERR);
    /** The standard input, once read. */
    #stdin: Input | undefined;
    /** The channel to the parent, once read. */
    #parent: ProcessChannel | undefined;
    /** The runtime, read on first use. */
    #runtime: (Runtime & { readonly plugin: (plugin: BunPlugin) => Promise<void> }) | undefined;

    /** The standard input, read from its first access on. */
    get stdin(): Input {
        this.#stdin ??= { readable: Bun.stdin.stream(), isTerminal: process.stdin.isTTY };

        return this.#stdin;
    }

    /** The arguments after the executable and the script. */
    get arguments(): readonly string[] {
        return process.argv.slice(2);
    }

    /** The environment variables. */
    get variables(): Readonly<Record<string, string>> {
        return Object.fromEntries(
            Object.entries(process.env).flatMap(([name, value]) =>
                value === undefined ? [] : [[name, value]],
            ),
        );
    }

    /** The working directory. */
    get workingDirectory(): string {
        return process.cwd();
    }

    /** The process identifier. */
    get id(): number {
        return process.pid;
    }

    /** The user the process runs as, absent on Windows. */
    get userId(): number | undefined {
        return process.getuid?.();
    }

    /** The path of the running executable. */
    get executable(): string {
        return process.execPath;
    }

    /** The runtime running the process, which also takes module loader plugins. */
    get runtime(): Runtime & { readonly plugin: (plugin: BunPlugin) => Promise<void> } {
        // read Bun's build once, on first use, so a Node process can load this module
        this.#runtime ??= {
            revision: Bun.revision,
            isStandalone: Bun.isStandaloneExecutable,
            plugin: async (plugin) => {
                await Bun.plugin(plugin);
            },
        };

        return this.#runtime;
    }

    /** Find a command on the process's search path, absent when none. */
    which(command: string): string | undefined {
        return BunProcess.which(command, this.variables["PATH"]);
    }

    /** Report whether the process's loader finds a shared library, loading the FFI on use. */
    async hasLibrary(name: string): Promise<boolean> {
        // open and close the library, a library the loader cannot load counting as absent
        const { dlopen } = await import("bun:ffi");
        try {
            dlopen(name, {}).close();

            return true;
        } catch {
            return false;
        }
    }

    /** Find a command on a search path, absent when none, for every process that searches one. */
    static which(command: string, path: string | undefined): string | undefined {
        return Bun.which(command, path === undefined ? {} : { PATH: path }) ?? undefined;
    }

    /** The channel to the parent process, absent when no parent connected it. */
    get parent(): ProcessChannel | undefined {
        // read the channel Node's process opens for a connected child
        if (process.send === undefined) {
            return undefined;
        }

        // listen once for the close of the channel
        this.#parent ??= {
            send: (message) => {
                process.send?.(message);
            },
            on: (listener) => {
                process.on("message", listener);

                return { [Symbol.dispose]: () => process.off("message", listener) };
            },
            disconnect: () => {
                process.disconnect?.();
            },
            disconnected: new Promise((resolve) => {
                process.once("disconnect", () => resolve());
            }),
        };

        return this.#parent;
    }

    /** Send a signal to a process by its id, or to a process group by its negated id. */
    kill(id: number, signal: Signal = "SIGTERM"): void {
        try {
            process.kill(id, signal);
        } catch (error) {
            throw BunProcess.refusal(error, `signal ${id}`);
        }
    }

    /** Call a listener on each failure nothing caught, ending the process once it settles. */
    onFailure(listener: (error: unknown) => Promise<void> | void): Disposable {
        // run the listener, then raise the failure again when no other listener handles it
        const fail = (event: FailureEvent, error: unknown, raise: () => void) => {
            const isFatal = process.listenerCount(event) === 1;
            void Promise.resolve(listener(error)).finally(() => {
                if (isFatal) {
                    remove();
                    raise();
                }
            });
        };

        // raise each failure as the process raises it by default
        const onException = (error: Error) =>
            fail("uncaughtException", error, () =>
                setTimeout(() => {
                    throw error;
                }),
            );
        const onRejection = (reason: unknown) =>
            fail("unhandledRejection", reason, () => void Promise.reject(reason));
        const remove = () => {
            process.off("uncaughtException", onException);
            process.off("unhandledRejection", onRejection);
        };
        process.on("uncaughtException", onException);
        process.on("unhandledRejection", onRejection);

        return { [Symbol.dispose]: remove };
    }

    /** Read the CPU time the process used so far, in seconds, by mode. */
    usage(): ProcessUsage {
        const { user, system } = process.cpuUsage();

        return { user: user / MICROSECONDS_PER_SECOND, system: system / MICROSECONDS_PER_SECOND };
    }

    /** Call a listener on each signal until the subscription disposes. */
    on(signal: Signal, listener: () => void): Disposable {
        process.on(signal, listener);

        return { [Symbol.dispose]: () => process.off(signal, listener) };
    }

    /** The exit code the process ends with once its work settles, absent for zero. */
    get exitCode(): number | undefined {
        const { exitCode } = process;

        return typeof exitCode === "number" ? exitCode : undefined;
    }

    /** End the process with an exit code once its work settles. */
    set exitCode(code: number | undefined) {
        process.exitCode = code;
    }

    /** End the process with an exit code, zero by default. */
    exit(code = 0): never {
        process.exit(code);
    }

    /** Start a child process from a command and its arguments. */
    spawn(command: readonly string[], options: SpawnOptions = {}): Subprocess {
        return BunProcess.#subprocess(BunProcess.#start(command, options, {}));
    }

    /** Start a command connected to this process by a channel. */
    fork(
        command: readonly string[],
        options: SpawnOptions = {},
    ): Subprocess & { readonly channel: ProcessChannel } {
        // start the child, passing its messages to the channel's listeners
        const listeners = new Set<(message: unknown) => void>();
        const { promise: disconnected, resolve: disconnect } = Promise.withResolvers<void>();
        const child = BunProcess.#start(command, options, {
            ipc: (message) => {
                for (const listener of listeners) {
                    listener(message);
                }
            },
            onDisconnect: () => disconnect(),
        });

        // settle the channel once the child exits
        void child.exited.then(() => disconnect());
        const channel: ProcessChannel = {
            send: (message) => child.send(message),
            on: (listener) => {
                listeners.add(listener);

                return { [Symbol.dispose]: () => listeners.delete(listener) };
            },
            disconnect: () => child.disconnect(),
            disconnected,
        };

        return { ...BunProcess.#subprocess(child), channel };
    }

    /** Start a child with its streams, killing it once the signal aborts. */
    static #start(
        command: readonly string[],
        options: SpawnOptions,
        connection: {
            readonly ipc?: (message: unknown) => void;
            readonly onDisconnect?: () => void;
        },
    ): Bun.Subprocess {
        // open the files the child's streams read and write
        const isDetached = options.isDetached === true;
        const opened = new Map<string, number>();
        const stdio = (stream: StandardStream, flags: "r" | "a") =>
            typeof stream === "string" ? stream : BunProcess.#open(stream.file, flags, opened);
        const child = Bun.spawn([...command], {
            ...(options.workingDirectory === undefined ? {} : { cwd: options.workingDirectory }),
            ...(options.variables === undefined ? {} : { env: { ...options.variables } }),
            ...(options.signal === undefined ? {} : { signal: options.signal }),
            ...(isDetached ? { detached: true } : {}),
            ...connection,
            stdin: stdio(options.stdin ?? "ignore", "r"),
            stdout: stdio(options.stdout ?? (isDetached ? "ignore" : "pipe"), "a"),
            stderr: stdio(options.stderr ?? (isDetached ? "ignore" : "inherit"), "a"),
        });

        // close the parent's copies of the files the child writes, and stop waiting for a detached child
        for (const descriptor of opened.values()) {
            closeSync(descriptor);
        }
        if (isDetached) {
            child.unref();
        }

        return child;
    }

    /** Read a Bun subprocess as a subprocess. */
    static #subprocess(child: Bun.Subprocess): Subprocess {
        // read its streams, and its output through them
        const stdout = BunProcess.#readable(child.stdout);
        const stderr = BunProcess.#readable(child.stderr);

        // answer it as a subprocess
        const subprocess: Subprocess = {
            id: child.pid,
            stdin: BunProcess.#sink(child.stdin),
            stdout,
            stderr,
            exited: child.exited,
            get signal() {
                return child.signalCode ?? undefined;
            },
            kill: (signal) => child.kill(signal),
            output: () => Subprocess.output(subprocess),
        };

        return subprocess;
    }

    /** Open a file a child's stream reads or appends to, once per spawn and mode. */
    static #open(path: string, flags: "r" | "a", opened: Map<string, number>): number {
        // reuse the descriptor both outputs of one file share
        const key = `${flags} ${path}`;
        const known = opened.get(key);
        if (known !== undefined) {
            return known;
        }

        // open it, created private when missing
        const descriptor = openSync(path, flags, PRIVATE_FILE_MODE);
        opened.set(key, descriptor);

        return descriptor;
    }

    /** Read a subprocess's output stream, empty for one it does not pipe. */
    static #readable(stream: ReadableStream<Uint8Array> | number | undefined | null) {
        return stream instanceof ReadableStream
            ? stream
            : new ReadableStream<Uint8Array>({ start: (controller) => controller.close() });
    }

    /** Read a standard stream of Node's process, at its file descriptor, as an output. */
    static #output(stream: NodeJS.WriteStream, descriptor: number): Output {
        return {
            write: (chunk) =>
                new Promise((resolve, reject) => {
                    stream.write(chunk, (error) => (error ? reject(error) : resolve()));
                }),
            writeSync: (chunk) => {
                writeSync(descriptor, typeof chunk === "string" ? Buffer.from(chunk) : chunk);
            },
            isTerminal: stream.isTTY,
        };
    }

    /** Read a subprocess's input sink as a writable, refusing writes to an unpiped input. */
    static #sink(sink: FileSink | number | undefined | null): WritableStream<Uint8Array> {
        return new WritableStream<Uint8Array>({
            write: async (chunk) => {
                if (typeof sink !== "object" || sink === null) {
                    throw new TypeError("the subprocess's standard input is not piped");
                }
                await sink.write(chunk);
                await sink.flush();
            },
            close: async () => {
                if (typeof sink === "object" && sink !== null) {
                    await sink.end();
                }
            },
        });
    }

    /** Read a host failure of a signal as the processes' failure. */
    static refusal(error: unknown, action: string): unknown {
        // pass on a failure without an error code
        if (!(error instanceof Error) || !("code" in error) || typeof error.code !== "string") {
            return error;
        }

        // refuse by the error code, keeping the host's failure as the cause
        const message = `cannot ${action}: ${error.code}`;
        if (error.code === "ESRCH") {
            return ProcessError.of("NOT_FOUND", message, { cause: error });
        } else if (error.code === "EPERM") {
            return ProcessError.of("PERMISSION_DENIED", message, { cause: error });
        } else {
            return ProcessError.of("PROCESS_FAILED", message, { cause: error });
        }
    }
}
