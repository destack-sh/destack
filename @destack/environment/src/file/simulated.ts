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
import { FileWatcher } from "./watcher.ts";

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
    /** The watcher of the changes this file system's verbs make. */
    readonly #watcher = new FileWatcher();

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

    /** Run host work writing a path, then report it as created when new, else as modified. */
    async #write<Value>(
        path: string,
        start: () => Promise<Value>,
        isModifying = true,
    ): Promise<Value> {
        // read whether a watched path is new before the work
        const isWatched = this.#watcher.sees(path);
        const isNew = isWatched && !(await this.#run(() => this.#host.exists(path)));
        const value = await this.#run(start);

        // report the change once the work is done
        if (isNew) {
            this.#watcher.emit("create", path);
        } else if (isWatched && isModifying) {
            this.#watcher.emit("modify", path);
        }

        return value;
    }

    /** Run host work removing a path, then report it as removed when it existed. */
    async #delete(path: string, start: () => Promise<unknown>): Promise<void> {
        // read whether a watched path exists before the work
        const isWatched = this.#watcher.sees(path);
        const isExisting = isWatched && (await this.#run(() => this.#host.exists(path)));
        await this.#run(start);

        // report the removal once the work is done
        if (isExisting) {
            this.#watcher.emit("remove", path);
        }
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
        const resolved = this.#resolve(path);

        return this.#write(resolved, () => this.#host.writeFile(resolved, data, options));
    }

    /** Write text to a file as UTF-8. */
    writeTextFile(path: string, text: string, options?: WriteFileOptions): Promise<void> {
        const resolved = this.#resolve(path);

        return this.#write(resolved, () => this.#host.writeTextFile(resolved, text, options));
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
        const resolved = this.#resolve(path);

        return this.#write(resolved, () => this.#host.mkdir(resolved, options), false);
    }

    /** Create a new empty directory with a prefix, under the world's root by default. */
    async makeTemporaryDirectory(
        prefix: string,
        options: TemporaryDirectoryOptions = {},
    ): Promise<string> {
        // name it with a suffix the world draws
        const parent = options.directory ?? this.root;
        const directory = join(parent, TemporaryDirectory.draw(prefix, this.#random));

        // create the parent, then the directory
        await this.#run(() => mkdir(parent, { recursive: true }));
        await this.#write(directory, () => mkdir(directory), false);

        return directory;
    }

    /** Remove a path. */
    remove(path: string, options?: RemoveOptions): Promise<void> {
        const resolved = this.#resolve(path);

        return this.#delete(resolved, () => this.#host.remove(resolved, options));
    }

    /** Move a path to another, replacing a file there. */
    async rename(from: string, to: string): Promise<void> {
        // report the source removed, then the target written
        const [source, target] = [this.#resolve(from), this.#resolve(to)];
        const isNew = this.#watcher.sees(target) && !(await this.exists(target));
        await this.#delete(source, () => this.#host.rename(source, target));
        if (this.#watcher.sees(target)) {
            this.#watcher.emit(isNew ? "create" : "modify", target);
        }
    }

    /** Copy a file to another path. */
    copyFile(from: string, to: string): Promise<void> {
        const target = this.#resolve(to);

        return this.#write(target, () => this.#host.copyFile(this.#resolve(from), target));
    }

    /** Copy a path to another, with everything inside a directory. */
    copy(from: string, to: string): Promise<void> {
        const target = this.#resolve(to);

        return this.#write(target, () => this.#host.copy(this.#resolve(from), target));
    }

    /** Create a symbolic link at a path to a target. */
    symlink(target: string, path: string): Promise<void> {
        const resolved = this.#resolve(path);

        return this.#write(resolved, () => this.#host.symlink(target, resolved));
    }

    /** Read a symbolic link's target. */
    readLink(path: string): Promise<string> {
        return this.#run(() => this.#host.readLink(this.#resolve(path)));
    }

    /** Create a hard link at a path to a file. */
    link(target: string, path: string): Promise<void> {
        const resolved = this.#resolve(path);

        return this.#write(resolved, () => this.#host.link(this.#resolve(target), resolved));
    }

    /** Change a path's permission bits. */
    chmod(path: string, mode: number): Promise<void> {
        return this.#run(() => this.#host.chmod(this.#resolve(path), mode));
    }

    /** Open a file, tracking each read, write and operation on it. */
    async open(path: string, options: OpenOptions): Promise<FileHandle> {
        // open it, reporting a created or truncated file
        const resolved = this.#resolve(path);
        const opening = () => this.#host.open(resolved, options);
        const isWriting = options.create === true || options.truncate === true;
        const opened = await (isWriting
            ? this.#write(resolved, opening, options.truncate === true)
            : this.#run(opening));
        const modified = () => this.#watcher.emit("modify", resolved);

        return {
            readable: () => {
                // pull each chunk as tracked work
                const reader = opened.readable().getReader();

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
                const writer = opened.writable().getWriter();

                return new WritableStream<Uint8Array>({
                    write: async (chunk) => {
                        await this.#run(() => writer.write(chunk));
                        modified();
                    },
                    close: () => this.#run(() => writer.close()),
                });
            },
            read: (buffer) => this.#run(() => opened.read(buffer)),
            seek: (offset) => opened.seek(offset),
            stat: () => this.#run(() => opened.stat()),
            sync: () => this.#run(() => opened.sync()),
            truncate: async (length) => {
                await this.#run(() => opened.truncate(length));
                modified();
            },
            close: () => this.#run(() => opened.close()),
            [Symbol.asyncDispose]: () => this.#run(() => opened.close()),
        };
    }

    /** Watch the changes this file system's verbs make at an existing path until aborted. */
    async watch(path: string, options?: WatchOptions): Promise<AsyncIterable<FileEvent>> {
        // refuse a missing path, then watch at once
        const resolved = this.#resolve(path);
        await this.stat(resolved);

        return this.#watcher.watch(resolved, options);
    }

    /** Read the world's cache directory under its root, created on first use. */
    cacheDirectory(): Promise<string> {
        const directory = join(this.root, "cache");

        return this.#run(() =>
            this.#host.mkdir(directory, { recursive: true }).then(() => directory),
        );
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
