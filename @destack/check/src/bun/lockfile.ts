import { readFile, stat } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { schema } from "@destack/schema";

/** The file Bun records a workspace's resolutions in, at the workspace root. */
const LOCKFILE = "bun.lock";

/** The dependency specifiers of one dependency group, by name. */
const Specifiers = schema.record(schema.string(), schema.string()).exactOptional();

/** A workspace member as the lockfile records it. */
const LockedWorkspace = schema
    .object({
        name: schema.string().exactOptional(),
        dependencies: Specifiers,
        devDependencies: Specifiers,
        peerDependencies: Specifiers,
        optionalDependencies: Specifiers,
    })
    .strip();
/** A workspace member as the lockfile records it. */
export type LockedWorkspace = schema.Infer<typeof LockedWorkspace>;

/** A resolved package: its release, its tarball location, its metadata and its integrity. */
const LockedPackage = schema.tuple([
    schema.string(),
    schema.string().exactOptional(),
    schema.json().exactOptional(),
    schema.string().exactOptional(),
]);
/** A resolved package: its release, its tarball location, its metadata and its integrity. */
export type LockedPackage = schema.Infer<typeof LockedPackage>;

/** The fields of a lockfile that workspaces and builds read. */
const LockfileText = schema
    .object({
        lockfileVersion: schema.number(),
        workspaces: schema.record(schema.string(), LockedWorkspace),
        packages: schema.record(schema.string(), LockedPackage),
    })
    .strip();

/** A Bun lockfile: the workspace members and resolved packages it records at its workspace root. */
export class BunLockfile {
    /** The absolute directory holding the lockfile. */
    readonly directory: string;
    /** The lockfile format version. */
    readonly version: number;
    /** The workspace members by directory, with the root as "". */
    readonly workspaces: Readonly<Record<string, LockedWorkspace>>;
    /** The resolved packages by their key in the dependency tree. */
    readonly packages: Readonly<Record<string, LockedPackage>>;

    /** Keep a parsed lockfile. */
    private constructor(directory: string, text: schema.Infer<typeof LockfileText>) {
        // keep the fields the readers use
        this.directory = directory;
        this.version = text.lockfileVersion;
        this.workspaces = text.workspaces;
        this.packages = text.packages;
    }

    /** Read the nearest lockfile in a directory or above it, absent without one. */
    static async read(directory: string): Promise<BunLockfile | undefined> {
        // locate the nearest lockfile
        const root = await BunLockfile.locate(directory);
        if (root === undefined) {
            return undefined;
        }

        // parse its text, which carries trailing commas
        const text = await readFile(join(root, LOCKFILE), "utf8");

        return new BunLockfile(root, LockfileText.parse(JSON.parse(dropTrailingCommas(text))));
    }

    /** Find the nearest directory holding a lockfile, absent without one. */
    static async locate(directory: string): Promise<string | undefined> {
        for (let current = resolve(directory); ; current = dirname(current)) {
            if (await exists(join(current, LOCKFILE))) {
                return current;
            }
            if (dirname(current) === current) {
                return undefined;
            }
        }
    }
}

/** Report whether a path exists. */
async function exists(path: string): Promise<boolean> {
    try {
        await stat(path);

        return true;
    } catch (error) {
        if (!isMissing(error)) {
            throw error;
        }

        return false;
    }
}

/** Report whether a file system error names a missing path. */
function isMissing(error: unknown): boolean {
    return error instanceof Error && "code" in error && error.code === "ENOENT";
}

/** Drop the trailing commas Bun writes into lockfiles, matching strings whole to keep their commas. */
function dropTrailingCommas(text: string): string {
    return text.replaceAll(/("(?:[^"\\]|\\.)*")|,(?=\s*[}\]])/gu, "$1");
}
