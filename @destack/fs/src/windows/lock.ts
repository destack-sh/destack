import { dlopen } from "bun:ffi";
import { toNamespacedPath } from "node:path";
import { FileSystemError } from "../error/index.ts";

/** The access flags requesting read and write access. */
const GENERIC_READ_WRITE = 0xc0000000;
/** The share flags allowing other handles to read and write. */
const FILE_SHARE_READ_WRITE = 3;
/** The creation disposition opening or creating the file. */
const OPEN_ALWAYS = 4;
/** The attributes of a plain file. */
const FILE_ATTRIBUTE_NORMAL = 0x80;
/** Acquire exclusive byte-range ownership without waiting. */
const LOCK_EXCLUSIVE_NONBLOCKING = 3;
/** The error code for a range locked by another handle. */
const ERROR_LOCK_VIOLATION = 33;
/** The handle value returned when opening fails. */
const INVALID_HANDLE = 0xffffffffffffffffn;
/** Use native handles directly, independently of runtime file-descriptor translation. */
const SYSTEM = dlopen("kernel32.dll", {
    CreateFileW: { args: ["buffer", "u32", "u32", "ptr", "u32", "u32", "ptr"], returns: "u64" },
    LockFileEx: { args: ["u64", "u32", "u32", "u32", "u32", "buffer"], returns: "i32" },
    CloseHandle: { args: ["u64"], returns: "i32" },
    GetLastError: { args: [], returns: "u32" },
});

/** A Windows file handle retained for LockFileEx ownership. */
export class NativeLock {
    /** The lock file's absolute path. */
    readonly path: string;
    /** The open handle that retains ownership. */
    private readonly handle: bigint;

    /** Whether the handle has been closed. */
    private isClosed = false;

    /** Retain the file handle. */
    private constructor(path: string, handle: bigint) {
        this.path = path;
        this.handle = handle;
    }

    /** Open the persistent lock file without truncating it. */
    static async open(path: string): Promise<NativeLock> {
        // open the UTF-16 path with shared access for competing lock holders
        const name = Buffer.from(toNamespacedPath(path) + "\0", "utf16le");
        const handle = SYSTEM.symbols.CreateFileW(
            name,
            GENERIC_READ_WRITE,
            FILE_SHARE_READ_WRITE,
            null,
            OPEN_ALWAYS,
            FILE_ATTRIBUTE_NORMAL,
            null,
        );

        // retain the native failure before issuing another system call
        if (handle === INVALID_HANDLE) {
            throw new FileSystemError("open", path, SYSTEM.symbols.GetLastError());
        }

        return new NativeLock(path, handle);
    }

    /** Return false only when another handle holds the lock. */
    tryAcquire(): boolean {
        // lock the complete range with zero offsets in the 64-bit OVERLAPPED structure
        const overlapped = new Uint8Array(32);
        if (
            SYSTEM.symbols.LockFileEx(
                this.handle,
                LOCK_EXCLUSIVE_NONBLOCKING,
                0,
                0xffffffff,
                0xffffffff,
                overlapped,
            ) !== 0
        ) {
            return true;
        }

        // distinguish contention from failed filesystem operations
        const code = SYSTEM.symbols.GetLastError();
        if (code === ERROR_LOCK_VIOLATION) {
            return false;
        }
        throw new FileSystemError("lock", this.path, code);
    }

    /** Close the handle and release its lock. */
    async close(): Promise<void> {
        // close the handle once across repeated calls
        if (this.isClosed) {
            return;
        }

        if (SYSTEM.symbols.CloseHandle(this.handle) === 0) {
            throw new FileSystemError("close", this.path, SYSTEM.symbols.GetLastError());
        }

        this.isClosed = true;
    }
}
