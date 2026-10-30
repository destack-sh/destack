import type { Environment } from "@destack/environment";
import { ForgeError } from "../error/index.ts";

/** A git configuration key and its value. */
export type GitSetting = readonly [key: string, value: string];

/** How a git command runs: its configuration, its standard input and the signal aborting it. */
export interface GitOptions {
    /** The configuration git reads beside its defaults, such as credential headers. */
    readonly configuration?: readonly GitSetting[];
    /** The text git reads from its standard input. */
    readonly input?: string;
    /** The signal that stops git. */
    readonly signal?: AbortSignal;
}

/** The null device of each operating system, git's global configuration file. */
const NULL_DEVICE = { darwin: "/dev/null", linux: "/dev/null", win32: "\\\\.\\nul" } as const;

/** The git command line, run without prompts, user configuration or system configuration. */
export const Git = {
    /** Run a git command in a directory, returning its output and failing with git's message. */
    async run(
        directory: string,
        command: readonly [string, ...string[]],
        environment: Environment<"process" | "host">,
        options: GitOptions = {},
    ): Promise<string> {
        // pass the configuration in the environment, so credentials stay out of the process list
        const configuration = options.configuration ?? [];
        const settings = Object.fromEntries(
            configuration.flatMap(([key, value], index) => [
                [`GIT_CONFIG_KEY_${index}`, key],
                [`GIT_CONFIG_VALUE_${index}`, value],
            ]),
        );
        const { variables } = environment.process;
        const host = Object.fromEntries(
            ["PATH", "SystemRoot"].flatMap((name) => {
                const value = variables[name];

                return value === undefined ? [] : [[name, value]];
            }),
        );

        // run git as host work until it exits or the signal aborts
        const child = environment.process.spawn(["git", "-C", directory, ...command], {
            variables: {
                ...host,
                GIT_CONFIG_NOSYSTEM: "1",
                GIT_CONFIG_GLOBAL: NULL_DEVICE[environment.host.os],
                GIT_TERMINAL_PROMPT: "0",
                GIT_CONFIG_COUNT: String(configuration.length),
                ...settings,
            },
            stdin: "pipe",
            stdout: "pipe",
            stderr: "pipe",
            ...(options.signal === undefined ? {} : { signal: options.signal }),
        });
        const input = child.stdin.getWriter();
        await input.write(new TextEncoder().encode(options.input ?? ""));
        await input.close();
        const [code, output, errors] = await environment.clock.trackHost(
            Promise.all([
                child.exited,
                new Response(child.stdout).text(),
                new Response(child.stderr).text(),
            ]),
        );

        // fail with why the signal aborted, or with git's message
        options.signal?.throwIfAborted();
        if (code !== 0) {
            throw ForgeError.of(
                "GIT_FAILED",
                `git ${command[0]} failed with ${code}: ${errors.trim()}`,
            );
        }

        return output;
    },
};
