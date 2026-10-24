import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { BunFileSystem } from "./bun.ts";
import type { Clock } from "../clock/clock.ts";
import type {
    DirectoryEntry,
    FileEvent,
    FileHandle,
    FileStatus,
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
import { Random } from "../random/random.ts";

/** The letters of a temporary directory's suffix, as `mkdtemp` draws them. */
const SUFFIX_LETTERS = "abcdefghijklmnopqrstuvwxyz0123456789";

/** The length of a temporary directory's suffix, as `mkdtemp` draws it. */
const SUFFIX_LENGTH = 6;

/** The host's files at their own paths, each operation tracked on the world's clock. */
export class SimulatedFileSystem implements FileSystem {
    /** The world's root, which its temporary directories go under and its dispose removes. */
    readonly root: string;
    /** The host's files. */
    readonly #host = new BunFileSystem();
    /** The world's clock, which counts each operation. */
    readonly #clock: Clock;
    /** The world's randomness, which temporary names draw from. */
    readonly #random: Random;

    /** Track the host's files on the world's clock, its temporary directories under its root. */
    constructor(root: string, clock: Clock, random: Random) {
        this.root = root;
        this.#clock = clock;
        this.#random = random;
    }

    /** Read a file's bytes. */
    readFile(path: string): Promise<Uint8Array<ArrayBuffer>> {
        return this.#clock.trackHost(this.#host.readFile(path));
    }

    /** Read a file's text as UTF-8. */
    readTextFile(path: string): Promise<string> {
        return this.#clock.trackHost(this.#host.readTextFile(path));
    }

    /** Write bytes to a file. */
    writeFile(path: string, data: Uint8Array, options?: WriteFileOptions): Promise<void> {
        return this.#clock.trackHost(this.#host.writeFile(path, data, options));
    }

    /** Write text to a file as UTF-8. */
    writeTextFile(path: string, text: string, options?: WriteFileOptions): Promise<void> {
        return this.#clock.trackHost(this.#host.writeTextFile(path, text, options));
    }

    /** Read whether a path exists. */
    exists(path: string): Promise<boolean> {
        return this.#clock.trackHost(this.#host.exists(path));
    }

    /** Read a path's status, following links. */
    stat(path: string): Promise<FileStatus> {
        return this.#clock.trackHost(this.#host.stat(path));
    }

    /** Read a path's status without following a link. */
    lstat(path: string): Promise<FileStatus> {
        return this.#clock.trackHost(this.#host.lstat(path));
    }

    /** Read the absolute path a path resolves to, its links followed. */
    realPath(path: string): Promise<string> {
        return this.#clock.trackHost(this.#host.realPath(path));
    }

    /** List a directory's entries. */
    readDirectory(path: string, options?: ReadDirectoryOptions): Promise<DirectoryEntry[]> {
        return this.#clock.trackHost(this.#host.readDirectory(path, options));
    }

    /** Create a directory. */
    mkdir(path: string, options?: MkdirOptions): Promise<void> {
        return this.#clock.trackHost(this.#host.mkdir(path, options));
    }

    /** Create a new empty directory with a prefix, under the world's root by default. */
    makeTemporaryDirectory(
        prefix: string,
        options: TemporaryDirectoryOptions = {},
    ): Promise<string> {
        // name it with a suffix the world draws
        const suffix = Array.from(
            { length: SUFFIX_LENGTH },
            () => SUFFIX_LETTERS[Random.int(this.#random, SUFFIX_LETTERS.length)],
        ).join("");
        const parent = options.directory ?? this.root;
        const directory = join(parent, `${prefix}${suffix}`);

        // create the parent, then the directory
        const created = mkdir(parent, { recursive: true })
            .then(() => mkdir(directory))
            .then(() => directory);

        return this.#clock.trackHost(created);
    }

    /** Remove a path. */
    remove(path: string, options?: RemoveOptions): Promise<void> {
        return this.#clock.trackHost(this.#host.remove(path, options));
    }

    /** Move a path to another, replacing a file there. */
    rename(from: string, to: string): Promise<void> {
        return this.#clock.trackHost(this.#host.rename(from, to));
    }

    /** Copy a file to another path. */
    copyFile(from: string, to: string): Promise<void> {
        return this.#clock.trackHost(this.#host.copyFile(from, to));
    }

    /** Copy a path to another, with everything inside a directory. */
    copy(from: string, to: string): Promise<void> {
        return this.#clock.trackHost(this.#host.copy(from, to));
    }

    /** Create a symbolic link at a path to a target. */
    symlink(target: string, path: string): Promise<void> {
        return this.#clock.trackHost(this.#host.symlink(target, path));
    }

    /** Read a symbolic link's target. */
    readLink(path: string): Promise<string> {
        return this.#clock.trackHost(this.#host.readLink(path));
    }

    /** Create a hard link at a path to a file. */
    link(target: string, path: string): Promise<void> {
        return this.#clock.trackHost(this.#host.link(target, path));
    }

    /** Change a path's permission bits. */
    chmod(path: string, mode: number): Promise<void> {
        return this.#clock.trackHost(this.#host.chmod(path, mode));
    }

    /** Open a file, tracking each read, write and operation on it. */
    async open(path: string, options: OpenOptions): Promise<FileHandle> {
        const file = await this.#clock.trackHost(this.#host.open(path, options));
        const clock = this.#clock;

        return {
            readable: () => {
                // pull each chunk as tracked work
                const reader = file.readable().getReader();

                return new ReadableStream<Uint8Array>({
                    pull: async (controller) => {
                        const { done, value } = await clock.trackHost(reader.read());
                        if (done) {
                            controller.close();
                        } else {
                            controller.enqueue(value);
                        }
                    },
                    cancel: (reason) => reader.cancel(reason),
                });
            },
            writable: () => {
                // write each chunk as tracked work
                const writer = file.writable().getWriter();

                return new WritableStream<Uint8Array>({
                    write: (chunk) => clock.trackHost(writer.write(chunk)),
                    close: () => clock.trackHost(writer.close()),
                });
            },
            read: (buffer) => clock.trackHost(file.read(buffer)),
            seek: (offset) => file.seek(offset),
            stat: () => clock.trackHost(file.stat()),
            sync: () => clock.trackHost(file.sync()),
            truncate: (length) => clock.trackHost(file.truncate(length)),
            close: () => clock.trackHost(file.close()),
            [Symbol.asyncDispose]: () => clock.trackHost(file.close()),
        };
    }

    /** Report the changes at a path until the signal aborts, as untracked waits. */
    watch(path: string, options?: WatchOptions): AsyncIterable<FileEvent> {
        return this.#host.watch(path, options);
    }

    /** Read the world's runtime directory under its root, created private on first use. */
    runtimeDirectory(): Promise<string> {
        const directory = join(this.root, "runtime");

        return this.#clock.trackHost(
            this.#host.mkdir(directory, { recursive: true, mode: 0o700 }).then(() => directory),
        );
    }

    /** Open or create a file and lock it in a mode, waiting untracked only on contention. */
    async lock(
        path: string,
        mode: FileLockMode,
        options?: { readonly signal?: AbortSignal },
    ): Promise<FileLock> {
        // take a free exclusive lock at once, else wait for the holder, who may wait on simulated time
        options?.signal?.throwIfAborted();
        const free = mode === "exclusive" ? await this.tryLock(path) : undefined;

        return free ?? this.#adopt(await this.#host.lock(path, mode, options));
    }

    /** Open or create a file and lock it exclusively, absent while another holder has it. */
    async tryLock(path: string): Promise<FileLock | undefined> {
        const lock = await this.#clock.trackHost(this.#host.tryLock(path));

        return lock === undefined ? undefined : this.#adopt(lock);
    }

    /** Count a lock's release as host work, and leave its conversions untracked. */
    #adopt(lock: FileLock): FileLock {
        const clock = this.#clock;

        return {
            get mode() {
                return lock.mode;
            },
            tryExclusive: () => lock.tryExclusive(),
            share: () => lock.share(),
            close: () => clock.trackHost(lock.close()),
            [Symbol.asyncDispose]: () => clock.trackHost(lock.close()),
        };
    }
}
