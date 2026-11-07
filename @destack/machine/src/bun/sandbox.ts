import { MachineError } from "../error/index.ts";
import { BucketKind } from "@destack/bucket";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { PackageError, type DirectoryGrant } from "@destack/package";
import { bunSqliteChannels } from "@destack/db/bun";
import { channelFiles } from "@destack/db/channel/socket";
import type { SandboxOptions } from "@destack/sandbox";
import type { InstanceSpec, WorkloadResource } from "@destack/stack/runtime";

/** The journal files SQLite writes beside a database file. */
const SQLITE_SIBLINGS = ["-wal", "-shm", "-journal"];

/** The directories an instance runs in: its build, its dependency trees, its data and its cache. */
export interface BunInstanceLayout {
    /** The instance's directory with its build's files. */
    readonly build: string;
    /** The roots of the build's closure in the store's trees, which its dependencies link into. */
    readonly roots: readonly string[];
    /** The installation's data directory, always granted. */
    readonly data: string;
    /** The installation's cache directory, always granted. */
    readonly cache: string;
}

/** What a machine gives one workload's sandbox. */
export interface BunWorkloadSandboxOptions {
    /** The Bun executable. */
    readonly executable: string;
    /** The instance's directories. */
    readonly directories: BunInstanceLayout;
    /** The runner's file below the build directory. */
    readonly runner: string;
    /** The machine's runtime directory, where processes sharing a file meet. */
    readonly runtime: string;
    /** The machine's egress, below which the workload calls addresses. */
    readonly egress: string;
    /** The Unix socket the runner serves its machine on. */
    readonly socket: string;
    /** The machine's environment, which granted names pass through from. */
    readonly environment: Readonly<Record<string, string | undefined>>;
    /** Find a command on the machine's path. */
    readonly which: (command: string) => string | undefined;
}

/** The sandbox a machine runs a workload's Bun process in. */
export class BunWorkloadSandbox {
    /** What the machine gives the workload's sandbox. */
    readonly machine: BunWorkloadSandboxOptions;

    /** Create the sandbox of what a machine gives a workload. */
    constructor(machine: BunWorkloadSandboxOptions) {
        this.machine = machine;
    }

    /** Derive a workload's sandbox options. */
    options(spec: InstanceSpec): SandboxOptions {
        // refuse the abilities the sandbox cannot enforce
        const { machine } = this;
        const { sandbox } = spec;
        const connect = sandbox.network?.connect ?? [];
        if (connect.includes("*")) {
            throw PackageError.of(
                "ABILITY_UNENFORCEABLE",
                "connecting to any host is not enforced by this machine's sandbox, which allows listed hosts only",
                { data: { ability: "network" } },
            );
        }

        // read the granted directories and commands and write the bound resources' files
        const directories = sandbox.files === undefined ? [] : spec.directories;
        const resources = spec.resources.map((resource) => this.#resource(resource));
        const commands = (sandbox.run?.commands ?? []).map((command) => {
            const path = machine.which(command);
            if (path === undefined) {
                throw MachineError.of(
                    "COMMAND_MISSING",
                    `${spec.name} runs ${command}, which this machine lacks`,
                );
            }

            return path;
        });

        // serve the machine on the runner's socket, and listen on loopback only with `listen`
        return {
            executable: machine.executable,
            arguments: [
                "--no-env-file",
                join(machine.directories.build, machine.runner),
                machine.socket,
            ],
            directory: machine.directories.build,
            environment: this.#environment(sandbox.env?.names ?? []),
            read: [
                machine.directories.build,
                ...machine.directories.roots,
                ...BunWorkloadSandbox.#granted(directories, "read"),
                ...commands,
            ],
            write: [
                machine.directories.data,
                machine.directories.cache,
                machine.socket,
                ...resources.flatMap((resource) => resource.files),
                ...BunWorkloadSandbox.#granted(directories, "write"),
            ],
            network: [new URL(machine.egress).host, ...connect],
            sockets: [machine.socket, ...resources.flatMap((resource) => resource.sockets)],
            listening: sandbox.server === undefined ? "denied" : "loopback",
        };
    }

    /** List the files and sockets a workload opens for a bound resource. */
    #resource(resource: WorkloadResource): {
        readonly files: readonly string[];
        readonly sockets: readonly string[];
    } {
        // pass over a resource without a file
        const { reference } = resource;
        if (
            reference === null ||
            !reference.startsWith("file:") ||
            resource.kind === BucketKind.name
        ) {
            return { files: [], sockets: [] };
        }

        // write the file, its journals and the channels where it meets its other writers
        const path = fileURLToPath(reference);
        const channels = bunSqliteChannels(path).map((channel) =>
            channelFiles(channel, this.machine.runtime),
        );

        return {
            files: [
                path,
                ...SQLITE_SIBLINGS.map((suffix) => `${path}${suffix}`),
                ...channels.flatMap((channel) => [channel.socket, channel.lock]),
            ],
            sockets: channels.map((channel) => channel.socket),
        };
    }

    /** Pass the path, the installation's directories and the granted variables the machine has. */
    #environment(names: readonly string[]): Record<string, string> {
        // pass the granted variables the machine has
        const { directories, environment, runtime } = this.machine;
        const granted: Record<string, string> = {};
        for (const name of names) {
            const value = environment[name];
            if (value !== undefined) {
                granted[name] = value;
            }
        }

        // pass the path and the directories
        const path = environment["PATH"];

        return {
            ...granted,
            ...(path === undefined ? {} : { PATH: path }),
            HOME: directories.data,
            XDG_DATA_HOME: directories.data,
            XDG_CACHE_HOME: directories.cache,
            XDG_RUNTIME_DIR: runtime,
        };
    }

    /** List the paths of the directories granted with an access. */
    static #granted(
        directories: readonly DirectoryGrant[],
        access: DirectoryGrant["access"],
    ): string[] {
        return directories.flatMap((directory) =>
            directory.access === access ? [directory.path] : [],
        );
    }
}
