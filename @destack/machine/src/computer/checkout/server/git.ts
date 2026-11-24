import type { GitAuthor } from "@destack/git";
import { Uuid } from "@destack/schema";
import { dirname, isAbsolute } from "node:path";
import { FileSystemError } from "@destack/environment";
import { Command, type CommandHost } from "../../process/command.ts";
import type { WorkingTree, WorkingTreeSource } from "../object/tree.ts";
import type {} from "@destack/package/import-meta";
import { MachineError } from "../../../error/index.ts";

/** How long a Git command reading a working tree may take, in milliseconds. */
const GIT_TIMEOUT_MILLISECONDS = 3000;

/** How long a Git command calling a remote may take: 10 minutes, six 1 GB clones at 10 MB/s. */
const REMOTE_GIT_MILLISECONDS = 10 * 60_000;

/** The prefix of the index a snapshot stages a working tree in, inside the Git directory. */
const SNAPSHOT_INDEX = "destack-snapshot-index";

/** The directory of a working tree's build mirror, inside its repository's Git directory. */
const MIRROR_DIRECTORY = "destack-build";

/** How long writing a mirror's files may take: 1 minute, 100k files at about 2k files a second. */
const MIRROR_MILLISECONDS = 60_000;

/** The lockfiles whose change makes a mirror install its dependencies again. */
const LOCKFILES = ":(glob)**/bun.lock";

/** The null device of each operating system, Git's global configuration file. */
const NULL_DEVICE = { darwin: "/dev/null", linux: "/dev/null", win32: "\\\\.\\nul" } as const;

/** The Git verbs reading and creating a checkout's working tree on a host. */
export class Git {
    /** The host running Git: its files, processes and platform. */
    readonly host: CommandHost;

    /** Run Git on a host. */
    constructor(host: CommandHost) {
        this.host = host;
    }

    /** Resolve an absolute directory to its canonical path and the root of its Git working tree. */
    async root(directory: string): Promise<WorkingTree> {
        // require an absolute path before asking Git
        Git.#requireAbsolute(directory);
        const canonical = await this.host.files.realPath(directory).catch((error: unknown) => {
            if (!FileSystemError.is(error, "FILE_NOT_FOUND")) {
                throw error;
            }
            throw MachineError.of("DIRECTORY_NOT_FOUND", `directory '${directory}' not found`);
        });

        // ask Git for the working tree's root and its origin, which Git leaves unset with exit code 1
        const root = await this.#git(canonical, "rev-parse", "--show-toplevel");
        const { code, output } = await Command.run(
            this.host,
            ["git", "-C", canonical, "config", "--get", "remote.origin.url"],
            { timeout: GIT_TIMEOUT_MILLISECONDS, accepted: [1] },
        );

        return {
            directory: canonical,
            root: await this.host.files.realPath(root),
            remote: code === 0 ? output.trim() : null,
        };
    }

    /** Open a checkout's working tree from its source. */
    open(directory: string, source: WorkingTreeSource): Promise<WorkingTree> {
        // register the working tree already there
        if (source.kind === "existing") {
            return this.root(directory);
        }
        // clone the remote
        else if (source.kind === "clone") {
            return this.clone(directory, source.remote, source.ref);
        }
        // initialize a new repository, tracking a template's empty remote as a clone of it does
        else {
            return this.initialize(
                directory,
                source.kind === "template" ? source.remote : undefined,
            );
        }
    }

    /** Create a directory with a new Git repository, its origin a remote when given. */
    async initialize(directory: string, remote?: string): Promise<WorkingTree> {
        // create the directory and the repository in it
        Git.#requireAbsolute(directory);
        await this.host.files.mkdir(directory, { recursive: true });
        await this.#git(directory, "init", "--quiet");
        if (remote !== undefined) {
            await this.#git(directory, "remote", "add", "origin", remote);
        }

        return this.root(directory);
    }

    /** Clone a remote repository into a new directory, and answer its working tree. */
    async clone(directory: string, remote: string, reference?: string): Promise<WorkingTree> {
        // clone into the new directory, then check out the reference
        Git.#requireAbsolute(directory);
        await this.#remote(dirname(directory), "clone", "--quiet", "--", remote, directory);
        if (reference !== undefined) {
            await this.#remote(directory, "checkout", "--quiet", reference, "--");
        }

        return this.root(directory);
    }

    /** Resolve the directory a checkout's working tree moved to. */
    async relocated(previous: string, directory: string): Promise<WorkingTree> {
        // require the working tree moved away from its earlier directory
        if (await this.host.files.exists(previous)) {
            throw MachineError.of(
                "TREE_BLOCKED",
                `cannot relocate the checkout: the working tree is still at '${previous}', so move it first`,
            );
        }

        return this.root(directory);
    }

    /** Commit a working tree as it is on its HEAD, which the commit is for a clean tree. */
    async snapshot(
        root: string,
        message: string,
        author: GitAuthor,
    ): Promise<{ readonly commit: string; readonly head: string }> {
        // read HEAD, refusing a working tree without a commit
        const { code, output } = await Command.run(
            this.host,
            ["git", "-C", root, "rev-parse", "--verify", "--quiet", "HEAD"],
            { timeout: GIT_TIMEOUT_MILLISECONDS, accepted: [1] },
        );
        if (code !== 0) {
            throw MachineError.of(
                "HEAD_NOT_FOUND",
                `HEAD of '${root}' not found: the working tree has no commit, so commit first`,
            );
        }
        const head = output.trim();

        // answer HEAD when the working tree is its tree, else commit the tree on HEAD as the person
        const tree = await this.tree(root);
        const isClean = tree === (await this.#git(root, "rev-parse", `${head}^{tree}`));
        const commit = isClean
            ? head
            : await this.#local(
                  root,
                  {},
                  ...Git.#identity(author),
                  "commit-tree",
                  tree,
                  "-p",
                  head,
                  "-m",
                  message,
              );

        return { commit, head };
    }

    /** Commit every file of a new working tree as the person, answering the commit and branch. */
    async introduce(
        root: string,
        message: string,
        author: GitAuthor,
    ): Promise<{ readonly commit: string; readonly branch: string }> {
        // commit every file on the working tree's branch
        await this.#local(root, {}, "add", "--all");
        await this.#local(root, {}, ...Git.#identity(author), "commit", "--quiet", "-m", message);
        const branch = await this.#git(root, "symbolic-ref", "--short", "HEAD");

        return { commit: await this.#git(root, "rev-parse", "HEAD"), branch };
    }

    /** Write a working tree's files as they are into a Git tree, answering the tree's name. */
    async tree(root: string): Promise<string> {
        // stage every file on a fresh copy of the index, leaving the real index and HEAD unchanged
        const path = (name: string) =>
            this.#git(root, "rev-parse", "--path-format=absolute", "--git-path", name);
        const index = await path(`${SNAPSHOT_INDEX}-${Uuid.v4(this.host.random)}`);
        const real = await path("index");
        if (await this.host.files.exists(real)) {
            await this.host.files.copyFile(real, index);
        }

        // write the tree, removing the copy after
        try {
            const staged = { GIT_INDEX_FILE: index };
            await this.#local(root, staged, "add", "--all");

            return await this.#local(root, staged, "write-tree");
        } finally {
            await this.host.files.remove(index, { force: true });
        }
    }

    /** Report whether a working tree's repository has a commit. */
    async has(root: string, commit: string): Promise<boolean> {
        // leave a working tree that moved away
        if (!(await this.host.files.exists(root))) {
            return false;
        }
        const verify = ["rev-parse", "--verify", "--quiet", `${commit}^{commit}`];
        const { code } = await this.#run(root, verify, {
            timeout: GIT_TIMEOUT_MILLISECONDS,
            accepted: [1],
        });

        return code === 0;
    }

    /** Move a working tree's build mirror to a commit, creating it once. */
    async mirror(
        root: string,
        commit: string,
    ): Promise<{ readonly path: string; readonly isLockfileChanged: boolean }> {
        // place the mirror in the repository's Git directory, never running the repository's hooks
        const path = await this.#mirrorPath(root);
        const hooks = ["-c", `core.hooksPath=${NULL_DEVICE[this.host.host.os]}`];
        const writing = { timeout: MIRROR_MILLISECONDS };

        // create it once, as a detached worktree sharing the repository's objects
        if (!(await this.host.files.exists(path))) {
            await this.#run(
                root,
                [...hooks, "worktree", "add", "--detach", "--force", path, commit],
                writing,
            );

            return { path, isLockfileChanged: true };
        }

        // move it to the commit, removing the files the commit lacks except installed dependencies
        const previous = await this.#git(path, "rev-parse", "HEAD");
        await this.#run(
            path,
            [...hooks, "checkout", "--quiet", "--force", "--detach", commit],
            writing,
        );
        await this.#run(path, ["clean", "-ffdxq", "--exclude=node_modules"], writing);

        // compare the lockfiles of the previous commit and this one
        const compared = ["diff", "--quiet", previous, commit, "--", LOCKFILES];
        const { code } = await this.#run(path, compared, {
            timeout: GIT_TIMEOUT_MILLISECONDS,
            accepted: [1],
        });

        return { path, isLockfileChanged: code === 1 };
    }

    /** Remove a working tree's build mirror, answering its path, absent without one. */
    async removeMirror(root: string): Promise<string | undefined> {
        // leave a working tree that moved away, or one without a mirror
        if (!(await this.host.files.exists(root))) {
            return undefined;
        }
        const path = await this.#mirrorPath(root);
        if (!(await this.host.files.exists(path))) {
            return undefined;
        }

        // remove the worktree and its files
        await this.#run(root, ["worktree", "remove", "--force", path], {
            timeout: MIRROR_MILLISECONDS,
        });

        return path;
    }

    /** Read the path of a working tree's build mirror. */
    #mirrorPath(root: string): Promise<string> {
        return this.#git(
            root,
            "rev-parse",
            "--path-format=absolute",
            "--git-path",
            MIRROR_DIRECTORY,
        );
    }

    /** Refuse a checkout directory that is no absolute path. */
    static #requireAbsolute(directory: string): void {
        if (!isAbsolute(directory)) {
            throw MachineError.of(
                "RELATIVE_DIRECTORY_INVALID",
                "checkout directory is invalid: it must be absolute",
            );
        }
    }

    /** Write the configuration of a commit's author and committer. */
    static #identity(author: GitAuthor): readonly string[] {
        return ["-c", `user.name=${author.name}`, "-c", `user.email=${author.email}`];
    }

    /** Run a local Git command as the person running the machine, with more variables. */
    async #local(
        directory: string,
        variables: Readonly<Record<string, string>>,
        ...command: readonly string[]
    ): Promise<string> {
        const { output } = await Command.run(this.host, ["git", "-C", directory, ...command], {
            variables: { ...this.host.process.variables, ...variables },
            timeout: GIT_TIMEOUT_MILLISECONDS,
        });

        return output.trim();
    }

    /** Run a Git command that calls a remote as the person running the machine. */
    async #remote(directory: string, ...command: readonly string[]): Promise<void> {
        // run Git in the person's environment, without prompting, its remote maybe answering from the world
        await Command.run(this.host, ["git", "-C", directory, ...command], {
            variables: { ...this.host.process.variables, GIT_TERMINAL_PROMPT: "0" },
            timeout: REMOTE_GIT_MILLISECONDS,
        });
    }

    /** Run a Git command in a directory without user or system configuration. */
    async #git(directory: string, ...command: readonly string[]): Promise<string> {
        const { output } = await this.#run(directory, command, {
            timeout: GIT_TIMEOUT_MILLISECONDS,
        });

        return output.trim();
    }

    /** Run a Git command without user or system configuration, answering its code and output. */
    async #run(
        directory: string,
        command: readonly string[],
        options: { readonly timeout: number; readonly accepted?: readonly number[] },
    ): Promise<{ readonly code: number; readonly output: string }> {
        // run Git with the host's search path alone, and read its output and diagnostic
        const { variables } = this.host.process;
        const system = Object.fromEntries(
            ["PATH", "SystemRoot"].flatMap((name) => {
                const value = variables[name];

                return value === undefined ? [] : [[name, value]];
            }),
        );

        return await Command.run(this.host, ["git", "-C", directory, ...command], {
            variables: {
                ...system,
                GIT_CONFIG_NOSYSTEM: "1",
                GIT_CONFIG_GLOBAL: NULL_DEVICE[this.host.host.os],
            },
            ...options,
        });
    }
}
