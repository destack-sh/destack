import type { Clock } from "../clock/index.ts";
import { BunProcess } from "./bun.ts";
import { ProcessError } from "./error.ts";
import type {
    Input,
    Output,
    Process,
    ProcessChannel,
    ProcessUsage,
    Runtime,
    Signal,
    SpawnOptions,
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
    /** The clock a child's whole output counts on as host work. */
    readonly #clock: Clock;
    /** The ids of the children the node started that have not exited. */
    readonly #children = new Set<number>();

    /** Give a node its process from given values, its children's output counted on a clock. */
    constructor(options: ProcessOptions, clock: Clock) {
        // store the given values
        this.arguments = options.arguments;
        this.variables = options.variables;
        this.workingDirectory = options.workingDirectory;
        this.id = options.id;
        this.userId = options.userId;
        this.executable = options.executable;
        this.runtime = options.runtime;
        this.parent = options.parent;
        this.#clock = clock;
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
        // deliver a signal to the node's own listeners
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

    /** Start a real child with the node's variables, counting its whole output as host work. */
    spawn(command: readonly string[], options: SpawnOptions = {}): Subprocess {
        return this.#adopt(this.#host.spawn(command, this.#options(options)));
    }

    /** Start a command connected to this process by a channel. */
    fork(
        command: readonly string[],
        options: SpawnOptions = {},
    ): Subprocess & { readonly channel: ProcessChannel } {
        return this.#adopt(this.#host.fork(command, this.#options(options)));
    }

    /** Count a child as the node's until it exits, and its whole output as host work. */
    #adopt<Child extends Subprocess>(child: Child): Child {
        // know the child's id while it runs
        this.#children.add(child.id);
        void child.exited.finally(() => this.#children.delete(child.id));

        return {
            ...child,
            get signal() {
                return child.signal;
            },
            output: () => this.#clock.trackHost(child.output()),
        };
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
