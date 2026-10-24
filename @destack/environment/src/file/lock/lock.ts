import { resolve } from "node:path";
import { setTimeout } from "node:timers/promises";
import { FileSystemError } from "../error.ts";
import type { FileLock, FileLockMode } from "../file.ts";

/** Delay between nonblocking operating-system lock attempts. */
const POLL_INTERVAL = 25;

/** An operating-system file lock on Bun, held shared or exclusive. */
export class BunFileLock implements FileLock {
    /** The locked file's absolute path. */
    readonly path: string;
    /** The retained operating-system file. */
    readonly #native: NativeLock;
    /** How the lock is held now. */
    #mode: FileLockMode;

    /** Retain the open file until explicitly released. */
    private constructor(path: string, native: NativeLock, mode: FileLockMode) {
        this.path = path;
        this.#native = native;
        this.#mode = mode;
    }

    /** How the lock is held now. */
    get mode(): FileLockMode {
        return this.#mode;
    }

    /** Open or create a file and wait for exclusive access. */
    static async acquire(path: string, options: FileLockOptions = {}): Promise<BunFileLock> {
        return await BunFileLock.#acquire(path, "exclusive", options);
    }

    /** Open or create a file and wait for access shared with other shared holders. */
    static async acquireShared(path: string, options: FileLockOptions = {}): Promise<BunFileLock> {
        return await BunFileLock.#acquire(path, "shared", options);
    }

    /** Open or create a file and wait to lock it in a mode. */
    static async #acquire(
        path: string,
        mode: FileLockMode,
        options: FileLockOptions,
    ): Promise<BunFileLock> {
        // open the file before attempting ownership
        options.signal?.throwIfAborted();
        const native = await open(path);

        try {
            // poll native ownership with cancellable waits
            while (true) {
                options.signal?.throwIfAborted();
                if (native.tryAcquire(mode)) {
                    return new BunFileLock(native.path, native, mode);
                }

                await setTimeout(POLL_INTERVAL, undefined, { signal: options.signal });
            }
        } catch (error) {
            await native.close();

            throw error;
        }
    }

    /** Lock a file in a mode now, absent on contention, throwing on a filesystem error. */
    static async tryAcquire(path: string, mode: FileLockMode): Promise<BunFileLock | undefined> {
        // retain the file only after acquiring ownership
        const native = await open(path);
        let isAcquired = false;
        try {
            isAcquired = native.tryAcquire(mode);
            if (isAcquired) {
                return new BunFileLock(native.path, native, mode);
            }
        } finally {
            if (!isAcquired) {
                await native.close();
            }
        }

        return undefined;
    }

    /** Convert a held lock to exclusive, else keep it shared while another holder has it. */
    async tryExclusive(): Promise<boolean> {
        // convert, or hold it shared again, since a failed conversion may have released it
        if (this.#native.tryAcquire("exclusive")) {
            this.#mode = "exclusive";

            return true;
        }
        await this.#hold("shared");

        return false;
    }

    /** Convert a held lock back to shared, waiting while another holder takes it meanwhile. */
    async share(): Promise<void> {
        await this.#hold("shared");
    }

    /** Wait until the file description holds the lock in a mode. */
    async #hold(mode: FileLockMode): Promise<void> {
        // poll, since flock and LockFileEx convert by releasing first
        while (!this.#native.tryAcquire(mode)) {
            await setTimeout(POLL_INTERVAL);
        }
        this.#mode = mode;
    }

    /** Release the lock and close the file, leaving its directory entry intact. */
    async close(): Promise<void> {
        await this.#native.close();
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
        throw FileSystemError.of("FILE_SYSTEM_FAILED", `cannot open ${absolute}: EINVAL`);
    }

    // load only the current platform's system libraries
    if (process.platform === "win32") {
        return await (await import("./windows.ts")).NativeLock.open(absolute);
    } else if (process.platform === "darwin" || process.platform === "linux") {
        return await (await import("./unix.ts")).NativeLock.open(absolute);
    } else {
        throw FileSystemError.of("FILE_SYSTEM_FAILED", `cannot lock ${absolute}: ENOTSUP`);
    }
}
