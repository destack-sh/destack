import { mkdir } from "node:fs/promises";
import { isAbsolute, join } from "node:path";
import { BunFileSystem } from "./bun.ts";
import type { Scheduler } from "../scheduler/scheduler.ts";
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
import type { Random } from "../random/random.ts";
import { TemporaryDirectory } from "./temporary.ts";

/** The host's files at their real paths, each operation tracked as host work. */
export class SimulatedFileSystem implements FileSystem {
    /** The world's root, which its temporary directories go under and its dispose removes. */
    readonly root: string;
    /** The host's files. */
    readonly #host = new BunFileSystem();
    /** The world's scheduler, which counts each operation. */
    readonly #scheduler: Scheduler;
    /** The world's randomness, which temporary names draw from. */
    readonly #random: Random;
    /** The locks the node took, released once it crashes. */
    readonly #locks = new Set<FileLock>();

    /** Track the host's files on a scheduler, its temporary directories under its root. */
    constructor(root: string, scheduler: Scheduler, random: Random) {
        this.root = root;
        this.#scheduler = scheduler;
        this.#random = random;
    }

    /** Resolve a path against the world's root, as a node's working directory is. */
    #resolve(path: string): string {
        return isAbsolute(path) ? path : join(this.root, path);
    }

    /** Run host file work as an operation of the world's scheduler. */
    #run<Value>(start: () => Promise<Value>): Promise<Value> {
        return this.#scheduler.run(start);
    }

    /** Read a file's bytes. */
    readFile(path: string): Promise<Uint8Array<ArrayBuffer>> {
        return this.#run(() => this.#host.readFile(this.#resolve(path)));
    }

    /** Read a file's text as UTF-8. */
    readTextFile(path: string): Promise<string> {
        return this.#run(() => this.#host.readTextFile(this.#resolve(path)));
    }

    /** Write bytes to a file. */
    writeFile(path: string, data: Uint8Array, options?: WriteFileOptions): Promise<void> {
        return this.#run(() => this.#host.writeFile(this.#resolve(path), data, options));
    }

    /** Write text to a file as UTF-8. */
    writeTextFile(path: string, text: string, options?: WriteFileOptions): Promise<void> {
        return this.#run(() => this.#host.writeTextFile(this.#resolve(path), text, options));
    }

    /** Read whether a path exists. */
    exists(path: string): Promise<boolean> {
        return this.#run(() => this.#host.exists(this.#resolve(path)));
    }

    /** Read a path's status, following links. */
    stat(path: string): Promise<FileStatus> {
        return this.#run(() => this.#host.stat(this.#resolve(path)));
    }

    /** Read a path's status without following a link. */
    lstat(path: string): Promise<FileStatus> {
        return this.#run(() => this.#host.lstat(this.#resolve(path)));
    }

    /** Read the absolute path a path resolves to, its links followed. */
    realPath(path: string): Promise<string> {
        return this.#run(() => this.#host.realPath(this.#resolve(path)));
    }

    /** Join a directory and the names below it into one path, as the host does. */
    join(directory: string, ...names: readonly string[]): string {
        return this.#host.join(directory, ...names);
    }

    /** List a directory's entries. */
    readDirectory(path: string, options?: ReadDirectoryOptions): Promise<DirectoryEntry[]> {
        return this.#run(() => this.#host.readDirectory(this.#resolve(path), options));
    }

    /** Create a directory. */
    mkdir(path: string, options?: MkdirOptions): Promise<void> {
        return this.#run(() => this.#host.mkdir(this.#resolve(path), options));
    }

    /** Create a new empty directory with a prefix, under the world's root by default. */
    makeTemporaryDirectory(
        prefix: string,
        options: TemporaryDirectoryOptions = {},
    ): Promise<string> {
        // name it with a suffix the world draws
        const parent = options.directory ?? this.root;
        const directory = join(parent, TemporaryDirectory.draw(prefix, this.#random));

        // create the parent, then the directory
        return this.#run(() =>
            mkdir(parent, { recursive: true })
                .then(() => mkdir(directory))
                .then(() => directory),
        );
    }

    /** Remove a path. */
    remove(path: string, options?: RemoveOptions): Promise<void> {
        return this.#run(() => this.#host.remove(this.#resolve(path), options));
    }

    /** Move a path to another, replacing a file there. */
    rename(from: string, to: string): Promise<void> {
        return this.#run(() => this.#host.rename(this.#resolve(from), this.#resolve(to)));
    }

    /** Copy a file to another path. */
    copyFile(from: string, to: string): Promise<void> {
        return this.#run(() => this.#host.copyFile(this.#resolve(from), this.#resolve(to)));
    }

    /** Copy a path to another, with everything inside a directory. */
    copy(from: string, to: string): Promise<void> {
        return this.#run(() => this.#host.copy(this.#resolve(from), this.#resolve(to)));
    }

    /** Create a symbolic link at a path to a target. */
    symlink(target: string, path: string): Promise<void> {
        return this.#run(() => this.#host.symlink(target, this.#resolve(path)));
    }

    /** Read a symbolic link's target. */
    readLink(path: string): Promise<string> {
        return this.#run(() => this.#host.readLink(this.#resolve(path)));
    }

    /** Create a hard link at a path to a file. */
    link(target: string, path: string): Promise<void> {
        return this.#run(() => this.#host.link(this.#resolve(target), this.#resolve(path)));
    }

    /** Change a path's permission bits. */
    chmod(path: string, mode: number): Promise<void> {
        return this.#run(() => this.#host.chmod(this.#resolve(path), mode));
    }

    /** Open a file, tracking each read, write and operation on it. */
    async open(path: string, options: OpenOptions): Promise<FileHandle> {
        const file = await this.#run(() => this.#host.open(this.#resolve(path), options));

        return {
            readable: () => {
                // pull each chunk as tracked work
                const reader = file.readable().getReader();

                return new ReadableStream<Uint8Array>({
                    pull: async (controller) => {
                        const { done, value } = await this.#run(() => reader.read());
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
                    write: (chunk) => this.#run(() => writer.write(chunk)),
                    close: () => this.#run(() => writer.close()),
                });
            },
            read: (buffer) => this.#run(() => file.read(buffer)),
            seek: (offset) => file.seek(offset),
            stat: () => this.#run(() => file.stat()),
            sync: () => this.#run(() => file.sync()),
            truncate: (length) => this.#run(() => file.truncate(length)),
            close: () => this.#run(() => file.close()),
            [Symbol.asyncDispose]: () => this.#run(() => file.close()),
        };
    }

    /** Report the changes at a path until the signal aborts, as untracked waits. */
    watch(path: string, options?: WatchOptions): AsyncIterable<FileEvent> {
        return this.#host.watch(this.#resolve(path), options);
    }

    /** Read the world's runtime directory under its root, created private on first use. */
    runtimeDirectory(): Promise<string> {
        const directory = join(this.root, "runtime");

        return this.#run(() =>
            this.#host.mkdir(directory, { recursive: true, mode: 0o700 }).then(() => directory),
        );
    }

    /** Open or create a file and lock it in a mode, waiting untracked only on contention. */
    async lock(
        path: string,
        mode: FileLockMode,
        options?: { readonly signal?: AbortSignal },
    ): Promise<FileLock> {
        // take a free lock at once, else wait for the holder, who may wait on simulated time
        options?.signal?.throwIfAborted();
        const free = await this.tryLock(path, mode);

        return free ?? this.#adopt(await this.#host.lock(this.#resolve(path), mode, options));
    }

    /** Lock a file in a mode now, absent while another holder has it. */
    async tryLock(path: string, mode: FileLockMode): Promise<FileLock | undefined> {
        const lock = await this.#run(() => this.#host.tryLock(this.#resolve(path), mode));

        return lock === undefined ? undefined : this.#adopt(lock);
    }

    /** Count a lock's release as host work, and leave its conversions untracked. */
    #adopt(lock: FileLock): FileLock {
        // remember the lock until it closes, so a crash releases it
        this.#locks.add(lock);
        const close = () => {
            this.#locks.delete(lock);

            return this.#run(() => lock.close());
        };

        return {
            get mode() {
                return lock.mode;
            },
            tryExclusive: () => lock.tryExclusive(),
            share: () => lock.share(),
            close,
            [Symbol.asyncDispose]: close,
        };
    }

    /** Release the locks the node took, as the operating system does once its process ends. */
    release(): void {
        // close each lock as host work, leaving a node with none untouched
        if (this.#locks.size > 0) {
            const locks = [...this.#locks];
            this.#locks.clear();
            void this.#run(() => Promise.all(locks.map((lock) => lock.close())));
        }
    }
}
