import type { Scheduler } from "../scheduler/scheduler.ts";
import { BunProcess } from "./bun.ts";
import { ProcessError } from "./error.ts";
import {
    type Input,
    type Output,
    type Process,
    type ProcessChannel,
    type ProcessUsage,
    type Runtime,
    type Signal,
    type SpawnOptions,
    Subprocess,
} from "./process.ts";

/** The process a simulation gives a node: its arguments, variables and working directory. */
export interface ProcessOptions {
    /** The arguments after the executable and the script. */
    readonly arguments: readonly string[];
    /** The environment variables. */
    readonly variables: Readonly<Record<string, string>>;
    /** The working directory, as the node's files see it. */
    readonly workingDirectory: string;
    /** The process identifier. */
    readonly id: number;
    /** The user the process runs as, absent on Windows. */
    readonly userId: number | undefined;
    /** The path of the running executable. */
    readonly executable: string;
    /** The runtime running the process. */
    readonly runtime: Runtime;
    /** The channel to the parent process, absent when no parent connected it. */
    readonly parent?: ProcessChannel;
}

/** A node's process: given values, written output, raised signals and real children. */
export class SimulatedProcess implements Process {
    /** The arguments after the executable and the script. */
    readonly arguments: readonly string[];
    /** The environment variables. */
    readonly variables: Readonly<Record<string, string>>;
    /** The working directory. */
    readonly workingDirectory: string;
    /** The process identifier. */
    readonly id: number;
    /** The user the process runs as, absent on Windows. */
    readonly userId: number | undefined;
    /** The path of the running executable. */
    readonly executable: string;
    /** The runtime running the process. */
    readonly runtime: Runtime;
    /** The channel to the parent process, absent when no parent connected it. */
    readonly parent: ProcessChannel | undefined;
    /** The standard input, empty. */
    readonly stdin: Input = {
        readable: new ReadableStream<Uint8Array>({ start: (controller) => controller.close() }),
        isTerminal: false,
    };
    /** The standard output, written into its chunks. */
    readonly stdout: Output & { readonly chunks: Uint8Array[] } = SimulatedProcess.#output();
    /** The standard error, written into its chunks. */
    readonly stderr: Output & { readonly chunks: Uint8Array[] } = SimulatedProcess.#output();
    /** The exit code the process ended with, absent while it runs. */
    exitCode: number | undefined;
    /** The listeners of each signal. */
    readonly #listeners = new Map<Signal, Set<() => void>>();
    /** The listeners of failures nothing caught. */
    readonly #failures = new Set<(error: unknown) => Promise<void> | void>();
    /** The host's process, which starts the children. */
    readonly #host = new BunProcess();
    /** The scheduler each child's lifetime counts on. */
    readonly #scheduler: Scheduler;
    /** The ids of the children the node started that have not exited. */
    readonly #children = new Set<number>();

    /** Give a node its process from given values, its children counted on a scheduler. */
    constructor(options: ProcessOptions, scheduler: Scheduler) {
        // store the given values
        this.arguments = options.arguments;
        this.variables = options.variables;
        this.workingDirectory = options.workingDirectory;
        this.id = options.id;
        this.userId = options.userId;
        this.executable = options.executable;
        this.runtime = options.runtime;
        this.parent = options.parent;
        this.#scheduler = scheduler;
    }

    /** Find a command on the process's search path, on the host's files. */
    which(command: string): string | undefined {
        return BunProcess.which(command, this.variables["PATH"]);
    }

    /** Report whether the host's loader finds a shared library. */
    hasLibrary(name: string): Promise<boolean> {
        return this.#host.hasLibrary(name);
    }

    /** Call a listener on each signal until the subscription disposes. */
    on(signal: Signal, listener: () => void): Disposable {
        // add the listener to the signal's
        const listeners = this.#listeners.get(signal) ?? new Set();
        listeners.add(listener);
        this.#listeners.set(signal, listeners);

        return { [Symbol.dispose]: () => listeners.delete(listener) };
    }

    /** Send a signal to the node, a child it started, or that child's group by its negated id. */
    kill(id: number, signal: Signal = "SIGTERM"): void {
        // deliver a signal to the node's listeners
        if (id === this.id) {
            this.raise(signal);
        }
        // signal a real child, refusing any other host process
        else if (this.#children.has(Math.abs(id))) {
            this.#host.kill(id, signal);
        } else {
            throw ProcessError.of("NOT_FOUND", `process ${id} is not this node or a running child`);
        }
    }

    /** Call a listener on each failure the node raises, leaving the host's failures to the host. */
    onFailure(listener: (error: unknown) => Promise<void> | void): Disposable {
        this.#failures.add(listener);

        return { [Symbol.dispose]: () => this.#failures.delete(listener) };
    }

    /** Deliver a failure nothing caught to its listeners, settling once they did. */
    async fail(error: unknown): Promise<void> {
        await Promise.all([...this.#failures].map(async (listener) => await listener(error)));
    }

    /** Read the CPU time the node used, none, as its work runs in the host's process. */
    usage(): ProcessUsage {
        return { user: 0, system: 0 };
    }

    /** Deliver a signal to its listeners, as the operating system does. */
    raise(signal: Signal): void {
        for (const listener of this.#listeners.get(signal) ?? []) {
            listener();
        }
    }

    /** End the process with an exit code, failing the work that called it. */
    exit(code = 0): never {
        this.exitCode = code;

        throw new Error(`the process exited with code ${code}`);
    }

    /** Kill every child the node started, as their parent's end does. */
    release(): void {
        for (const id of this.#children) {
            this.#host.kill(id, "SIGKILL");
        }
    }

    /** Start a real child with the node's variables, counting its lifetime as its option says. */
    spawn(command: readonly string[], options: SpawnOptions = {}): Subprocess {
        return this.#adopt(
            this.#host.spawn(command, this.#options(options)),
            SimulatedProcess.#tracking(options),
        );
    }

    /** Start a command connected to this process by a channel. */
    fork(
        command: readonly string[],
        options: SpawnOptions = {},
    ): Subprocess & { readonly channel: ProcessChannel } {
        return this.#adopt(
            this.#host.fork(command, this.#options(options)),
            SimulatedProcess.#tracking(options),
        );
    }

    /** Count a child as the node's until it exits, tracking its exit and streams as a peer's. */
    #adopt<Child extends Subprocess>(child: Child, tracking: "peer" | "none"): Child {
        // know the child's id while it runs
        this.#children.add(child.id);
        void child.exited.finally(() => this.#children.delete(child.id));

        // leave a service untracked, as time moves while it runs
        if (tracking === "none") {
            return child;
        }

        // track its exit, each read of its output and each write to its input
        const exited = this.#scheduler.track(child.exited, "peer");
        const stdout = this.#readable(child.stdout);
        const stderr = this.#readable(child.stderr);
        const stdin = this.#writable(child.stdin);

        // answer it with the tracked streams and exit, its output read through them
        const adopted: Child = {
            ...child,
            stdin,
            stdout,
            stderr,
            exited,
            get signal() {
                return child.signal;
            },
            output: () => Subprocess.output(adopted),
        };

        return adopted;
    }

    /** Read how a child is tracked: as a peer by default, and none when detached. */
    static #tracking(options: SpawnOptions): "peer" | "none" {
        return options.tracking ?? (options.isDetached === true ? "none" : "peer");
    }

    /** Read a child's stream on demand, each read tracked as a peer's. */
    #readable(stream: ReadableStream<Uint8Array>): ReadableStream<Uint8Array> {
        const reader = stream.getReader();

        return new ReadableStream<Uint8Array>(
            {
                pull: async (controller) => {
                    // pass each chunk on, closing at the end
                    const { done, value } = await this.#scheduler.track(reader.read(), "peer");
                    if (done) {
                        controller.close();
                    } else {
                        controller.enqueue(value);
                    }
                },
                cancel: (reason) => reader.cancel(reason),
            },
            { highWaterMark: 0 },
        );
    }

    /** Write a child's input, each write tracked as a peer's. */
    #writable(stream: WritableStream<Uint8Array>): WritableStream<Uint8Array> {
        const writer = stream.getWriter();

        return new WritableStream<Uint8Array>({
            write: (chunk) => this.#scheduler.track(writer.write(chunk), "peer"),
            close: () => this.#scheduler.track(writer.close(), "peer"),
            abort: (reason) => this.#scheduler.track(writer.abort(reason), "peer"),
        });
    }

    /** Default a child's working directory and variables to the process's. */
    #options(options: SpawnOptions): SpawnOptions {
        return {
            ...options,
            workingDirectory: options.workingDirectory ?? this.workingDirectory,
            variables: options.variables ?? this.variables,
        };
    }

    /** Make an output that stores what is written to it. */
    static #output(): Output & { readonly chunks: Uint8Array[] } {
        // store a copy of each chunk written, at once
        const chunks: Uint8Array[] = [];
        const encoder = new TextEncoder();
        const writeSync = (chunk: string | Uint8Array) => {
            chunks.push(typeof chunk === "string" ? encoder.encode(chunk) : chunk.slice());
        };

        return {
            write: async (chunk) => writeSync(chunk),
            writeSync,
            isTerminal: false,
            chunks,
        };
    }
}
