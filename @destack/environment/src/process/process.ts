/** The POSIX signals a process sends, and listens for all but SIGKILL. */
export type Signal =
    | "SIGHUP"
    | "SIGINT"
    | "SIGQUIT"
    | "SIGKILL"
    | "SIGTERM"
    | "SIGUSR1"
    | "SIGUSR2"
    | "SIGWINCH";

/** Where a subprocess's standard stream goes: a pipe, the parent's own, nowhere, or a file. */
export type StandardStream =
    | "pipe"
    | "inherit"
    | "ignore"
    | {
          /** The file it reads, or appends to, created with mode 0600 when missing. */
          readonly file: string;
      };

/** A standard stream a process reads. */
export interface Input {
    /** The bytes it reads. */
    readonly readable: ReadableStream<Uint8Array>;
    /** Whether a terminal is attached. */
    readonly isTerminal: boolean;
}

/** A standard stream a process writes. */
export interface Output {
    /** Write text as UTF-8, or bytes, settling once the stream took them. */
    readonly write: (chunk: string | Uint8Array) => Promise<void>;
    /** Write text as UTF-8, or bytes, at once, throwing on a failed write. */
    readonly writeSync: (chunk: string | Uint8Array) => void;
    /** Whether a terminal is attached. */
    readonly isTerminal: boolean;
}

/** The runtime running a process, as Bun describes itself. */
export interface Runtime {
    /** The Git revision of the runtime's build, as `Bun.revision` has it. */
    readonly revision: string;
    /** Whether the executable is a standalone program, as `Bun.isStandaloneExecutable` says. */
    readonly isStandalone: boolean;
}

/** How a subprocess starts. */
export interface SpawnOptions {
    /** The working directory, the parent's by default. */
    readonly workingDirectory?: string;
    /** The environment variables, the parent's by default. */
    readonly variables?: Readonly<Record<string, string>>;
    /** Where its standard input comes from, nowhere by default. */
    readonly stdin?: StandardStream;
    /** Where its standard output goes, a pipe by default. */
    readonly stdout?: StandardStream;
    /** Where its standard error goes, the parent's by default. */
    readonly stderr?: StandardStream;
    /** Kill the subprocess once it aborts. */
    readonly signal?: AbortSignal;
    /** Run it in its own process group, unwaited, its streams ignored by default. */
    readonly isDetached?: boolean;
    /** How a world tracks it: a peer whose life stops time, or none for a service. */
    readonly tracking?: "peer" | "none";
}

/** The CPU time a process used, in seconds, by mode. */
export interface ProcessUsage {
    /** The time spent running the process's own code. */
    readonly user: number;
    /** The time the system spent on the process's behalf. */
    readonly system: number;
}

/** A message channel between a process and its parent, alike on both ends. */
export interface ProcessChannel {
    /** Send a message to the other end. */
    readonly send: (message: unknown) => void;
    /** Call a listener on each message from the other end until the subscription disposes. */
    readonly on: (listener: (message: unknown) => void) => Disposable;
    /** Close the channel. */
    readonly disconnect: () => void;
    /** Settles once the channel closed, from either end or as the child exited. */
    readonly disconnected: Promise<void>;
}

/** A running child process, as Bun's `Subprocess` has it. */
export interface Subprocess {
    /** The process identifier. */
    readonly id: number;
    /** The bytes sent to its standard input, open when piped. */
    readonly stdin: WritableStream<Uint8Array>;
    /** The bytes of its standard output, empty unless piped. */
    readonly stdout: ReadableStream<Uint8Array>;
    /** The bytes of its standard error, empty unless piped. */
    readonly stderr: ReadableStream<Uint8Array>;
    /** Settles with the exit code once it exited. */
    readonly exited: Promise<number>;
    /** The name of the signal that ended it, absent while it runs or after it exited itself. */
    readonly signal: string | undefined;
    /** Send it a signal, SIGTERM by default. */
    readonly kill: (signal?: Signal) => void;
    /** Read its whole output once it exited, as Deno's `ChildProcess.output` does. */
    readonly output: () => Promise<SubprocessOutput>;
}

/** What a child process wrote and how it ended, as Deno's `CommandOutput` has it. */
export interface SubprocessOutput {
    /** The exit code. */
    readonly code: number;
    /** The name of the signal that ended it, absent once it exited itself. */
    readonly signal: string | undefined;
    /** The bytes of its standard output, empty unless piped. */
    readonly stdout: Uint8Array;
    /** The bytes of its standard error, empty unless piped. */
    readonly stderr: Uint8Array;
}

/** Subprocesses. */
export const Subprocess = {
    /** Read a child's streams to their end beside its exit. */
    async output(
        child: Pick<Subprocess, "stdout" | "stderr" | "exited" | "signal">,
    ): Promise<SubprocessOutput> {
        const [stdout, stderr, code] = await Promise.all([
            new Response(child.stdout).bytes(),
            new Response(child.stderr).bytes(),
            child.exited,
        ]);

        return { code, signal: child.signal, stdout, stderr };
    },
};

/** The running process: its arguments, variables, standard streams, signals and children. */
export interface Process {
    /** The arguments after the executable and the script. */
    readonly arguments: readonly string[];
    /** The environment variables. */
    readonly variables: Readonly<Record<string, string>>;
    /** The working directory. */
    readonly workingDirectory: string;
    /** The process identifier. */
    readonly id: number;
    /** The user the process runs as, absent on Windows, as `process.getuid` reads it. */
    readonly userId: number | undefined;
    /** The path of the running executable. */
    readonly executable: string;
    /** The runtime running the process. */
    readonly runtime: Runtime;
    /** Find a command on the process's search path, absent when none, as `Bun.which` does. */
    readonly which: (command: string) => string | undefined;
    /** Report whether the process's loader finds a shared library, as `dlopen` does. */
    readonly hasLibrary: (name: string) => Promise<boolean>;
    /** The standard input. */
    readonly stdin: Input;
    /** The standard output. */
    readonly stdout: Output;
    /** The standard error. */
    readonly stderr: Output;
    /** Call a listener on each signal until the subscription disposes. */
    readonly on: (signal: Signal, listener: () => void) => Disposable;
    /** Send a signal to a process by its id, or to a group by its negated id. */
    readonly kill: (id: number, signal?: Signal) => void;
    /** Call a listener on each failure nothing caught, ending the process once it settles. */
    readonly onFailure: (listener: (error: unknown) => Promise<void> | void) => Disposable;
    /** Read the CPU time the process used so far, in seconds, by mode. */
    readonly usage: () => ProcessUsage;
    /** The exit code the process ends with once its work settles, absent until one is set. */
    exitCode: number | undefined;
    /** End the process with an exit code, zero by default. */
    readonly exit: (code?: number) => never;
    /** Start a child process from a command and its arguments. */
    readonly spawn: (command: readonly string[], options?: SpawnOptions) => Subprocess;
    /** Start a command connected to this process by a channel. */
    readonly fork: (
        command: readonly string[],
        options?: SpawnOptions,
    ) => Subprocess & { readonly channel: ProcessChannel };
    /** The channel to the parent process, absent when no parent connected it. */
    readonly parent: ProcessChannel | undefined;
}
