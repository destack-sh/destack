import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { CapabilityError, type DirectoryGrant } from "@destack/package";
import { channelFiles } from "@destack/db/channel/socket";
import type { SandboxOptions } from "@destack/sandbox";
import type { InstanceSpec, WorkloadResource } from "./runtime.ts";

/** The journal files SQLite keeps beside a database file. */
const SQLITE_SIBLINGS = ["-wal", "-shm", "-journal"];

/** Where a host runs one workload: its files, its installation's folders and the host's egress and runtime directory. */
export interface SandboxPlaces {
    /** The Bun executable. */
    readonly executable: string;
    /** The instance's directory with the output's files. */
    readonly directory: string;
    /** The runner's file below the directory. */
    readonly runner: string;
    /** The installation's data folder, always granted. */
    readonly data: string;
    /** The installation's cache folder, always granted. */
    readonly cache: string;
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
    /** Derive a workload's sandbox: its files, its installation's folders, its resources, the egress and its capabilities, refusing ones this host cannot grant. */
    options(spec: InstanceSpec, places: SandboxPlaces): SandboxOptions {
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
        const resourcePaths = spec.resources.flatMap((resource) => resourceFiles(resource, places));
        const commands = (capabilities.run?.commands ?? []).flatMap((command) => {
            const path = places.which(command);

            return path === undefined ? [] : [path];
        });

        return {
            executable: places.executable,
            arguments: ["--no-env-file", join(places.directory, places.runner)],
            directory: places.directory,
            environment: environment(capabilities.env?.names ?? [], places),
            read: [places.directory, ...readDirectories, ...commands],
            write: [places.data, places.cache, ...resourcePaths, ...writeDirectories],
            network: [new URL(places.egress).host, ...connect],
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
function resourceFiles(resource: WorkloadResource, places: SandboxPlaces): string[] {
    // pass over a resource without a file
    if (resource.reference?.startsWith("file:") !== true) {
        return [];
    }

    // meet the file's other writers at its channel in the host's runtime directory
    const path = fileURLToPath(resource.reference);
    const channel = channelFiles(path, { XDG_RUNTIME_DIR: places.runtime });

    return [
        path,
        ...SQLITE_SIBLINGS.map((suffix) => `${path}${suffix}`),
        channel.socket,
        channel.lock,
    ];
}

/** Pass the path, the installation's folders and the granted variables the host has. */
function environment(names: readonly string[], places: SandboxPlaces): Record<string, string> {
    // pass the granted variables the host has
    const granted: Record<string, string> = {};
    for (const name of names) {
        const value = places.environment[name];
        if (value !== undefined) {
            granted[name] = value;
        }
    }

    // pass the path and the folders
    const path = places.environment["PATH"];

    return {
        ...granted,
        ...(path === undefined ? {} : { PATH: path }),
        HOME: places.data,
        XDG_DATA_HOME: places.data,
        XDG_CACHE_HOME: places.cache,
        XDG_RUNTIME_DIR: places.runtime,
    };
}
