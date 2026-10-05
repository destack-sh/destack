import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { CapabilityError, type DirectoryGrant } from "@destack/package";
import { sqliteChannels } from "@destack/db/bun";
import { channelFiles } from "@destack/db/channel/socket";
import type { SandboxOptions } from "@destack/sandbox";
import type { InstanceSpec, WorkloadResource } from "../runtime/runtime.ts";

/** The journal files SQLite keeps beside a database file. */
const SQLITE_SIBLINGS = ["-wal", "-shm", "-journal"];

/** The directories an instance runs in: its build's files, and its installation's data and cache. */
export interface InstanceDirectories {
    /** The instance's directory with its build's files. */
    readonly build: string;
    /** The installation's data directory, always granted. */
    readonly data: string;
    /** The installation's cache directory, always granted. */
    readonly cache: string;
}

/** What a host gives one workload's sandbox: its executable, directories, runtime directory, egress and environment. */
export interface WorkloadSandboxOptions {
    /** The Bun executable. */
    readonly executable: string;
    /** The instance's directories. */
    readonly directories: InstanceDirectories;
    /** The runner's file below the build directory. */
    readonly runner: string;
    /** The host's runtime directory, where processes sharing a file meet. */
    readonly runtime: string;
    /** The host's egress, below which the workload calls addresses. */
    readonly egress: string;
    /** The host's environment, which granted names pass through from. */
    readonly environment: Readonly<Record<string, string | undefined>>;
    /** Find a command on the host's path. */
    which(command: string): string | undefined;
}

/** The sandbox a host runs a workload's Bun process in, derived from the capabilities it runs with. */
export const WorkloadSandbox = {
    /** Derive a workload's sandbox: its files, its installation's directories, its resources, the egress and its capabilities, refusing ones this host cannot grant. */
    options(spec: InstanceSpec, host: WorkloadSandboxOptions): SandboxOptions {
        // refuse the capabilities the sandbox cannot enforce
        // TODO #Incomplete: withhold loopback listening and direct loopback connections from workloads without `listen` once runners serve their host another way
        const { capabilities } = spec;
        const connect = capabilities.network?.connect ?? [];
        if (connect.includes("*")) {
            throw new CapabilityError(
                "UNENFORCEABLE",
                "network",
                "connecting to any host is not enforced by this host's sandbox, which allows listed hosts only",
            );
        }

        // read the granted directories and commands, and write the bound resources' files
        const directories = capabilities.fs === undefined ? [] : spec.directories;
        const readDirectories = grantedDirectories(directories, "read");
        const writeDirectories = grantedDirectories(directories, "write");
        const resourcePaths = spec.resources.flatMap((resource) => resourceFiles(resource, host));
        const commands = (capabilities.run?.commands ?? []).flatMap((command) => {
            const path = host.which(command);

            return path === undefined ? [] : [path];
        });

        return {
            executable: host.executable,
            arguments: ["--no-env-file", join(host.directories.build, host.runner)],
            directory: host.directories.build,
            environment: environment(capabilities.env?.names ?? [], host),
            read: [host.directories.build, ...readDirectories, ...commands],
            write: [
                host.directories.data,
                host.directories.cache,
                ...resourcePaths,
                ...writeDirectories,
            ],
            network: [new URL(host.egress).host, ...connect],
            allowsListening: true,
        };
    },
};

/** List the paths of the directories granted with an access. */
function grantedDirectories(
    directories: readonly DirectoryGrant[],
    access: DirectoryGrant["access"],
): string[] {
    return directories.flatMap((directory) =>
        directory.access === access ? [directory.path] : [],
    );
}

/** List the files a workload opens for a bound resource: its file, its SQLite journals and its channel, none for a resource without a file. */
function resourceFiles(resource: WorkloadResource, host: WorkloadSandboxOptions): string[] {
    // pass over a resource without a file
    if (resource.reference?.startsWith("file:") !== true) {
        return [];
    }

    // meet the file's other writers at its channels in the host's runtime directory
    const path = fileURLToPath(resource.reference);
    const channels = sqliteChannels(path).map((channel) => channelFiles(channel, host.runtime));

    return [
        path,
        ...SQLITE_SIBLINGS.map((suffix) => `${path}${suffix}`),
        ...channels.flatMap((channel) => [channel.socket, channel.lock]),
    ];
}

/** Pass the path, the installation's directories and the granted variables the host has. */
function environment(
    names: readonly string[],
    host: WorkloadSandboxOptions,
): Record<string, string> {
    // pass the granted variables the host has
    const granted: Record<string, string> = {};
    for (const name of names) {
        const value = host.environment[name];
        if (value !== undefined) {
            granted[name] = value;
        }
    }

    // pass the path and the directories
    const path = host.environment["PATH"];

    return {
        ...granted,
        ...(path === undefined ? {} : { PATH: path }),
        HOME: host.directories.data,
        XDG_DATA_HOME: host.directories.data,
        XDG_CACHE_HOME: host.directories.cache,
        XDG_RUNTIME_DIR: host.runtime,
    };
}
