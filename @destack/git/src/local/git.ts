import { type Environment, Telemetry } from "@destack/environment";
import { GitError } from "../error/index.ts";

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
    /** The values git's messages must not show, such as a lease's address. */
    readonly masked?: readonly string[];
}

/** The null device of each operating system, git's global configuration file. */
const NULL_DEVICE = { darwin: "/dev/null", linux: "/dev/null", win32: "\\\\.\\nul" } as const;

/** The git command line, run without prompts, user configuration or system configuration. */
export const Git = {
    /** Build the variables git runs with: the host's path, no prompts, only the given settings. */
    variables(
        environment: Environment<"process" | "host">,
        configuration: readonly GitSetting[],
    ): Record<string, string> {
        // pass the configuration in the environment, so credentials stay out of the process list
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

        return {
            ...host,
            GIT_CONFIG_NOSYSTEM: "1",
            GIT_CONFIG_GLOBAL: NULL_DEVICE[environment.host.os],
            GIT_TERMINAL_PROMPT: "0",
            GIT_CONFIG_COUNT: String(configuration.length),
            ...settings,
        };
    },

    /** Run a git command in a directory, returning its output and failing with git's message. */
    async run(
        directory: string,
        command: readonly [string, ...string[]],
        environment: Environment<"process" | "host">,
        options: GitOptions = {},
    ): Promise<string> {
        // send the caller's trace context with each request git makes, so a push joins its trace
        const traced = new Headers();
        Telemetry.inject(traced, environment.telemetry);
        const trace = [...traced].map(([name, value]): GitSetting => [
            "http.extraHeader",
            `${name}: ${value}`,
        ]);
        const configuration = [...(options.configuration ?? []), ...trace];

        // run git to completion, or until the signal aborts
        const { code, stdout, stderr } = await environment.process.run(
            ["git", "-C", directory, ...command],
            {
                variables: Git.variables(environment, configuration),
                input: options.input ?? "",
                ...(options.signal === undefined ? {} : { signal: options.signal }),
            },
        );
        const decoder = new TextDecoder();

        // fail with why the signal aborted, or with git's message
        options.signal?.throwIfAborted();
        if (code !== 0) {
            const message = (options.masked ?? []).reduce(
                (text, value) => text.replaceAll(value, "***"),
                decoder.decode(stderr).trim(),
            );
            throw GitError.of(
                "COMMAND_BROKEN",
                `git '${command[0]}' exited with status ${code}: ${message}`,
            );
        }

        return decoder.decode(stdout);
    },
};
