import type { Environment, Subprocess } from "@destack/environment";

/** How long a child may take to drain and exit after a termination, in milliseconds. */
const STOP_MILLISECONDS = 15_000;

/** What a child of the dev environment runs. */
export interface ProcessOptions {
    /** The command and its arguments. */
    readonly command: readonly string[];
    /** The working directory. */
    readonly directory: string;
    /** The child's whole set of environment variables. */
    readonly variables: Readonly<Record<string, string>>;
    /** Print a line the child wrote, under its name. */
    readonly print: (name: string, line: string) => void;
}

/** A child process of the dev environment. */
export class LocalProcess implements AsyncDisposable {
    /** The name its lines print under, such as machine. */
    readonly name: string;
    /** The child. */
    readonly #child: Subprocess;
    /** The printing of its output, done once both streams end. */
    readonly #printed: Promise<unknown>;
    /** The clock the stop timeout runs on. */
    readonly #environment: Environment;
    /** Whether the child exited. */
    #isExited = false;

    /** Keep a started child and the printing of its output. */
    private constructor(
        name: string,
        child: Subprocess,
        printed: Promise<unknown>,
        environment: Environment,
    ) {
        // keep the child, and note its exit
        this.name = name;
        this.#child = child;
        this.#printed = printed;
        this.#environment = environment;
        void child.exited.then(() => {
            this.#isExited = true;
        });
    }

    /** Start a child, printing each line it writes under its name. */
    static start(
        name: string,
        options: ProcessOptions,
        environment: Environment<"process">,
    ): LocalProcess {
        // spawn the child, printing both its outputs line by line
        const child = environment.process.spawn(options.command, {
            workingDirectory: options.directory,
            variables: options.variables,
            stdin: "ignore",
            stdout: "pipe",
            stderr: "pipe",
        });
        const print = (line: string) => options.print(name, line);
        const printed = Promise.all([lines(child.stdout, print), lines(child.stderr, print)]);

        return new LocalProcess(name, child, printed, environment);
    }

    /** The exit status, once the child exits. */
    get exited(): Promise<number> {
        return this.#child.exited;
    }

    /** Ask the child to stop, and end it once it outlives the stop timeout. */
    async [Symbol.asyncDispose](): Promise<void> {
        // terminate, and kill a child that does not exit in time
        if (!this.#isExited) {
            const { clock } = this.#environment;
            this.#child.kill("SIGTERM");
            const timeout = clock.setTimeout(() => this.#child.kill("SIGKILL"), STOP_MILLISECONDS);
            await this.#child.exited;
            clock.clearTimeout(timeout);
        }
        await this.#printed;
    }
}

/** Hand on each line a stream carries, the last one without its line break too. */
export async function lines(
    stream: ReadableStream<Uint8Array>,
    print: (line: string) => void,
): Promise<void> {
    // hand on each complete line, keeping the rest for the next chunk
    const decoder = new TextDecoder();
    let pending = "";
    for await (const chunk of stream) {
        const split = `${pending}${decoder.decode(chunk, { stream: true })}`.split("\n");
        pending = split.pop() ?? "";
        for (const line of split) {
            print(line);
        }
    }

    // hand on the last line
    if (pending !== "") {
        print(pending);
    }
}
