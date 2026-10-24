/** What a path points at. */
export type FileKind = "file" | "directory" | "symlink" | "other";

/** How a file lock is held: shared with other shared holders, or exclusive of every other. */
export type FileLockMode = "shared" | "exclusive";

/** The status of a path, as `stat` reads it. */
export interface FileStatus {
    /** What the path points at. */
    readonly kind: FileKind;
    /** The size in bytes. */
    readonly size: number;
    /** When the contents last changed, in UTC epoch milliseconds. */
    readonly modifiedAt: number;
    /** The permission bits and file type, as POSIX `st_mode` has them. */
    readonly mode: number;
}

/** An entry of a directory. */
export interface DirectoryEntry {
    /** The entry's path relative to the listed directory, its name unless listed recursively. */
    readonly name: string;
    /** What the entry points at. */
    readonly kind: FileKind;
}

/** A change a watch reports. */
export interface FileEvent {
    /** What changed, as Deno's `FsEvent` has it: a path that appeared, changed or went away. */
    readonly kind: "create" | "modify" | "remove";
    /** The changed path, relative to the watched one. */
    readonly path: string;
}

/** How a file is written. */
export interface WriteFileOptions {
    /** Whether to add to the end of the file instead of replacing it. */
    readonly append?: boolean;
    /** Whether to refuse a path that exists already. */
    readonly createNew?: boolean;
    /** The permission bits of a created file. */
    readonly mode?: number;
}

/** How a directory is listed. */
export interface ReadDirectoryOptions {
    /** Whether to list the entries inside subdirectories too. */
    readonly recursive?: boolean;
}

/** Where a temporary directory is created. */
export interface TemporaryDirectoryOptions {
    /** The directory to create it in, the system's temporary directory by default. */
    readonly directory?: string;
}

/** How a directory is created. */
export interface MkdirOptions {
    /** Whether to create missing parents, and accept an existing directory. */
    readonly recursive?: boolean;
    /** The permission bits of created directories. */
    readonly mode?: number;
}

/** How a path is removed. */
export interface RemoveOptions {
    /** Whether to remove a directory with everything inside it. */
    readonly recursive?: boolean;
    /** Whether removing a missing path succeeds. */
    readonly force?: boolean;
}

/** How a file is opened. */
export interface OpenOptions {
    /** Whether to read it. */
    readonly read?: boolean;
    /** Whether to write it. */
    readonly write?: boolean;
    /** Whether writes go to the end. */
    readonly append?: boolean;
    /** Whether to empty it as it opens. */
    readonly truncate?: boolean;
    /** Whether to create it when missing. */
    readonly create?: boolean;
    /** Whether to refuse a path that exists already. */
    readonly createNew?: boolean;
    /** The permission bits of a created file. */
    readonly mode?: number;
}

/** How a path is watched. */
export interface WatchOptions {
    /** Whether to report changes inside subdirectories. */
    readonly recursive?: boolean;
    /** Stop watching once it aborts. */
    readonly signal?: AbortSignal;
}

/** An open file, read and written as web streams at its cursor, as Deno's `FsFile` has it. */
export interface FileHandle extends AsyncDisposable {
    /** The file's bytes from its cursor on, moving the cursor. */
    readonly readable: () => ReadableStream<Uint8Array>;
    /** A stream writing bytes at the file's cursor, moving the cursor. */
    readonly writable: () => WritableStream<Uint8Array>;
    /** Read bytes at the cursor into a buffer, answering their count, or null at the file's end. */
    readonly read: (buffer: Uint8Array) => Promise<number | null>;
    /** Move the cursor to a byte offset from the file's start, answering the offset. */
    readonly seek: (offset: number) => Promise<number>;
    /** Read the file's status. */
    readonly stat: () => Promise<FileStatus>;
    /** Flush the file's contents and status to the disk. */
    readonly sync: () => Promise<void>;
    /** Cut or extend the file to a length, empty by default. */
    readonly truncate: (length?: number) => Promise<void>;
    /** Close the file. */
    readonly close: () => Promise<void>;
}

/** An operating-system lock on a file, released on close. */
export interface FileLock extends AsyncDisposable {
    /** How the lock is held now. */
    readonly mode: FileLockMode;
    /** Convert a held lock to exclusive, else keep it shared while another holder has it. */
    readonly tryExclusive: () => Promise<boolean>;
    /** Convert a held lock back to shared. */
    readonly share: () => Promise<void>;
    /** Release the lock and close the file, leaving its directory entry intact. */
    readonly close: () => Promise<void>;
}

/** The files a process reads and writes, as Deno's file system API has them. */
export interface FileSystem {
    /** Read a file's bytes. */
    readonly readFile: (path: string) => Promise<Uint8Array<ArrayBuffer>>;
    /** Read a file's text as UTF-8. */
    readonly readTextFile: (path: string) => Promise<string>;
    /** Write bytes to a file. */
    readonly writeFile: (
        path: string,
        data: Uint8Array,
        options?: WriteFileOptions,
    ) => Promise<void>;
    /** Write text to a file as UTF-8. */
    readonly writeTextFile: (
        path: string,
        text: string,
        options?: WriteFileOptions,
    ) => Promise<void>;
    /** Read whether a path exists. */
    readonly exists: (path: string) => Promise<boolean>;
    /** Read a path's status, following links. */
    readonly stat: (path: string) => Promise<FileStatus>;
    /** Read a path's status without following a link. */
    readonly lstat: (path: string) => Promise<FileStatus>;
    /** Read the absolute path a path resolves to, its links followed. */
    readonly realPath: (path: string) => Promise<string>;
    /** List a directory's entries. */
    readonly readDirectory: (
        path: string,
        options?: ReadDirectoryOptions,
    ) => Promise<DirectoryEntry[]>;

    /** Create a directory. */
    readonly mkdir: (path: string, options?: MkdirOptions) => Promise<void>;
    /** Create a new empty directory with a prefix, in the temporary directory by default. */
    readonly makeTemporaryDirectory: (
        prefix: string,
        options?: TemporaryDirectoryOptions,
    ) => Promise<string>;
    /** Remove a path. */
    readonly remove: (path: string, options?: RemoveOptions) => Promise<void>;
    /** Move a path to another, replacing a file there. */
    readonly rename: (from: string, to: string) => Promise<void>;
    /** Copy a file to another path, cloning its blocks where the file system can. */
    readonly copyFile: (from: string, to: string) => Promise<void>;
    /** Copy a path to another, with everything inside a directory. */
    readonly copy: (from: string, to: string) => Promise<void>;
    /** Create a symbolic link at a path to a target. */
    readonly symlink: (target: string, path: string) => Promise<void>;
    /** Read a symbolic link's target. */
    readonly readLink: (path: string) => Promise<string>;
    /** Create a hard link at a path to a file. */
    readonly link: (target: string, path: string) => Promise<void>;
    /** Change a path's permission bits. */
    readonly chmod: (path: string, mode: number) => Promise<void>;
    /** Open a file. */
    readonly open: (path: string, options: OpenOptions) => Promise<FileHandle>;
    /** Report the changes at a path until the signal aborts. */
    readonly watch: (path: string, options?: WatchOptions) => AsyncIterable<FileEvent>;
    /** Find the private directory of the person's runtime files, such as sockets and locks. */
    readonly runtimeDirectory: () => Promise<string>;
    /** Open or create a file and wait to lock it in a mode. */
    readonly lock: (
        path: string,
        mode: FileLockMode,
        options?: { readonly signal?: AbortSignal },
    ) => Promise<FileLock>;
    /** Lock a file in a mode now, absent while another holder has it. */
    readonly tryLock: (path: string, mode: FileLockMode) => Promise<FileLock | undefined>;
}
