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

        // ask Git for the working tree's root
        const root = await this.#git(canonical, "rev-parse", "--show-toplevel");

        return { directory: canonical, root: await this.host.files.realPath(root) };
    }

    /** Open a checkout's working tree from its source. */
    open(directory: string, source: WorkingTreeSource): Promise<WorkingTree> {
        // register the working tree already there
        if (source.kind === "existing") {
            return this.root(directory);
        }
        // initialize a new repository
        else if (source.kind === "empty") {
            return this.initialize(directory);
        }
        // clone the remote
        else {
            return this.clone(directory, source.remote, source.ref);
        }
    }

    /** Create a directory with a new Git repository, answering its working tree. */
    async initialize(directory: string): Promise<WorkingTree> {
        // create the directory and the repository in it
        Git.#requireAbsolute(directory);
        await this.host.files.mkdir(directory, { recursive: true });
        await this.#git(directory, "init", "--quiet");

        return this.root(directory);
    }

    /** Clone a remote repository into a new directory, and answer its working tree. */
    async clone(directory: string, remote: string, reference?: string): Promise<WorkingTree> {
        // clone into the new directory, then check out the reference
        Git.#requireAbsolute(directory);
        await this.#asPerson(dirname(directory), "clone", "--quiet", "--", remote, directory);
        if (reference !== undefined) {
            await this.#asPerson(directory, "checkout", "--quiet", reference, "--");
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

    /** Read the commit a working tree is at when clean, untracked files included. */
    async commit(root: string): Promise<string | undefined> {
        const status = await this.#git(root, "status", "--porcelain", "--untracked-files=normal");

        return status === "" ? this.#git(root, "rev-parse", "HEAD") : undefined;
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

    /** Run a Git command that calls a remote as the person running the machine. */
    async #asPerson(directory: string, ...command: readonly string[]): Promise<void> {
        // run Git in the person's environment, without prompting, its remote maybe answering from the world
        await Command.run(this.host, ["git", "-C", directory, ...command], {
            variables: { ...this.host.process.variables, GIT_TERMINAL_PROMPT: "0" },
            timeout: REMOTE_GIT_MILLISECONDS,
        });
    }

    /** Run a Git command in a directory without user or system configuration. */
    async #git(directory: string, ...command: readonly string[]): Promise<string> {
        // run Git with the host's search path alone, and read its output and diagnostic
        const { variables } = this.host.process;
        const system = Object.fromEntries(
            ["PATH", "SystemRoot"].flatMap((name) => {
                const value = variables[name];

                return value === undefined ? [] : [[name, value]];
            }),
        );
        const { output } = await Command.run(this.host, ["git", "-C", directory, ...command], {
            variables: {
                ...system,
                GIT_CONFIG_NOSYSTEM: "1",
                GIT_CONFIG_GLOBAL: NULL_DEVICE[this.host.host.os],
            },
            timeout: GIT_TIMEOUT_MILLISECONDS,
        });

        return output.trim();
    }
}
