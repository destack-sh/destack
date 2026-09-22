import { resolve } from "node:path";
import { setTimeout } from "node:timers/promises";
import { FileSystemError } from "../error/index.ts";

/** Delay between nonblocking operating-system lock attempts. */
const POLL_INTERVAL = 25;

/**
 * An exclusive operating-system file lock.
 * Use a persistent lock file in a trusted directory; keep its path intact while locks can exist.
 */
export class FileLock implements AsyncDisposable {
    /** The locked file's absolute path. */
    readonly path: string;
    /** The retained operating-system file. */
    private readonly native: NativeLock;

    /** Retain the open file until explicitly released. */
    private constructor(path: string, native: NativeLock) {
        this.path = path;
        this.native = native;
    }

    /** Open or create a file and wait for exclusive access. */
    static async acquire(path: string, options: FileLockOptions = {}): Promise<FileLock> {
        // open the file before attempting ownership
        options.signal?.throwIfAborted();
        const native = await open(path);

        try {
            // poll native ownership with cancellable waits
            while (true) {
                options.signal?.throwIfAborted();
                if (native.tryAcquire()) {
                    return new FileLock(native.path, native);
                }

                await setTimeout(POLL_INTERVAL, undefined, { signal: options.signal });
            }
        } catch (error) {
            await native.close();

            throw error;
        }
    }

    /** Open or create a file; return undefined on contention and throw on filesystem failure. */
    static async tryAcquire(path: string): Promise<FileLock | undefined> {
        // retain the file only after acquiring ownership
        const native = await open(path);
        let isAcquired = false;

        try {
            isAcquired = native.tryAcquire();
            if (isAcquired) {
                return new FileLock(native.path, native);
            }
        } finally {
            if (!isAcquired) {
                await native.close();
            }
        }

        return undefined;
    }

    /** Release exclusive access and close the file, leaving its directory entry intact. */
    async close(): Promise<void> {
        await this.native.close();
    }

    /** Release the lock when its scope ends. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.close();
    }
}

/** Control a pending lock acquisition. */
export interface FileLockOptions {
    /** Cancel pending acquisition. */
    signal?: AbortSignal;
}

/** Platform implementations share acquisition and closure operations. */
type NativeLock = import("./unix.ts").NativeLock | import("./windows.ts").NativeLock;

/** Open a lock file through the current operating system. */
async function open(path: string): Promise<NativeLock> {
    // resolve the path before opening the native file
    const absolute = resolve(path);

    // reject embedded terminators before passing the path to a native string API
    if (absolute.includes("\0")) {
        throw new FileSystemError("open", absolute, "EINVAL");
    }

    // load only the current platform's system libraries
    if (process.platform === "win32") {
        return await (await import("./windows.ts")).NativeLock.open(absolute);
    } else if (process.platform === "darwin" || process.platform === "linux") {
        return await (await import("./unix.ts")).NativeLock.open(absolute);
    } else {
        throw new FileSystemError("lock", absolute, "ENOTSUP");
    }
}
