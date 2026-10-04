import { spawn } from "node:child_process";
import { mkdir } from "node:fs/promises";
import { devNull } from "node:os";
import { text } from "node:stream/consumers";
import type { Lease } from "@destack/resource";

/** The commits fetched from a leased repository: the one commit a build names, without its history. */
const FETCH_DEPTH = 1;

/** Working trees of single commits that the git command line fetches through repository leases, over Git's smart HTTP or local transports. */
export const GitLease = {
    /** Fetch one commit of a leased repository into a new working tree in a directory, checked out detached. */
    async checkout(
        lease: Lease,
        commit: string,
        directory: string,
        signal: AbortSignal,
    ): Promise<void> {
        // start an empty repository in the directory
        await mkdir(directory, { recursive: true });
        await git(directory, [], ["init", "--quiet"], signal);

        // fetch the commit alone, sending the lease's headers with each request
        const headers = Object.entries(lease.headers).map(([name, value]): Setting => [
            "http.extraHeader",
            `${name}: ${value}`,
        ]);
        const fetched: [string, ...string[]] = [
            "fetch",
            "--quiet",
            `--depth=${FETCH_DEPTH}`,
            lease.url,
            commit,
        ];
        await git(directory, headers, fetched, signal);

        // check the fetched commit out
        const detached: Setting[] = [["advice.detachedHead", "false"]];
        await git(directory, detached, ["checkout", "--quiet", "FETCH_HEAD"], signal);
    },
};

/** A git configuration key and its value. */
type Setting = readonly [key: string, value: string];

/** Run a git subcommand in a directory with some configuration and none of the user's or system's, failing with its diagnostic. */
async function git(
    directory: string,
    configuration: readonly Setting[],
    command: readonly [string, ...string[]],
    signal: AbortSignal,
): Promise<void> {
    // pass the configuration in the environment, which keeps credentials out of the process list
    const settings = Object.fromEntries(
        configuration.flatMap(([key, value], index) => [
            [`GIT_CONFIG_KEY_${index}`, key],
            [`GIT_CONFIG_VALUE_${index}`, value],
        ]),
    );

    // run git without prompts until it exits or the signal aborts
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
        stdio: ["ignore", "ignore", "pipe"],
        signal,
    });
    const exited = new Promise<number | NodeJS.Signals | null>((resolve, reject) => {
        child.once("error", (error) => reject(signal.aborted ? signal.reason : error));
        child.once("close", (code, exitSignal) => resolve(exitSignal ?? code));
    });
    const [code, errors] = await Promise.all([exited, text(child.stderr)]);

    // fail with why the signal aborted, or with git's message
    signal.throwIfAborted();
    if (code !== 0) {
        throw new Error(`git ${command[0]} failed with ${code}: ${errors.trim()}`);
    }
}
