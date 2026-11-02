import type { Lease } from "@destack/resource";
import { Git, type GitSetting } from "./git.ts";
import type { LocalGitEnvironment } from "./storage.ts";

/** The commits fetched from a leased repository: the build's one commit, without its history. */
const FETCH_DEPTH = 1;

/** Working trees of single commits that git fetches through repository leases. */
export const GitLease = {
    /** Fetch one commit of a leased repository into a new working tree in a directory. */
    async checkout(
        lease: Lease,
        commit: string,
        directory: string,
        environment: LocalGitEnvironment,
        signal: AbortSignal,
    ): Promise<void> {
        // start an empty repository in the directory
        await environment.files.mkdir(directory, { recursive: true });
        await Git.run(directory, ["init", "--quiet"], environment, { signal });

        // fetch the commit alone, sending the lease's headers with each request
        const headers = Object.entries(lease.headers).map(([name, value]): GitSetting => [
            "http.extraHeader",
            `${name}: ${value}`,
        ]);
        await Git.run(
            directory,
            ["fetch", "--quiet", `--depth=${FETCH_DEPTH}`, lease.url, commit],
            environment,
            { configuration: headers, signal },
        );

        // check the fetched commit out
        await Git.run(directory, ["checkout", "--quiet", "FETCH_HEAD"], environment, {
            configuration: [["advice.detachedHead", "false"]],
            signal,
        });
    },
};
