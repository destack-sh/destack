import type { Sqlite3Static, WasmPointer } from "@sqlite.org/sqlite-wasm";

/** The members of a VFS the coalescing VFS passes to the VFS below as they are. */
const VFS_MEMBERS = [
    "xDelete",
    "xAccess",
    "xFullPathname",
    "xRandomness",
    "xSleep",
    "xCurrentTime",
    "xGetLastError",
    "xCurrentTimeInt64",
] as const;

/** The members of a file's methods the coalescing VFS takes from the VFS below unchanged. */
const FILE_MEMBERS = [
    "xLock",
    "xUnlock",
    "xCheckReservedLock",
    "xFileControl",
    "xSectorSize",
    "xDeviceCharacteristics",
] as const;

/** A file method of the VFS below, called with the arguments SQLite passes it. */
type Method = (...values: readonly (number | bigint)[]) => number;

/** The file methods of the VFS below that the coalescing VFS calls. */
interface Below {
    /** Write bytes from the heap at an offset. */
    readonly xWrite: Method;
    /** Read bytes into the heap from an offset. */
    readonly xRead: Method;
    /** Make the written bytes durable. */
    readonly xSync: Method;
    /** Read the file's size. */
    readonly xFileSize: Method;
    /** Cut the file to a size. */
    readonly xTruncate: Method;
    /** Close the file. */
    readonly xClose: Method;
}

/** A write a file holds back: its bytes at an offset. */
interface Write {
    /** The offset in the file. */
    readonly offset: number;
    /** The bytes written. */
    readonly bytes: Uint8Array;
}

/** A VFS over another that passes each file's writes down at its next sync, one per run. */
export class CoalescingVfs {
    /** The name SQLite opens databases under. */
    readonly name: string;
    /** The runtime. */
    readonly #sqlite: Sqlite3Static;
    /** The writes each open file holds back, by file. */
    readonly #held = new Map<WasmPointer, Write[]>();
    /** The file methods below and this VFS's own over them, once the first file opens. */
    #files: { readonly below: Below; readonly methods: WasmPointer } | undefined;

    /** Register a VFS over a registered one, named after it. */
    constructor(sqlite: Sqlite3Static, below: string) {
        // read the VFS below
        const { capi, wasm } = sqlite;
        const pointer = capi.sqlite3_vfs_find(below);
        if (pointer === 0) {
            throw new TypeError(`vfs '${below}' is not registered`);
        }
        const base = new capi.sqlite3_vfs(pointer);
        const open = CoalescingVfs.#function(sqlite, base, "xOpen");
        this.name = `${below}-coalescing`;
        this.#sqlite = sqlite;

        // take the VFS below's members, opening each file below and routing its methods here
        const vfs = new capi.sqlite3_vfs();
        vfs.$iVersion = base.$iVersion;
        vfs.$szOsFile = base.$szOsFile;
        vfs.$mxPathname = base.$mxPathname;
        vfs.pAppData = base.pAppData;
        const members = Object.fromEntries(
            VFS_MEMBERS.map((member) => {
                // call the VFS below as itself, as it finds its state by its own pointer
                const method = CoalescingVfs.#function(sqlite, base, member);

                return [
                    member,
                    (_vfs: WasmPointer, ...values: (number | bigint)[]) =>
                        method(pointer, ...values),
                ];
            }),
        );
        vfs.installMethods({
            ...members,
            xOpen: (
                _vfs: WasmPointer,
                name: WasmPointer,
                file: WasmPointer,
                flags: number,
                outFlags: WasmPointer,
            ) => {
                // open the file below, failing as it fails
                const opened = open(pointer, name, file, flags, outFlags);
                if (opened !== 0) {
                    return opened;
                }

                // route its methods here, the VFS below serving every file one struct
                this.#files ??= this.#over(wasm.peekPtr(file));
                wasm.pokePtr(file, this.#files.methods);
                this.#held.set(file, []);

                return 0;
            },
        });
        sqlite.vfs.installVfs({ vfs: { struct: vfs, name: this.name, methods: {} } });
    }

    /** Build file methods over the ones below, holding writes back until any other access. */
    #over(pointer: WasmPointer): { readonly below: Below; readonly methods: WasmPointer } {
        // read the methods below
        const sqlite = this.#sqlite;
        const { capi, wasm } = sqlite;
        const base = new capi.sqlite3_io_methods(pointer);
        const below: Below = {
            xWrite: CoalescingVfs.#function(sqlite, base, "xWrite"),
            xRead: CoalescingVfs.#function(sqlite, base, "xRead"),
            xSync: CoalescingVfs.#function(sqlite, base, "xSync"),
            xFileSize: CoalescingVfs.#function(sqlite, base, "xFileSize"),
            xTruncate: CoalescingVfs.#function(sqlite, base, "xTruncate"),
            xClose: CoalescingVfs.#function(sqlite, base, "xClose"),
        };

        // pass the locks and controls down as they are
        const methods = new capi.sqlite3_io_methods();
        Reflect.set(methods, "$iVersion", 1);
        for (const member of FILE_MEMBERS) {
            methods.installMethod(member, CoalescingVfs.#pointer(base, member));
        }

        // hold each write back, and pass the held ones down before a sync, read, size, cut or close
        methods.installMethods({
            xWrite: (file: WasmPointer, source: WasmPointer, length: number, offset: bigint) => {
                const bytes = wasm.heap8u().slice(source, source + length);
                this.#writes(file).push({ offset: Number(offset), bytes });

                return 0;
            },
            xSync: (file: WasmPointer, flags: number) =>
                this.#flush(file) || below.xSync(file, flags),
            xRead: (file: WasmPointer, target: WasmPointer, length: number, offset: bigint) =>
                this.#flush(file) || below.xRead(file, target, length, offset),
            xFileSize: (file: WasmPointer, size: WasmPointer) =>
                this.#flush(file) || below.xFileSize(file, size),
            xTruncate: (file: WasmPointer, size: bigint) =>
                this.#flush(file) || below.xTruncate(file, size),
            xClose: (file: WasmPointer) => {
                const flushed = this.#flush(file);
                this.#held.delete(file);

                return flushed || below.xClose(file);
            },
        });

        return { below, methods: methods.pointer };
    }

    /** Pass a file's held writes down in order, one per run, answering the code below. */
    #flush(file: WasmPointer): number {
        // take the held writes
        const held = this.#writes(file);
        if (held.length === 0) {
            return 0;
        }
        this.#held.set(file, []);

        // join each write that continues the one before into its run
        const runs: Write[][] = [];
        for (const write of held) {
            const run = runs.at(-1);
            const last = run?.at(-1);
            const isContinued =
                run !== undefined &&
                last !== undefined &&
                last.offset + last.bytes.length === write.offset;
            if (isContinued) {
                run.push(write);
            } else {
                runs.push([write]);
            }
        }

        // write each run from one heap buffer, stopping at a failure
        const { wasm } = this.#sqlite;
        const below = CoalescingVfs.#required(this.#files, "the file methods below").below;
        for (const run of runs) {
            const length = run.reduce((total, write) => total + write.bytes.length, 0);
            const offset = CoalescingVfs.#required(run[0], "a run's first write").offset;
            const buffer = wasm.alloc(length);
            try {
                let at = buffer;
                for (const write of run) {
                    wasm.heap8u().set(write.bytes, at);
                    at += write.bytes.length;
                }
                const written = below.xWrite(file, buffer, length, BigInt(offset));
                if (written !== 0) {
                    return written;
                }
            } finally {
                wasm.dealloc(buffer);
            }
        }

        return 0;
    }

    /** Read the writes an open file holds back. */
    #writes(file: WasmPointer): Write[] {
        return CoalescingVfs.#required(this.#held.get(file), `the writes of file ${file}`);
    }

    /** Read a function pointer member of a struct as a function. */
    static #function(
        sqlite: Sqlite3Static,
        struct: { readonly pointer: WasmPointer },
        member: string,
    ): Method {
        const entry = sqlite.wasm.functionEntry(CoalescingVfs.#pointer(struct, member));
        if (entry === undefined) {
            throw new TypeError(`struct member '${member}' is no function`);
        }

        return (...values) => Number(Reflect.apply(entry, undefined, values));
    }

    /** Read a function pointer member of a struct. */
    static #pointer(struct: { readonly pointer: WasmPointer }, member: string): WasmPointer {
        // read the member as the struct binder exposes it, with a dollar sign
        const fields: Readonly<Record<string, unknown>> = struct;
        const value = fields[`$${member}`];
        if (typeof value !== "number") {
            throw new TypeError(`struct member '${member}' is no pointer`);
        }

        return value;
    }

    /** Require a value an invariant of this VFS promises. */
    static #required<Value>(value: Value | undefined, what: string): Value {
        if (value === undefined) {
            throw new TypeError(`value '${what}' is missing`);
        }

        return value;
    }
}
