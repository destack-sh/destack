import { constants, type Dirent, type Stats, watch } from "node:fs";
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
    rmdir,
    stat,
    symlink,
    writeFile,
} from "node:fs/promises";
import { homedir, tmpdir } from "node:os";
import { join, relative } from "node:path";
import {
    DirectoryEntry,
    type FileEvent,
    type FileHandle,
    type FileStatus,
    type FileKind,
    type FileLock,
    type FileLockMode,
    type FileSystem,
    type MkdirOptions,
    type ReadDirectoryOptions,
    type TemporaryDirectoryOptions,
    type OpenOptions,
    type RemoveOptions,
    type WatchOptions,
    type WriteFileOptions,
} from "./file.ts";
import { FileSystemError } from "./error.ts";
import { BunFileLock } from "./lock/lock.ts";

/** The permission bits of a directory only its owner opens. */
const PRIVATE_MODE = 0o700;

/** The permission bits that share a directory with the group or others. */
const SHARED_MODE = 0o077;

/** The bytes one read of an open file takes, as Node's file streams do: 64 KiB. */
const CHUNK_BYTES = 64 * 1024;

/** The name prefix of the hidden file a watch writes to learn its host watcher is armed. */
const WATCH_PROBE_PREFIX = ".destack-watch-";

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
        // answer false for a missing path
        try {
            await lstat(path);

            return true;
        } catch (error) {
            if (error instanceof Error && "code" in error && error.code === "ENOENT") {
                return false;
            }
            throw BunFileSystem.refusal(error, `read ${path}`);
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

    /** Join a directory and the names below it into one path, in the platform's separators. */
    join(directory: string, ...names: readonly string[]): string {
        return join(directory, ...names);
    }

    /** List a directory's entries. */
    readDirectory(path: string, options: ReadDirectoryOptions = {}): Promise<DirectoryEntry[]> {
        return BunFileSystem.#run(`list ${path}`, async () => {
            const entries = await readdir(path, {
                withFileTypes: true,
                recursive: options.recursive === true,
            });

            return entries
                .map((entry) => ({
                    name: relative(path, join(entry.parentPath, entry.name)),
                    kind: BunFileSystem.#kind(entry),
                }))
                .toSorted((left, right) => DirectoryEntry.compare(left, right));
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

    /** Remove a path, an empty directory without recursion, as Deno's `remove` does. */
    remove(path: string, options: RemoveOptions = {}): Promise<void> {
        return BunFileSystem.#run(`remove ${path}`, async () => {
            // remove a directory without recursion only when empty
            const recursive = options.recursive ?? false;
            const force = options.force ?? false;
            const status = recursive
                ? undefined
                : await lstat(path).catch((error: unknown) => {
                      if (
                          force &&
                          error instanceof Error &&
                          "code" in error &&
                          error.code === "ENOENT"
                      ) {
                          return undefined;
                      }
                      throw error;
                  });
            if (status?.isDirectory() === true) {
                await rmdir(path);
            }
            // remove anything else as rm does
            else {
                await rm(path, { recursive, force });
            }
        });
    }

    /** Move a path to another, replacing a file there. */
    rename(from: string, to: string): Promise<void> {
        return BunFileSystem.#run(`move ${from} to ${to}`, () => rename(from, to));
    }

    /** Copy a file to another path, cloning its blocks where the file system can. */
    copyFile(from: string, to: string): Promise<void> {
        return BunFileSystem.#run(`copy ${from} to ${to}`, () =>
            copyFile(from, to, constants.COPYFILE_FICLONE),
        );
    }

    /** Copy a path to another, with everything inside a directory, copying links as written. */
    copy(from: string, to: string): Promise<void> {
        // clone each file copy-on-write where the volume can, keeping links as written
        return BunFileSystem.#run(`copy ${from} to ${to}`, () =>
            cp(from, to, {
                recursive: true,
                mode: constants.COPYFILE_FICLONE,
                verbatimSymlinks: true,
            }),
        );
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
        // open the file in the flags its options choose, reopening a directory on Windows with the write access its flush needs
        const opened = await BunFileSystem.#run(`open ${path}`, () =>
            open(path, BunFileSystem.#flags(options), options.mode),
        );
        const isWindowsDirectory =
            process.platform === "win32" &&
            options.write !== true &&
            (await opened.stat()).isDirectory();
        if (isWindowsDirectory) {
            await opened.close();
        }
        const file = isWindowsDirectory
            ? await BunFileSystem.#run(`open ${path}`, () =>
                  open(path, BunFileSystem.#flags({ ...options, write: true }), options.mode),
              )
            : opened;

        // read and write at one cursor, as Deno's FsFile does, appending writes at the end
        let cursor = 0;
        const isAppending = options.append === true;
        const read = async (buffer: Uint8Array): Promise<number> => {
            // read at the cursor, moving it unless a seek moved it meanwhile
            const position = cursor;
            const { bytesRead } = await file.read(buffer, 0, buffer.byteLength, position);
            cursor = cursor === position ? position + bytesRead : cursor;

            return bytesRead;
        };

        return {
            readable: () =>
                new ReadableStream<Uint8Array>({
                    pull: async (controller) => {
                        // read the next chunk at the cursor, closing at the end
                        const chunk = new Uint8Array(CHUNK_BYTES);
                        const bytesRead = await read(chunk);
                        if (bytesRead === 0) {
                            controller.close();
                        } else {
                            controller.enqueue(chunk.subarray(0, bytesRead));
                        }
                    },
                }),
            writable: () =>
                new WritableStream<Uint8Array>({
                    write: async (chunk) => {
                        // write at the cursor, or at the end while appending, which moves the cursor there
                        const position = cursor;
                        const { bytesWritten } = await file.write(
                            chunk,
                            0,
                            chunk.byteLength,
                            isAppending ? null : position,
                        );
                        const end = isAppending
                            ? (await file.stat()).size
                            : position + bytesWritten;
                        cursor = cursor === position ? end : cursor;
                    },
                }),
            read: async (buffer) => {
                const bytesRead = await read(buffer);

                return bytesRead === 0 && buffer.byteLength > 0 ? null : bytesRead;
            },
            seek: async (offset) => {
                cursor = offset;

                return cursor;
            },
            stat: async () => BunFileSystem.#status(await file.stat()),
            sync: () => file.sync(),
            truncate: (length) => file.truncate(length),
            close: () => file.close(),
            [Symbol.asyncDispose]: () => file.close(),
        };
    }

    /** Watch a path, resolving once its host watcher is armed, until the signal aborts. */
    async watch(path: string, options: WatchOptions = {}): Promise<AsyncIterable<FileEvent>> {
        // start the host's watcher, closing it when arming it fails
        const recursive = options.recursive ?? false;
        const watcher = BunFileSystem.#listen(path, recursive, options.signal);
        try {
            // write a probe and pass the names reported before its own, which the host reports once armed
            const probe = `${WATCH_PROBE_PREFIX}${crypto.randomUUID()}`;
            const names = watcher.names[Symbol.asyncIterator]();
            await this.writeFile(join(path, probe), new Uint8Array());
            let next = await names.next();
            while (next.done !== true && next.value !== probe) {
                next = await names.next();
            }

            // remove the probe, then list the names whose later changes the armed watcher reports
            await this.remove(join(path, probe));
            const entries = await this.readDirectory(path, { recursive });
            const reported = BunFileSystem.#without(probe, names);

            return this.#watched(path, entries, reported, watcher.close);
        } catch (error) {
            watcher.close();
            throw error;
        }
    }

    /** Start the host's watcher of a path at once, buffering its changes until read. */
    static #listen(
        path: string,
        recursive: boolean,
        signal: AbortSignal | undefined,
    ): { readonly names: AsyncIterable<string | null>; readonly close: () => void } {
        // buffer each changed name, failure and close the watcher reports
        const changes: { readonly name: string | null }[] = [];
        let failure: Error | undefined;
        let isClosed = signal?.aborted === true;
        let wake: (() => void) | undefined;
        const watcher = watch(path, { recursive, encoding: "utf8" }, (_, filename) => {
            changes.push({ name: filename });
            wake?.();
        });
        watcher.on("error", (error: Error) => {
            failure = error;
            wake?.();
        });

        // stop the watcher once closed
        const close = () => {
            // mark it closed, stop listening and wake a waiting reader
            isClosed = true;
            signal?.removeEventListener("abort", close);
            watcher.close();
            wake?.();
        };

        // close on abort, and at once for an aborted signal
        signal?.addEventListener("abort", close, { once: true });
        if (isClosed) {
            close();
        }

        // wait for the next report, as the reader does once the buffer runs dry
        const changed = () =>
            new Promise<void>((resolve) => {
                wake = resolve;
            });

        // read the buffered names as they come
        const names = (async function* () {
            // yield each buffered change, then fail, end or wait for the next
            for (;;) {
                const change = changes.shift();
                if (change !== undefined) {
                    yield change.name;
                } else if (failure !== undefined) {
                    throw failure;
                } else if (isClosed) {
                    return;
                } else {
                    await changed();
                    wake = undefined;
                }
            }
        })();

        return { names, close };
    }

    /** Read a watch's changes, closing its watcher however the reading ends. */
    async *#watched(
        path: string,
        entries: readonly DirectoryEntry[],
        names: AsyncIterable<string | null>,
        close: () => void,
    ): AsyncIterable<FileEvent> {
        try {
            yield* this.#classified(path, entries, names);
        } finally {
            close();
        }
    }

    /** Read the reported names as they come, leaving out a probe's. */
    static async *#without(
        probe: string,
        names: AsyncIterator<string | null>,
    ): AsyncIterable<string | null> {
        // yield each name but the probe's
        for (let next = await names.next(); next.done !== true; next = await names.next()) {
            if (next.value !== probe) {
                yield next.value;
            }
        }
    }

    /** Classify a watch's changed names as created, modified or removed by the names it knows. */
    async *#classified(
        path: string,
        entries: readonly DirectoryEntry[],
        names: AsyncIterable<string | null>,
    ): AsyncIterable<FileEvent> {
        // know the names present at the start
        const known = new Set(entries.map((entry) => entry.name));

        // read each change until the watcher closes
        for await (const name of names) {
            // refuse a host that reports a change without its path
            if (name === null) {
                throw new TypeError(`the host reported a change under '${path}' without its path`);
            }

            // classify the change by the names known, a removal once, as a host may report it twice
            const isPresent = await this.exists(join(path, name));
            if (!isPresent) {
                if (known.delete(name)) {
                    yield { kind: "remove", path: name };
                }
            } else if (known.has(name)) {
                yield { kind: "modify", path: name };
            } else {
                known.add(name);
                yield { kind: "create", path: name };
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

    /** Find the person's cache directory: XDG_CACHE_HOME, else the platform's own. */
    async cacheDirectory(): Promise<string> {
        return BunFileSystem.cacheDirectoryOf(process.env);
    }

    /** Find the person's cache directory from their variables, XDG_CACHE_HOME first. */
    static cacheDirectoryOf(variables: Readonly<Record<string, string | undefined>>): string {
        // take the XDG directory when set, else the platform's caches
        const named = variables["XDG_CACHE_HOME"];
        const local = variables["LOCALAPPDATA"];
        if (named !== undefined && named !== "") {
            return named;
        } else if (process.platform === "darwin") {
            return join(homedir(), "Library", "Caches");
        } else if (process.platform === "win32" && local !== undefined && local !== "") {
            return local;
        }

        return join(homedir(), ".cache");
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
                "FILE_SYSTEM_BROKEN",
                `cannot trust runtime directory '${directory}': EACCES`,
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

    /** Lock a file in a mode now, absent while another holder has it. */
    async tryLock(path: string, mode: FileLockMode): Promise<FileLock | undefined> {
        return await BunFileLock.tryAcquire(path, mode);
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
        return FileSystemError.posix(error.code, action, error);
    }
}
