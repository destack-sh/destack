import type { Clock } from "../clock/clock.ts";
import type { Random } from "../random/random.ts";
import { FileSystemError } from "./error.ts";
import {
    DirectoryEntry,
    type FileEvent,
    type FileHandle,
    type FileKind,
    type FileLock,
    type FileLockMode,
    type FileStatus,
    type FileSystem,
    type MkdirOptions,
    type OpenOptions,
    type ReadDirectoryOptions,
    type RemoveOptions,
    type TemporaryDirectoryOptions,
    type WatchOptions,
    type WriteFileOptions,
} from "./file.ts";
import { TemporaryDirectory } from "./temporary.ts";

/** The file type bits of a file, a directory and a symbolic link, as POSIX `st_mode` has them. */
const FILE_TYPES = { file: 0o100000, directory: 0o040000, symlink: 0o120000 } as const;

/** The permission bits of `st_mode`. */
const PERMISSION_BITS = 0o7777;

/** The permission bits of a created file and directory unless given, after a 022 umask. */
const DEFAULT_MODES = { file: 0o644, directory: 0o755 } as const;

/** The most symbolic links one path resolves through before failing, as Linux's `MAXSYMLINKS`. */
const MOST_LINKS = 40;

/** The bytes a file's stream reads at once, as Bun's file streams chunk them. */
const CHUNK_BYTES = 64 * 1024;

/** A regular file: its bytes in a doubling buffer, permissions, change time and locks. */
class MemoryFile {
    /** What it is. */
    readonly kind = "file";
    /** The buffer, its first `size` bytes the file's. */
    #bytes: Uint8Array<ArrayBuffer>;
    /** The file's length in bytes. */
    size: number;
    /** The permission bits. */
    mode: number;
    /** When the bytes last changed, in UTC epoch milliseconds. */
    modifiedAt: number;
    /** The locks on it. */
    readonly locks = new Set<MemoryLock>();
    /** The waits for a lock that does not fit beside the others yet. */
    readonly waiters = new Set<() => void>();

    /** Make a file of a copy of some bytes with permission bits, stamped at a time. */
    constructor(data: Uint8Array, mode: number, at: number) {
        // copy the bytes, keeping only the permission bits
        this.#bytes = data.slice();
        this.size = data.byteLength;
        this.mode = mode & PERMISSION_BITS;
        this.modifiedAt = at;
    }

    /** The file's bytes, a view of its buffer. */
    get contents(): Uint8Array<ArrayBuffer> {
        return this.#bytes.subarray(0, this.size);
    }

    /** Write bytes at an offset, its length cut to their end when replacing, stamped at a time. */
    write(offset: number, data: Uint8Array, isReplacing: boolean, at: number): void {
        // grow the buffer to fit, then copy the bytes in and stamp the file
        const end = offset + data.byteLength;
        this.#reserve(end);
        this.#bytes.set(data, offset);
        this.size = isReplacing ? end : Math.max(this.size, end);
        this.modifiedAt = at;
    }

    /** Cut the file to a length or extend it with zeros, stamped at a time. */
    resize(length: number, at: number): void {
        // grow the buffer to fit, and zero what a cut left behind for a later extension
        this.#reserve(length);
        this.#bytes.fill(0, Math.min(length, this.size), Math.max(length, this.size));
        this.size = length;
        this.modifiedAt = at;
    }

    /** Report whether a lock of a mode fits beside the others. */
    fits(mode: FileLockMode): boolean {
        return (
            this.locks.size === 0 ||
            (mode === "shared" && [...this.locks].every((each) => each.mode === "shared"))
        );
    }

    /** Wake every wait for a lock, as a release or share may let it fit. */
    wake(): void {
        const woken = [...this.waiters];
        this.waiters.clear();
        for (const waiter of woken) {
            waiter();
        }
    }

    /** Grow the buffer to fit some bytes, doubling it so streamed writes cost linear time. */
    #reserve(length: number): void {
        if (length > this.#bytes.byteLength) {
            const grown = new Uint8Array(Math.max(length, this.#bytes.byteLength * 2));
            grown.set(this.contents);
            this.#bytes = grown;
        }
    }
}

/** A directory: its entries by name, permissions and change time. */
interface MemoryDirectory {
    /** What it is. */
    readonly kind: "directory";
    /** The entries by name. */
    readonly entries: Map<string, Inode>;
    /** The permission bits. */
    mode: number;
    /** When the entries last changed, in UTC epoch milliseconds. */
    modifiedAt: number;
}

/** A symbolic link: the path it points at. */
interface MemoryLink {
    /** What it is. */
    readonly kind: "symlink";
    /** The path it points at, relative to its directory unless absolute. */
    readonly target: string;
    /** When it was made, in UTC epoch milliseconds. */
    readonly modifiedAt: number;
}

/** A file system object, shared by every hard link to it. */
type Inode = MemoryFile | MemoryDirectory | MemoryLink;

/** A lock on a file: its mode, changed by converting it. */
interface MemoryLock {
    /** How the lock is taken. */
    mode: FileLockMode;
}

/** A path found in the tree: its real path, its directory and name there, and what is there. */
interface Located {
    /** The path with every link on the way resolved. */
    readonly path: string;
    /** The directory with the entry. */
    readonly parent: MemoryDirectory;
    /** Its name in the directory, empty for the root. */
    readonly name: string;
    /** What is there, absent when nothing is. */
    readonly node: Inode | undefined;
}

/** A watch of a path: where it looks, how deep, and how it takes each change. */
interface MemoryWatch {
    /** The watched path. */
    readonly path: string;
    /** Whether it reports changes inside subdirectories. */
    readonly isRecursive: boolean;
    /** Take a change. */
    readonly take: (event: FileEvent) => void;
}

/** A world's tree of files: its root, working directory, watches, clock and randomness. */
interface MemoryTree {
    /** The root directory. */
    readonly root: MemoryDirectory;
    /** The directory relative paths start from, and temporary and runtime directories go below. */
    readonly workingDirectory: string;
    /** The watches of paths. */
    readonly watches: Set<MemoryWatch>;
    /** The world's clock, which stamps each change. */
    readonly clock: Clock;
    /** The world's randomness, which temporary names draw from. */
    readonly random: Random;
}

/** A node's view of a world's files in memory: one shared tree, and the locks this node took. */
export class MemoryFileSystem implements FileSystem {
    /** The world's tree. */
    readonly #tree: MemoryTree;
    /** The locks the node took, released once it crashes. */
    readonly #locks = new Set<FileLock>();

    /** View a tree. */
    private constructor(tree: MemoryTree) {
        this.#tree = tree;
    }

    /** Make a world's tree with its working directory and some directories, for a first node. */
    static create(options: {
        readonly clock: Clock;
        readonly random: Random;
        readonly workingDirectory: string;
        readonly directories?: readonly string[];
    }): MemoryFileSystem {
        // start with the root, then the working directory below it
        const { clock, random } = options;
        const root: MemoryDirectory = {
            kind: "directory",
            entries: new Map(),
            mode: DEFAULT_MODES.directory,
            modifiedAt: clock.now(),
        };
        const workingDirectory = normalize("/", options.workingDirectory);
        const files = new MemoryFileSystem({
            root,
            workingDirectory,
            watches: new Set(),
            clock,
            random,
        });
        for (const directory of [workingDirectory, ...(options.directories ?? [])]) {
            files.#makeDirectories(directory, DEFAULT_MODES.directory, `create ${directory}`);
        }

        return files;
    }

    /** View the same tree as another node, with separate locks. */
    view(): MemoryFileSystem {
        return new MemoryFileSystem(this.#tree);
    }

    /** Release the locks this view took, as the operating system does once a process ends. */
    release(): void {
        for (const lock of this.#locks) {
            void lock.close();
        }
    }

    /** Read a file's bytes. */
    async readFile(path: string): Promise<Uint8Array<ArrayBuffer>> {
        return this.#file(path, `read ${path}`).contents.slice();
    }

    /** Read a file's text as UTF-8. */
    async readTextFile(path: string): Promise<string> {
        return new TextDecoder().decode(this.#file(path, `read ${path}`).contents);
    }

    /** Write bytes to a file. */
    async writeFile(path: string, data: Uint8Array, options: WriteFileOptions = {}): Promise<void> {
        // refuse any existing path when asked for a new one
        const action = `write ${path}`;
        const found = this.#locate(path, true, action);
        const { node } = found;
        if (node !== undefined && options.createNew === true) {
            throw FileSystemError.posix("EEXIST", action);
        }
        // write into an existing file, at its end when appending
        else if (node !== undefined) {
            const file = this.#asFile(node, action);
            const isAppending = options.append === true;
            file.write(isAppending ? file.size : 0, data, !isAppending, this.#tree.clock.now());
            this.#emit("modify", found.path);
        }
        // create it in its directory otherwise
        else {
            this.#add(found, this.#makeFile(data, options.mode ?? DEFAULT_MODES.file));
        }
    }

    /** Write text to a file as UTF-8. */
    writeTextFile(path: string, text: string, options?: WriteFileOptions): Promise<void> {
        return this.writeFile(path, new TextEncoder().encode(text), options);
    }

    /** Read whether a path exists, its links followed. */
    async exists(path: string): Promise<boolean> {
        try {
            return this.#locate(path, true, `read ${path}`).node !== undefined;
        } catch (error) {
            return FileSystemError.absent(error) ?? false;
        }
    }

    /** Read a path's status, following links. */
    async stat(path: string): Promise<FileStatus> {
        return MemoryFileSystem.#status(this.#existing(path, true, `read ${path}`).node);
    }

    /** Read a path's status without following a link. */
    async lstat(path: string): Promise<FileStatus> {
        return MemoryFileSystem.#status(this.#existing(path, false, `read ${path}`).node);
    }

    /** Read the absolute path a path resolves to, its links followed. */
    async realPath(path: string): Promise<string> {
        return this.#existing(path, true, `resolve ${path}`).path;
    }

    /** List a directory's entries in walk order, inside subdirectories too when recursive. */
    async readDirectory(
        path: string,
        options: ReadDirectoryOptions = {},
    ): Promise<DirectoryEntry[]> {
        // list the directory's entries, and each subdirectory's when recursive
        const action = `list ${path}`;
        const directory = this.#asDirectory(this.#existing(path, true, action).node, action);
        const entries: DirectoryEntry[] = [];
        const list = (from: MemoryDirectory, prefix: string) => {
            for (const [name, node] of from.entries) {
                entries.push({ name: `${prefix}${name}`, kind: node.kind });
                if (options.recursive === true && node.kind === "directory") {
                    list(node, `${prefix}${name}/`);
                }
            }
        };
        list(directory, "");

        return entries.toSorted((left, right) => DirectoryEntry.compare(left, right));
    }

    /** Create a directory, with its missing parents when recursive. */
    async mkdir(path: string, options: MkdirOptions = {}): Promise<void> {
        // create each missing parent first when recursive
        const action = `create ${path}`;
        const mode = options.mode ?? DEFAULT_MODES.directory;
        if (options.recursive === true) {
            this.#makeDirectories(path, mode, action);

            return;
        }

        // create one directory in an existing parent, refusing an existing path
        const found = this.#locate(path, false, action);
        if (found.node !== undefined) {
            throw FileSystemError.posix("EEXIST", action);
        }
        this.#add(found, this.#makeDirectory(mode));
    }

    /** Create a new empty directory with a prefix, in the working directory by default. */
    async makeTemporaryDirectory(
        prefix: string,
        options: TemporaryDirectoryOptions = {},
    ): Promise<string> {
        // create the parent, then a directory under a free name
        const parent = this.#resolve(options.directory ?? this.#tree.workingDirectory);
        await this.mkdir(parent, { recursive: true });
        for (;;) {
            // draw a name, taking it unless a path has it
            const path = join(parent, TemporaryDirectory.draw(prefix, this.#tree.random));
            if (this.#locate(path, false, `create ${prefix} in ${parent}`).node === undefined) {
                await this.mkdir(path, { mode: 0o700 });

                return path;
            }
        }
    }

    /** Remove a path, with everything inside a directory when recursive, an empty one without. */
    async remove(path: string, options: RemoveOptions = {}): Promise<void> {
        // accept a missing path when forced
        const action = `remove ${path}`;
        const found = this.#locate(path, false, action);
        const { node } = found;
        if (node === undefined) {
            if (options.force === true) {
                return;
            }
            throw FileSystemError.posix("ENOENT", action);
        }

        // refuse the root, and a directory with entries unless recursive
        if (node === this.#tree.root) {
            throw FileSystemError.posix("EBUSY", action);
        } else if (
            node.kind === "directory" &&
            node.entries.size > 0 &&
            options.recursive !== true
        ) {
            throw FileSystemError.posix("ENOTEMPTY", action);
        }
        this.#delete(found);
    }

    /** Move a path to another, replacing a file or an empty directory there. */
    async rename(from: string, to: string): Promise<void> {
        // require the source, leave a move onto the same object alone, and refuse moving a directory into itself
        const action = `move ${from} to ${to}`;
        const source = this.#existing(from, false, action);
        const target = this.#locate(to, false, action);
        if (source.node === target.node) {
            return;
        } else if (
            source.node === this.#tree.root ||
            `${target.path}/`.startsWith(`${source.path}/`)
        ) {
            throw FileSystemError.posix("EINVAL", action);
        }

        // refuse replacing what a rename cannot, as POSIX rename does
        const replaced = target.node;
        const isDirectory = source.node.kind === "directory";
        if (replaced !== undefined && isDirectory && replaced.kind !== "directory") {
            throw FileSystemError.posix("ENOTDIR", action);
        } else if (replaced !== undefined && !isDirectory && replaced.kind === "directory") {
            throw FileSystemError.posix("EISDIR", action);
        } else if (replaced?.kind === "directory" && replaced.entries.size > 0) {
            throw FileSystemError.posix("ENOTEMPTY", action);
        }

        // move the entry
        this.#delete(source);
        this.#add(target, source.node, replaced === undefined ? "create" : "modify");
    }

    /** Copy a file's bytes and permissions to another path. */
    async copyFile(from: string, to: string): Promise<void> {
        // write the source's bytes, then give the copy its permission bits
        const action = `copy ${from} to ${to}`;
        const source = this.#file(from, action);
        await this.writeFile(to, source.contents, { mode: source.mode });
        this.#file(to, action).mode = source.mode;
    }

    /** Copy a path to another, merging a directory into an existing one, copying links as links. */
    async copy(from: string, to: string): Promise<void> {
        // copy a link as a link, and a file by its bytes
        const action = `copy ${from} to ${to}`;
        const source = this.#existing(from, false, action);
        const { node } = source;
        if (node.kind === "symlink") {
            await this.remove(to, { force: true });
            await this.symlink(node.target, to);
        } else if (node.kind === "file") {
            await this.copyFile(from, to);
        }
        // refuse copying a directory into itself
        else if (`${this.#resolve(to)}/`.startsWith(`${source.path}/`)) {
            throw FileSystemError.posix("ERR_FS_CP_EINVAL", action);
        }
        // copy a directory's entries into one at the target
        else {
            const target = this.#locate(to, true, action).node;
            if (target === undefined) {
                await this.mkdir(to, { mode: node.mode });
            } else {
                this.#asDirectory(target, action);
            }
            for (const name of node.entries.keys()) {
                await this.copy(join(source.path, name), join(this.#resolve(to), name));
            }
        }
    }

    /** Create a symbolic link at a path to a target. */
    async symlink(target: string, path: string): Promise<void> {
        // refuse a taken path, then add the link
        const action = `link ${path} to ${target}`;
        const found = this.#locate(path, false, action);
        if (found.node !== undefined) {
            throw FileSystemError.posix("EEXIST", action);
        }
        this.#add(found, { kind: "symlink", target, modifiedAt: this.#tree.clock.now() });
    }

    /** Read a symbolic link's target. */
    async readLink(path: string): Promise<string> {
        // refuse anything but a link
        const action = `read ${path}`;
        const { node } = this.#existing(path, false, action);
        if (node.kind !== "symlink") {
            throw FileSystemError.posix("EINVAL", action);
        }

        return node.target;
    }

    /** Create a hard link at a path to a file, both sharing its bytes. */
    async link(target: string, path: string): Promise<void> {
        // require a file at the target, refusing a directory, and a free path
        const action = `link ${path} to ${target}`;
        const { node } = this.#existing(target, false, action);
        if (node.kind === "directory") {
            throw FileSystemError.posix("EPERM", action);
        }
        const found = this.#locate(path, false, action);
        if (found.node !== undefined) {
            throw FileSystemError.posix("EEXIST", action);
        }
        this.#add(found, node);
    }

    /** Change a path's permission bits. */
    async chmod(path: string, mode: number): Promise<void> {
        // refuse a link, and set the bits of anything else
        const action = `change ${path}`;
        const { node } = this.#existing(path, true, action);
        if (node.kind === "symlink") {
            throw FileSystemError.posix("EINVAL", action);
        }
        node.mode = mode & PERMISSION_BITS;
    }

    /** Open a file, created, emptied or appended to as the options ask. */
    async open(path: string, options: OpenOptions): Promise<FileHandle> {
        // refuse an existing path when asked for a new one, and a missing one unless asked to create it
        const action = `open ${path}`;
        const found = this.#locate(path, true, action);
        if (found.node !== undefined && options.createNew === true) {
            throw FileSystemError.posix("EEXIST", action);
        } else if (
            found.node === undefined &&
            options.create !== true &&
            options.createNew !== true
        ) {
            throw FileSystemError.posix("ENOENT", action);
        } else if (found.node === undefined) {
            this.#add(found, this.#makeFile(new Uint8Array(0), options.mode ?? DEFAULT_MODES.file));
        }
        const opened = this.#existing(path, true, action);
        const file = this.#asFile(opened.node, action);

        // empty it when asked
        if (options.truncate === true) {
            file.resize(0, this.#tree.clock.now());
            this.#emit("modify", opened.path);
        }

        return this.#handle(file, opened.path, options);
    }

    /** Report the changes at a path until the signal aborts or the reader stops. */
    watch(path: string, options: WatchOptions = {}): AsyncIterable<FileEvent> {
        // buffer each change until read, waking a waiting reader
        const { watches } = this.#tree;
        const events: FileEvent[] = [];
        let wake: (() => void) | undefined;
        const watch: MemoryWatch = {
            path: this.#existing(path, true, `watch ${path}`).path,
            isRecursive: options.recursive ?? false,
            take: (event) => {
                events.push(event);
                wake?.();
            },
        };
        watches.add(watch);

        // stop watching once the signal aborts
        const { signal } = options;
        const stop = () => {
            watches.delete(watch);
            wake?.();
        };
        signal?.addEventListener("abort", stop, { once: true });
        if (signal?.aborted === true) {
            stop();
        }

        // wait for the next change, as the reader does once the buffer runs dry
        const changed = () =>
            new Promise<void>((resolve) => {
                wake = resolve;
            });

        return (async function* () {
            try {
                // yield each buffered change, then end once stopped, or wait for the next
                for (;;) {
                    const event = events.shift();
                    if (event !== undefined) {
                        yield event;
                    } else if (signal?.aborted === true) {
                        return;
                    } else {
                        await changed();
                    }
                }
            } finally {
                // stop watching once the reader stops early
                signal?.removeEventListener("abort", stop);
                watches.delete(watch);
            }
        })();
    }

    /** Find the runtime's private directory below the working directory, made on first use. */
    async runtimeDirectory(): Promise<string> {
        const directory = join(this.#tree.workingDirectory, "runtime");
        await this.mkdir(directory, { recursive: true, mode: 0o700 });

        return directory;
    }

    /** Open or create a file and wait to lock it in a mode, until the signal aborts. */
    async lock(
        path: string,
        mode: FileLockMode,
        options: { readonly signal?: AbortSignal } = {},
    ): Promise<FileLock> {
        // wait until the lock fits beside the others, or the signal aborts
        const { signal } = options;
        const file = await this.#lockable(path);
        while (!file.fits(mode)) {
            signal?.throwIfAborted();
            await new Promise<void>((resolve, reject) => {
                // wake on a release, or fail on the signal, leaving no wait behind
                const woken = () => {
                    signal?.removeEventListener("abort", abort);
                    resolve();
                };
                const abort = () => {
                    file.waiters.delete(woken);
                    reject(signal?.reason);
                };
                file.waiters.add(woken);
                signal?.addEventListener("abort", abort, { once: true });
            });
        }

        return this.#take(file, mode);
    }

    /** Lock a file in a mode now, absent while another lock is in the way. */
    async tryLock(path: string, mode: FileLockMode): Promise<FileLock | undefined> {
        const file = await this.#lockable(path);

        return file.fits(mode) ? this.#take(file, mode) : undefined;
    }

    /** Open or create the file a lock is taken on. */
    async #lockable(path: string): Promise<MemoryFile> {
        await (await this.open(path, { read: true, write: true, create: true })).close();

        return this.#file(path, `open ${path}`);
    }

    /** Take a lock on a file for this view, waking the file's waiters on each release or share. */
    #take(file: MemoryFile, mode: FileLockMode): FileLock {
        // take the lock, and wake the waiters once it changes
        const taken: MemoryLock = { mode };
        file.locks.add(taken);
        const lock: FileLock = {
            get mode() {
                return taken.mode;
            },
            tryExclusive: async () => {
                // convert it only when no other lock is on the file
                const isAlone = [...file.locks].every((each) => each === taken);
                taken.mode = isAlone ? "exclusive" : taken.mode;

                return isAlone;
            },
            share: async () => {
                taken.mode = "shared";
                file.wake();
            },
            close: async () => {
                file.locks.delete(taken);
                this.#locks.delete(lock);
                file.wake();
            },
            [Symbol.asyncDispose]: () => lock.close(),
        };
        this.#locks.add(lock);

        return lock;
    }

    /** Make an open file's handle, reading and writing at its cursor. */
    #handle(file: MemoryFile, path: string, options: OpenOptions): FileHandle {
        // refuse reads and writes the handle was not opened for, and any once closed
        let cursor = 0;
        let isClosed = false;
        const isWriting = options.write === true || options.append === true;
        const require = (isAllowed: boolean | undefined, action: string) => {
            if (isClosed || isAllowed !== true) {
                throw FileSystemError.posix("EBADF", `${action} ${path}`);
            }
        };
        const read = (buffer: Uint8Array): number | null => {
            // copy the bytes at the cursor into the buffer, moving the cursor, or answer the end
            require(options.read, "read");
            const count = Math.min(buffer.byteLength, file.size - cursor);
            if (count <= 0) {
                return buffer.byteLength === 0 ? 0 : null;
            }
            buffer.set(file.contents.subarray(cursor, cursor + count));
            cursor += count;

            return count;
        };
        const write = (chunk: Uint8Array) => {
            // write the bytes at the cursor, or at the end when appending, then move the cursor past them
            require(isWriting, "write");
            const at = options.append === true ? file.size : cursor;
            file.write(at, chunk, false, this.#tree.clock.now());
            cursor = at + chunk.byteLength;
            this.#emit("modify", path);
        };
        const close = async () => {
            isClosed = true;
        };

        return {
            readable: () =>
                new ReadableStream<Uint8Array>({
                    pull: (controller) => {
                        // pass a chunk at the cursor on, closing at the end
                        const buffer = new Uint8Array(CHUNK_BYTES);
                        const count = read(buffer);
                        if (count === null) {
                            controller.close();
                        } else {
                            controller.enqueue(buffer.subarray(0, count));
                        }
                    },
                }),
            writable: () => new WritableStream<Uint8Array>({ write: (chunk) => write(chunk) }),
            read: async (buffer) => read(buffer),
            seek: async (offset) => {
                require(!isClosed, "seek");
                cursor = offset;

                return cursor;
            },
            stat: async () => MemoryFileSystem.#status(file),
            sync: async () => require(!isClosed, "sync"),
            truncate: async (length = 0) => {
                require(isWriting, "truncate");
                file.resize(length, this.#tree.clock.now());
                this.#emit("modify", path);
            },
            close,
            [Symbol.asyncDispose]: close,
        };
    }

    /** Create a directory with its missing parents, accepting existing ones. */
    #makeDirectories(path: string, mode: number, action: string): void {
        const parts = normalize(this.#tree.workingDirectory, path)
            .split("/")
            .filter((part) => part !== "");
        for (let index = 1; index <= parts.length; index += 1) {
            // accept an existing directory, and create a missing one
            const found = this.#locate(`/${parts.slice(0, index).join("/")}`, true, action);
            if (found.node === undefined) {
                this.#add(found, this.#makeDirectory(mode));
            } else {
                this.#asDirectory(found.node, action);
            }
        }
    }

    /** Find a path name by name, following links on the way and the last one when asked. */
    #locate(path: string, isFollowing: boolean, action: string, hops = 0): Located {
        // refuse a path through too many links
        if (hops > MOST_LINKS) {
            throw FileSystemError.posix("ELOOP", action);
        }

        // walk from the root by the real path so far, `..` stepping up from where links led
        const names = this.#resolve(path)
            .split("/")
            .filter((name) => name !== "" && name !== ".");
        const real: string[] = [];
        let directory = this.#tree.root;
        for (const [index, name] of names.entries()) {
            const isLast = index === names.length - 1;
            const rest = names.slice(index + 1);
            const entry = name === ".." ? undefined : directory.entries.get(name);

            // step up to the parent of the real path
            if (name === "..") {
                real.pop();
                directory = this.#directoryAt(real, action);
            }
            // follow a link on the way, or the last one when asked, by its target and what follows it
            else if (entry?.kind === "symlink" && (!isLast || isFollowing)) {
                const target = entry.target.startsWith("/")
                    ? entry.target
                    : `/${[...real, entry.target].join("/")}`;
                return this.#locate([target, ...rest].join("/"), isFollowing, action, hops + 1);
            }
            // answer the last name in its directory
            else if (isLast) {
                return {
                    path: `/${[...real, name].join("/")}`,
                    parent: directory,
                    name,
                    node: entry,
                };
            }
            // step into a directory on the way
            else if (entry === undefined) {
                throw FileSystemError.posix("ENOENT", action);
            } else {
                real.push(name);
                directory = this.#asDirectory(entry, action);
            }
        }

        return {
            path: `/${real.join("/")}`,
            parent: this.#directoryAt(real.slice(0, -1), action),
            name: real.at(-1) ?? "",
            node: directory,
        };
    }

    /** Read the directory at a real path of names, which has no links on it. */
    #directoryAt(names: readonly string[], action: string): MemoryDirectory {
        let directory = this.#tree.root;
        for (const name of names) {
            const entry = directory.entries.get(name);
            if (entry === undefined) {
                throw FileSystemError.posix("ENOENT", action);
            }
            directory = this.#asDirectory(entry, action);
        }

        return directory;
    }

    /** Find an existing path, refusing a missing one. */
    #existing(
        path: string,
        isFollowing: boolean,
        action: string,
    ): Located & { readonly node: Inode } {
        // find it, refusing nothing there
        const found = this.#locate(path, isFollowing, action);
        const { node } = found;
        if (node === undefined) {
            throw FileSystemError.posix("ENOENT", action);
        }

        return { ...found, node };
    }

    /** Resolve a path against the working directory, without following links. */
    #resolve(path: string): string {
        return path.startsWith("/") ? path : join(this.#tree.workingDirectory, path);
    }

    /** Read the file at a path, its links followed, refusing a directory. */
    #file(path: string, action: string): MemoryFile {
        return this.#asFile(this.#existing(path, true, action).node, action);
    }

    /** Require a file, refusing a directory and anything else. */
    #asFile(node: Inode, action: string): MemoryFile {
        if (node.kind !== "file") {
            throw FileSystemError.posix(node.kind === "directory" ? "EISDIR" : "EINVAL", action);
        }

        return node;
    }

    /** Require a directory, refusing anything else. */
    #asDirectory(node: Inode, action: string): MemoryDirectory {
        if (node.kind !== "directory") {
            throw FileSystemError.posix("ENOTDIR", action);
        }

        return node;
    }

    /** Add a node at a located path, stamping its directory, and report it. */
    #add(found: Located, node: Inode, kind: FileEvent["kind"] = "create"): void {
        found.parent.entries.set(found.name, node);
        found.parent.modifiedAt = this.#tree.clock.now();
        this.#emit(kind, found.path);
    }

    /** Delete a located path from its directory, stamping it, and report it. */
    #delete(found: Located): void {
        found.parent.entries.delete(found.name);
        found.parent.modifiedAt = this.#tree.clock.now();
        this.#emit("remove", found.path);
    }

    /** Make a file of a copy of some bytes with permission bits, stamped now. */
    #makeFile(data: Uint8Array, mode: number): MemoryFile {
        return new MemoryFile(data, mode, this.#tree.clock.now());
    }

    /** Make an empty directory with permission bits, stamped now. */
    #makeDirectory(mode: number): MemoryDirectory {
        return {
            kind: "directory",
            entries: new Map(),
            mode: mode & PERMISSION_BITS,
            modifiedAt: this.#tree.clock.now(),
        };
    }

    /** Report a change at a path to each watch that sees it, relative to the watched path. */
    #emit(kind: FileEvent["kind"], path: string): void {
        for (const watch of this.#tree.watches) {
            // report the watched path by its name, and a path below it relative to it
            const below = watch.path === "/" ? path.slice(1) : path.slice(watch.path.length + 1);
            if (path === watch.path) {
                watch.take({ kind, path: path.slice(path.lastIndexOf("/") + 1) });
            } else if (
                path.startsWith(watch.path === "/" ? "/" : `${watch.path}/`) &&
                (watch.isRecursive || !below.includes("/"))
            ) {
                watch.take({ kind, path: below });
            }
        }
    }

    /** Read a node's status. */
    static #status(node: Inode): FileStatus {
        // read the size, a link measured by its target, and the permission bits with the file type
        const kind: FileKind = node.kind;
        const size =
            node.kind === "file" ? node.size : node.kind === "symlink" ? node.target.length : 0;
        const permissions = node.kind === "symlink" ? 0o777 : node.mode;

        return {
            kind,
            size,
            modifiedAt: node.modifiedAt,
            mode: FILE_TYPES[node.kind] | permissions,
        };
    }
}

/** Normalize a path against a directory to an absolute one, `.` and `..` taken by name. */
function normalize(directory: string, path: string): string {
    const names: string[] = [];
    for (const name of (path.startsWith("/") ? path : `${directory}/${path}`).split("/")) {
        // step up for `..`, and take every name but empty ones and `.`
        if (name === "..") {
            names.pop();
        } else if (name !== "" && name !== ".") {
            names.push(name);
        }
    }

    return `/${names.join("/")}`;
}

/** Join a directory's path and a name. */
function join(directory: string, name: string): string {
    return directory === "/" ? `/${name}` : `${directory}/${name}`;
}
