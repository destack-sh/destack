import { constants, type Dirent, type Stats } from "node:fs";
import {
    chmod,
    copyFile,
    cp,
    link,
    lstat,
    mkdir,
    mkdtemp,
    open,
    readdir,
    readFile,
    readlink,
    realpath,
    rename,
    rm,
    stat,
    symlink,
    watch,
    writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, relative } from "node:path";
import type {
    DirectoryEntry,
    FileEvent,
    FileHandle,
    FileStatus,
    FileKind,
    FileLock,
    FileLockMode,
    FileSystem,
    MkdirOptions,
    ReadDirectoryOptions,
    TemporaryDirectoryOptions,
    OpenOptions,
    RemoveOptions,
    WatchOptions,
    WriteFileOptions,
} from "./file.ts";
import { FileSystemError } from "./error.ts";
import { BunFileLock } from "./lock/lock.ts";

/** The permission bits of a directory only its owner opens. */
const PRIVATE_MODE = 0o700;

/** The permission bits that share a directory with the group or others. */
const SHARED_MODE = 0o077;

/** The bytes one read of an open file takes, as Node's file streams do: 64 KiB. */
const CHUNK_BYTES = 64 * 1024;

/** The files of the machine Bun runs on, through Node's file system API. */
export class BunFileSystem implements FileSystem {
    /** Read a file's bytes. */
    readFile(path: string): Promise<Uint8Array<ArrayBuffer>> {
        return BunFileSystem.#run(`read ${path}`, async () => {
            // view the read buffer's bytes without copying them
            const bytes = await readFile(path);

            return new Uint8Array(bytes.buffer, bytes.byteOffset, bytes.byteLength);
        });
    }

    /** Read a file's text as UTF-8. */
    readTextFile(path: string): Promise<string> {
        return BunFileSystem.#run(`read ${path}`, () => readFile(path, "utf8"));
    }

    /** Write bytes to a file. */
    writeFile(path: string, data: Uint8Array, options: WriteFileOptions = {}): Promise<void> {
        return BunFileSystem.#run(`write ${path}`, () =>
            writeFile(path, data, BunFileSystem.#writing(options)),
        );
    }

    /** Write text to a file as UTF-8. */
    writeTextFile(path: string, text: string, options: WriteFileOptions = {}): Promise<void> {
        return BunFileSystem.#run(`write ${path}`, () =>
            writeFile(path, text, { ...BunFileSystem.#writing(options), encoding: "utf8" }),
        );
    }

    /** Read whether a path exists. */
    async exists(path: string): Promise<boolean> {
        // answer false only for a missing path
        try {
            await this.lstat(path);

            return true;
        } catch (error) {
            if (FileSystemError.is(error, "NOT_FOUND")) {
                return false;
            }
            throw error;
        }
    }

    /** Read a path's status, following links. */
    stat(path: string): Promise<FileStatus> {
        return BunFileSystem.#run(`read ${path}`, async () =>
            BunFileSystem.#status(await stat(path)),
        );
    }

    /** Read a path's status without following a link. */
    lstat(path: string): Promise<FileStatus> {
        return BunFileSystem.#run(`read ${path}`, async () =>
            BunFileSystem.#status(await lstat(path)),
        );
    }

    /** Read the absolute path a path resolves to, its links followed. */
    realPath(path: string): Promise<string> {
        return BunFileSystem.#run(`resolve ${path}`, () => realpath(path));
    }

    /** List a directory's entries. */
    readDirectory(path: string, options: ReadDirectoryOptions = {}): Promise<DirectoryEntry[]> {
        return BunFileSystem.#run(`list ${path}`, async () => {
            const entries = await readdir(path, {
                withFileTypes: true,
                recursive: options.recursive === true,
            });

            return entries.map((entry) => ({
                name: relative(path, join(entry.parentPath, entry.name)),
                kind: BunFileSystem.#kind(entry),
            }));
        });
    }

    /** Create a directory. */
    mkdir(path: string, options: MkdirOptions = {}): Promise<void> {
        return BunFileSystem.#run(`create ${path}`, async () => {
            await mkdir(path, options);
        });
    }

    /** Create a new empty directory with a prefix, in the temporary directory by default. */
    makeTemporaryDirectory(
        prefix: string,
        options: TemporaryDirectoryOptions = {},
    ): Promise<string> {
        const directory = options.directory ?? tmpdir();

        return BunFileSystem.#run(`create ${prefix} in ${directory}`, () =>
            mkdtemp(join(directory, prefix)),
        );
    }

    /** Remove a path. */
    remove(path: string, options: RemoveOptions = {}): Promise<void> {
        return BunFileSystem.#run(`remove ${path}`, () =>
            rm(path, { recursive: options.recursive ?? false, force: options.force ?? false }),
        );
    }

    /** Move a path to another, replacing a file there. */
    rename(from: string, to: string): Promise<void> {
        return BunFileSystem.#run(`move ${from} to ${to}`, () => rename(from, to));
    }

    /** Copy a file to another path. */
    copyFile(from: string, to: string): Promise<void> {
        return BunFileSystem.#run(`copy ${from} to ${to}`, () => copyFile(from, to));
    }

    /** Copy a path to another, with everything inside a directory. */
    copy(from: string, to: string): Promise<void> {
        return BunFileSystem.#run(`copy ${from} to ${to}`, () => cp(from, to, { recursive: true }));
    }

    /** Create a symbolic link at a path to a target. */
    symlink(target: string, path: string): Promise<void> {
        return BunFileSystem.#run(`link ${path} to ${target}`, () => symlink(target, path));
    }

    /** Read a symbolic link's target. */
    readLink(path: string): Promise<string> {
        return BunFileSystem.#run(`read ${path}`, () => readlink(path));
    }

    /** Create a hard link at a path to a file. */
    link(target: string, path: string): Promise<void> {
        return BunFileSystem.#run(`link ${path} to ${target}`, () => link(target, path));
    }

    /** Change a path's permission bits. */
    chmod(path: string, mode: number): Promise<void> {
        return BunFileSystem.#run(`change ${path}`, () => chmod(path, mode));
    }

    /** Open a file. */
    async open(path: string, options: OpenOptions): Promise<FileHandle> {
        // open the file in the flags its options choose
        const file = await BunFileSystem.#run(`open ${path}`, () =>
            open(path, BunFileSystem.#flags(options), options.mode),
        );

        return {
            readable: () => {
                // read chunks from the start on, ending at the file's end
                let position = 0;

                return new ReadableStream<Uint8Array>({
                    pull: async (controller) => {
                        // read the next chunk, closing at the end
                        const chunk = new Uint8Array(CHUNK_BYTES);
                        const { bytesRead } = await file.read(chunk, 0, CHUNK_BYTES, position);
                        position += bytesRead;
                        if (bytesRead === 0) {
                            controller.close();
                        } else {
                            controller.enqueue(chunk.subarray(0, bytesRead));
                        }
                    },
                });
            },
            writable: () =>
                new WritableStream<Uint8Array>({
                    write: async (chunk) => {
                        await file.write(chunk);
                    },
                }),
            stat: async () => BunFileSystem.#status(await file.stat()),
            sync: () => file.sync(),
            truncate: (length) => file.truncate(length),
            close: () => file.close(),
            [Symbol.asyncDispose]: () => file.close(),
        };
    }

    /** Watch a path's changes from the call on, until the signal aborts. */
    watch(path: string, options: WatchOptions = {}): AsyncIterable<FileEvent> {
        // list the names and start watching at once
        const recursive = options.recursive ?? false;
        const listing = this.readDirectory(path, { recursive });
        const events = watch(path, {
            recursive,
            ...(options.signal === undefined ? {} : { signal: options.signal }),
        });

        return this.#watched(path, listing, events);
    }

    /** Read a watch's changes as created, modified or removed against the names it knows. */
    async *#watched(
        path: string,
        listing: Promise<readonly DirectoryEntry[]>,
        events: AsyncIterable<{ readonly eventType: string; readonly filename: string | null }>,
    ): AsyncIterable<FileEvent> {
        // know the names present at the start
        const known = new Set((await listing).map((entry) => entry.name));

        // read each change until the signal aborts
        try {
            for await (const event of events) {
                // refuse a host that reports a change without its path
                const name = event.filename;
                if (name === null) {
                    throw new TypeError(
                        `the host reported a change under ${path} without its path`,
                    );
                }

                // classify the change by the names known
                const isPresent = await this.exists(join(path, name));
                if (!isPresent) {
                    known.delete(name);
                    yield { kind: "remove", path: name };
                } else if (known.has(name)) {
                    yield { kind: "modify", path: name };
                } else {
                    known.add(name);
                    yield { kind: "create", path: name };
                }
            }
        } catch (error) {
            if (!(error instanceof Error) || error.name !== "AbortError") {
                throw error;
            }
        }
    }

    /** Find the session's runtime directory, else a private per-user one. */
    async runtimeDirectory(): Promise<string> {
        // take the session's runtime directory, else the temporary one where users have no identifiers
        const session = process.env["XDG_RUNTIME_DIR"];
        const user = process.getuid?.();
        if (session !== undefined && session !== "") {
            return session;
        } else if (user === undefined) {
            return tmpdir();
        }

        return BunFileSystem.trust(join(tmpdir(), `destack-${user}`), user);
    }

    /** Create a user's private directory once, refusing a link or a directory others may open. */
    static async trust(directory: string, user: number): Promise<string> {
        // create it with mode 0700 unless it exists, and read it without following a link
        const status = await BunFileSystem.#run(`create ${directory}`, async () => {
            await mkdir(directory, { mode: PRIVATE_MODE, recursive: true });

            return lstat(directory);
        });

        // refuse a link, another user's directory, and one the group or others may open
        if (!status.isDirectory() || status.uid !== user || (status.mode & SHARED_MODE) !== 0) {
            throw FileSystemError.of(
                "FILE_SYSTEM_FAILED",
                `cannot trust runtime directory ${directory}: EACCES`,
            );
        }

        return directory;
    }

    /** Open or create a file and wait to lock it in a mode. */
    async lock(
        path: string,
        mode: FileLockMode,
        options: { readonly signal?: AbortSignal } = {},
    ): Promise<FileLock> {
        return mode === "exclusive"
            ? await BunFileLock.acquire(path, options)
            : await BunFileLock.acquireShared(path, options);
    }

    /** Open or create a file and lock it exclusively, absent while another holder has it. */
    async tryLock(path: string): Promise<FileLock | undefined> {
        return await BunFileLock.tryAcquire(path);
    }

    /** Read the POSIX open flags of a file's open options. */
    static #flags(options: OpenOptions): number {
        // choose the access, then add each requested flag
        const isWriting = options.write === true || options.append === true;
        const access =
            options.read === true && isWriting
                ? constants.O_RDWR
                : isWriting
                  ? constants.O_WRONLY
                  : constants.O_RDONLY;
        const requested = [
            [options.append, constants.O_APPEND],
            [options.truncate, constants.O_TRUNC],
            [options.create, constants.O_CREAT],
            [options.createNew, constants.O_CREAT | constants.O_EXCL],
        ] as const;

        return requested.reduce(
            (flags, [isRequested, flag]) => (isRequested === true ? flags | flag : flags),
            access,
        );
    }

    /** Read the Node options of a write. */
    static #writing(options: WriteFileOptions) {
        return {
            flag: `${options.append === true ? "a" : "w"}${options.createNew === true ? "x" : ""}`,
            ...(options.mode === undefined ? {} : { mode: options.mode }),
        };
    }

    /** Read a host status as a file status. */
    static #status(stats: Stats): FileStatus {
        return {
            kind: BunFileSystem.#kind(stats),
            size: stats.size,
            modifiedAt: stats.mtimeMs,
            mode: stats.mode,
        };
    }

    /** Read what a status or directory entry points at. */
    static #kind(entry: Stats | Dirent): FileKind {
        if (entry.isFile()) {
            return "file";
        } else if (entry.isDirectory()) {
            return "directory";
        } else if (entry.isSymbolicLink()) {
            return "symlink";
        } else {
            return "other";
        }
    }

    /** Run a host operation, its failures refused as the file system's. */
    static async #run<Value>(action: string, operation: () => Promise<Value>): Promise<Value> {
        try {
            return await operation();
        } catch (error) {
            throw BunFileSystem.refusal(error, action);
        }
    }

    /** Read a host failure as the file system's failure. */
    static refusal(error: unknown, action: string): unknown {
        // pass on a failure without an error code
        if (!(error instanceof Error) || !("code" in error) || typeof error.code !== "string") {
            return error;
        }

        // refuse by the error code, keeping the host's failure as the cause
        const message = `cannot ${action}: ${error.code}`;
        if (error.code === "ENOENT" || error.code === "ENOTDIR") {
            return FileSystemError.of("NOT_FOUND", message, { cause: error });
        } else if (error.code === "EEXIST" || error.code === "ENOTEMPTY") {
            return FileSystemError.of("ALREADY_EXISTS", message, { cause: error });
        } else if (error.code === "EACCES" || error.code === "EPERM") {
            return FileSystemError.of("PERMISSION_DENIED", message, { cause: error });
        } else {
            return FileSystemError.of("FILE_SYSTEM_FAILED", message, { cause: error });
        }
    }
}
