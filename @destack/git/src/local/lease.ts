import type { Lease } from "@destack/resource";
import type { GitAuthor } from "../storage/index.ts";
import { Git, type GitOptions, type GitSetting } from "./git.ts";
import type { LocalGitEnvironment } from "./storage.ts";

/** The commits fetched from a leased repository: one commit, without its history. */
const FETCH_DEPTH = 1;

/** Git's work on leased repositories: single-commit working trees, branch heads and pushes. */
export const GitLease = {
    /** Fetch one commit or reference of a leased repository into a new working tree. */
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
        await Git.run(
            directory,
            ["fetch", "--quiet", `--depth=${FETCH_DEPTH}`, lease.url, commit],
            environment,
            GitLease.options(lease, signal),
        );

        // check the fetched commit out
        await Git.run(directory, ["checkout", "--quiet", "FETCH_HEAD"], environment, {
            configuration: [["advice.detachedHead", "false"]],
            signal,
        });
    },

    /** Fetch a branch's head into a new working tree, answering its commit. */
    async branch(
        lease: Lease,
        branch: string,
        directory: string,
        environment: LocalGitEnvironment,
        signal: AbortSignal,
    ): Promise<string> {
        // check the branch's head out alone
        await GitLease.checkout(lease, `refs/heads/${branch}`, directory, environment, signal);

        return (await Git.run(directory, ["rev-parse", "HEAD"], environment, { signal })).trim();
    },

    /** Read a branch's head at a leased repository's remote, absent for a missing branch. */
    async head(
        lease: Lease,
        branch: string,
        directory: string,
        environment: LocalGitEnvironment,
        signal: AbortSignal,
    ): Promise<string | undefined> {
        // list the branch's reference at the remote
        const listed = await Git.run(
            directory,
            ["ls-remote", lease.url, `refs/heads/${branch}`],
            environment,
            GitLease.options(lease, signal),
        );
        const [commit] = listed.trim().split("\t");

        return commit === "" ? undefined : commit;
    },

    /** Commit a working tree's changes and push them to a branch still at a base. */
    async push(
        lease: Lease,
        directory: string,
        target: { readonly branch: string; readonly base: string },
        commit: {
            readonly message: string;
            readonly author: GitAuthor;
            readonly paths: readonly string[];
        },
        environment: LocalGitEnvironment,
        signal: AbortSignal,
    ): Promise<string> {
        // commit the changed paths as the author
        const author: readonly GitSetting[] = [
            ["user.name", commit.author.name],
            ["user.email", commit.author.email],
        ];
        await Git.run(directory, ["add", "--", ...commit.paths], environment, { signal });
        await Git.run(directory, ["commit", "--quiet", "--file=-"], environment, {
            configuration: author,
            input: commit.message,
            signal,
        });

        // push it, refusing a branch another push moved past the base
        const reference = `refs/heads/${target.branch}`;
        await Git.run(
            directory,
            [
                "push",
                "--quiet",
                `--force-with-lease=${reference}:${target.base}`,
                lease.url,
                `HEAD:${reference}`,
            ],
            environment,
            GitLease.options(lease, signal),
        );

        return (await Git.run(directory, ["rev-parse", "HEAD"], environment, { signal })).trim();
    },

    /** Point a leased repository's reference at a local commit, replacing what it pointed at. */
    async set(
        lease: Lease,
        directory: string,
        reference: string,
        commit: string,
        environment: LocalGitEnvironment,
        signal: AbortSignal,
    ): Promise<void> {
        await Git.run(
            directory,
            ["push", "--quiet", "--force", lease.url, `${commit}:${reference}`],
            environment,
            GitLease.options(lease, signal),
        );
    },

    /** Read the options git reaches a leased repository with: its headers, its address masked. */
    options(lease: Lease, signal: AbortSignal): GitOptions {
        const configuration = Object.entries(lease.headers).map(([name, value]): GitSetting => [
            "http.extraHeader",
            `${name}: ${value}`,
        ]);

        return { configuration, masked: [lease.url], signal };
    },
};
