import { open, type FileHandle } from "node:fs/promises";
import { dlopen, read } from "bun:ffi";
import { FileSystemError } from "../error/index.ts";

/** Exclusive nonblocking flock flags on Darwin and Linux. */
const LOCK_EXCLUSIVE_NONBLOCKING = 2 | 4;
/** EWOULDBLOCK identifies an existing flock owner. */
const WOULD_BLOCK = process.platform === "darwin" ? 35 : 11;
/** Bind libc once for all locks in this process. */
const SYSTEM =
    process.platform === "darwin"
        ? dlopen("/usr/lib/libSystem.B.dylib", {
              flock: { args: ["i32", "i32"], returns: "i32" },
              __error: { args: [], returns: "ptr" },
          })
        : dlopen("libc.so.6", {
              flock: { args: ["i32", "i32"], returns: "i32" },
              __errno_location: { args: [], returns: "ptr" },
          });
/** Retain this runtime thread's errno address before issuing lock calls. */
const ERRNO =
    "__error" in SYSTEM.symbols ? SYSTEM.symbols.__error()! : SYSTEM.symbols.__errno_location()!;

/** A Unix file description retained for flock ownership. */
export class NativeLock {
    /** The lock file's absolute path. */
    readonly path: string;
    /** The open description that retains ownership. */
    private readonly file: FileHandle;

    /** Share closure across repeated callers. */
    private closing?: Promise<void>;

    /** Retain the file description. */
    private constructor(path: string, file: FileHandle) {
        this.path = path;
        this.file = file;
    }

    /** Open the persistent lock file without truncating it. */
    static async open(path: string): Promise<NativeLock> {
        try {
            const file = await open(path, "a+", 0o600);

            return new NativeLock(path, file);
        } catch (cause) {
            // retain the original filesystem error as the cause
            const code = (cause as NodeJS.ErrnoException).code;
            if (!code) {
                throw cause;
            }

            throw new FileSystemError("open", path, code, { cause });
        }
    }

    /** Return false only when another file description holds the lock. */
    tryAcquire(): boolean {
        // acquire exclusive ownership on the open file description
        if (SYSTEM.symbols.flock(this.file.fd, LOCK_EXCLUSIVE_NONBLOCKING) === 0) {
            return true;
        }

        // distinguish contention from failed filesystem operations
        const code = read.i32(ERRNO);
        if (code === WOULD_BLOCK) {
            return false;
        }
        throw new FileSystemError("lock", this.path, code);
    }

    /** Close the file description and release its lock. */
    close(): Promise<void> {
        return (this.closing ??= this.file.close());
    }
}
