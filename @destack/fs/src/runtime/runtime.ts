import type { Stats } from "node:fs";
import { lstat, mkdir } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { FileSystemError } from "../error/index.ts";

/** The permission bits of a runtime directory shared with the group or others, which a person's own directory lacks. */
const SHARED_MODE = 0o077;

/** Where a process finds the directory of the person's runtime files. */
export interface RuntimeDirectoryOptions {
    /** The environment, read for `XDG_RUNTIME_DIR`. */
    readonly environment?: Readonly<Record<string, string | undefined>>;
    /** The temporary directory a per-user directory goes under. */
    readonly temporary?: string;
    /** The user the directory belongs to, the process's own by default. */
    readonly user?: number;
}

/**
 * Find the directory of the person's runtime files, such as sockets and lock files.
 *
 * It is `XDG_RUNTIME_DIR` when set, or else a `destack-<uid>` directory under the temporary one, created with mode 0700.
 * A per-user directory that another user owns, that others may open, or that is a link is refused, as ssh-agent and tmux refuse theirs.
 * Systems without user identifiers keep runtime files in the temporary directory, which is the person's own there.
 */
export async function runtimeDirectory(options: RuntimeDirectoryOptions = {}): Promise<string> {
    // take the session's runtime directory
    const environment = options.environment ?? process.env;
    const session = environment["XDG_RUNTIME_DIR"];
    if (session !== undefined && session !== "") {
        return session;
    }

    // take the person's own temporary directory where users have no identifiers
    const temporary = options.temporary ?? tmpdir();
    const user = options.user ?? process.getuid?.();
    if (user === undefined) {
        return temporary;
    }

    // create the per-user directory unless it exists
    const directory = join(temporary, `destack-${user}`);
    const status = await createPrivate(directory);

    // refuse a link, another user's directory, and one the group or others may open
    if (!status.isDirectory() || status.uid !== user || (status.mode & SHARED_MODE) !== 0) {
        throw new FileSystemError("trust runtime directory", directory, "EACCES");
    }

    return directory;
}

/** Create a directory with mode 0700 unless it exists, and read its status without following a link. */
async function createPrivate(directory: string): Promise<Stats> {
    try {
        await mkdir(directory, { mode: 0o700, recursive: true });

        return await lstat(directory);
    } catch (cause) {
        // pass on a failure without an operating-system code
        if (!(cause instanceof Error) || !("code" in cause) || typeof cause.code !== "string") {
            throw cause;
        }

        // report any other failure with its operation and path
        throw new FileSystemError("create runtime directory", directory, cause.code, { cause });
    }
}
