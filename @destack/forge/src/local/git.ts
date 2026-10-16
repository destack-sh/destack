import { spawn } from "node:child_process";
import { devNull } from "node:os";
import { text } from "node:stream/consumers";

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

/** The git command line, run without prompts, user configuration or system configuration. */
export const Git = {
    /** Run a git command in a directory, returning its output and failing with git's message. */
    async run(
        directory: string,
        command: readonly [string, ...string[]],
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

        // run git until it exits or the signal aborts
        const child = spawn("git", ["-C", directory, ...command], {
            env: {
                PATH: process.env["PATH"],
                SystemRoot: process.env["SystemRoot"],
                GIT_CONFIG_NOSYSTEM: "1",
                GIT_CONFIG_GLOBAL: devNull,
                GIT_TERMINAL_PROMPT: "0",
                GIT_CONFIG_COUNT: String(configuration.length),
                ...settings,
            },
            ...(options.signal === undefined ? {} : { signal: options.signal }),
        });
        child.stdin.end(options.input ?? "");
        const exited = new Promise<number | NodeJS.Signals | null>((resolve, reject) => {
            child.once("error", (error) => reject(options.signal?.reason ?? error));
            child.stdin.once("error", reject);
            child.once("close", (code, signal) => resolve(signal ?? code));
        });
        const [code, output, errors] = await Promise.all([
            exited,
            text(child.stdout),
            text(child.stderr),
        ]);

        // fail with why the signal aborted, or with git's message
        options.signal?.throwIfAborted();
        if (code !== 0) {
            throw new Error(`git ${command[0]} failed with ${code}: ${errors.trim()}`);
        }

        return output;
    },
};
