import type { Environment, SubprocessOutput } from "@destack/environment";

/** How long a release tool may run, in milliseconds: twenty minutes, a notarization's wait. */
const TOOL_TIMEOUT_MILLISECONDS = 20 * 60 * 1000;

/** Run a release tool with inherited diagnostics and a bounded lifetime. */
export async function run(
    command: string,
    words: readonly string[],
    environment: Environment<"process">,
    directory = environment.process.workingDirectory,
    variables: Readonly<Record<string, string>> = {},
): Promise<void> {
    // run the tool in the directory with the variables added to this process's
    const child = environment.process.spawn([command, ...words], {
        workingDirectory: directory,
        variables: { ...environment.process.variables, ...variables },
        stdin: "ignore",
        stdout: "inherit",
        stderr: "inherit",
        signal: environment.clock.timeout(TOOL_TIMEOUT_MILLISECONDS),
    });

    // refuse a failed run
    const code = await child.exited;
    if (code !== 0) {
        throw new Error(`${command} exited with status ${code}`);
    }
}

/** Read what a release tool writes, with inherited diagnostics and a bounded lifetime. */
export async function read(
    command: string,
    words: readonly string[],
    environment: Environment<"process">,
    directory = environment.process.workingDirectory,
): Promise<string> {
    // run the tool in the directory, keeping what it writes
    const child = environment.process.spawn([command, ...words], {
        workingDirectory: directory,
        stdin: "ignore",
        stdout: "pipe",
        stderr: "inherit",
        signal: environment.clock.timeout(TOOL_TIMEOUT_MILLISECONDS),
    });
    const [code, output] = await Promise.all([child.exited, new Response(child.stdout).text()]);

    // refuse a failed run
    if (code !== 0) {
        throw new Error(`${command} exited with status ${code}`);
    }

    return output;
}

/** Run a tool through private pipes: write its input, then read its output and how it ended. */
export async function communicate(
    command: string,
    words: readonly string[],
    environment: Environment<"process">,
    options: {
        /** The bytes or text the tool reads, none by default. */
        readonly input?: string | Uint8Array;
        /** The tool's whole set of environment variables, this process's by default. */
        readonly variables?: Readonly<Record<string, string>>;
        /** How long the tool may run, in milliseconds. */
        readonly timeout: number;
    },
): Promise<SubprocessOutput> {
    // start the tool with every stream piped
    const child = environment.process.spawn([command, ...words], {
        ...(options.variables === undefined ? {} : { variables: options.variables }),
        stdin: "pipe",
        stdout: "pipe",
        stderr: "pipe",
        signal: environment.clock.timeout(options.timeout),
    });

    // write the input and close it while reading what the tool writes
    const { input = new Uint8Array() } = options;
    const bytes = typeof input === "string" ? new TextEncoder().encode(input) : input;
    const writer = child.stdin.getWriter();
    const writing = writer.write(bytes).then(() => writer.close());
    const [output] = await Promise.all([child.output(), writing]);

    return output;
}
