import { type Environment, type FileSystem, Pipe, waitAborted } from "@destack/environment";
import { Digest, type DigestCrypto, found, present } from "@destack/schema";
import type { BlobStorage } from "@destack/resource";
import {
    and,
    asc,
    BLOB_TABLES,
    type BlobStore,
    type DatabaseConnection,
    defineDatabase,
    eq,
    inArray,
    lte,
} from "@destack/db";
import { DependencyName, type DependencyRelease } from "@destack/package";
import { PackageFile } from "@destack/package/file";
import {
    BuildLayer,
    BuildReader,
    type ImportMap,
    Lock,
    MANIFEST_PATH,
    PackageManifest,
    type PackageOutput,
} from "@destack/package/manifest";
import type { Runtime } from "@destack/package/runtime";
import { BuildError } from "../error/index.ts";
import { TarballEntry } from "@destack/package/archive";
import { comparePath } from "../build/serialization.ts";
import type { CacheEntry } from "../cache/entry.ts";
import { type PackageAccess, serveStore } from "./http.ts";
import {
    BUILD_STORE_TABLES,
    buildCache,
    buildManifest,
    buildOutput,
    buildReference,
    buildReferrer,
    buildRoot,
} from "./table.ts";

/** The rows one statement binds: 500, far under SQLite's 32,766 parameters at two columns. */
const BATCH_ROWS = 500;

/** How long a cache entry or a used build lasts past its last use: a week, as Nx does. */
export const CACHE_MILLISECONDS = 7 * 24 * 60 * 60_000;

/** How often a store records a use: daily, a seventh of a cache entry's or build's lifetime. */
export const USE_MILLISECONDS = 24 * 60 * 60_000;

/** The layered files a read keeps in flight, overlapping file reads within bounded memory. */
const LAYER_READS = 64;

/** The prefix of the root a cache entry retains its blobs by, followed by its key. */
const CACHE_ROOT_PREFIX = "cache:";

/** What a closure follows from its root: what the root runs, or every export of the root. */
export type ClosureEntries = "run" | "export";

/** Read a build's manifest at a package's registry, absent where none is stored. */
export type ManifestRead = (manifest: Digest) => Promise<PackageManifest | undefined>;

/** The most bytes a write gathers into a pack or an upload sends: 64 MiB, bounding memory. */
export const MAX_WRITE_BYTES = 64 * 1024 * 1024;

/** Where a blob lies, with its byte count: in the store's layer, or in one of its packs. */
export type BlobLocation =
    | { readonly kind: "layered"; readonly size: number }
    | ({ readonly kind: "packed" } & Awaited<ReturnType<BlobStore["placements"]>>[number]);

/** A blob a transfer sends: a file, a build's manifest, or a lock. */
export interface BuildObject {
    /** What the blob is. */
    readonly kind: "file" | "manifest" | "lock";
    /** The blob's digest. */
    readonly digest: Digest;
    /** The blob's size in bytes. */
    readonly size: number;
}

/** The objects transfers send: listing them and their tarball paths. */
export const BuildObject = {
    /** List the objects of locks and builds for some runtimes: files, manifests, then locks. */
    async of(
        store: Pick<BuildStore, "closure" | "contents" | "head" | "lock">,
        roots: readonly Pick<BuildObject, "kind" | "digest">[],
        runtimes: readonly Runtime[] | "all",
    ): Promise<BuildObject[]> {
        // take every root's builds once, and the builds each runtime imports
        const builds = new Set<Digest>();
        const imported = new Map<Digest, Set<Runtime>>();
        for (const root of roots) {
            const lock = root.kind === "lock" ? await store.lock(root.digest) : undefined;
            for (const build of lock === undefined ? [root.digest] : Lock.builds(lock)) {
                builds.add(build);
            }
            for (const runtime of runtimes === "all" ? [] : runtimes) {
                const reached =
                    lock === undefined
                        ? [root.digest]
                        : await store.closure(lock, runtime, undefined, "export");
                for (const build of reached) {
                    imported.set(build, (imported.get(build) ?? new Set()).add(runtime));
                }
            }
        }

        // take every file of each build, or what the runtimes importing it run
        const files = new Map<Digest, BuildObject>();
        for (const build of builds) {
            const reader = await store.contents(build);
            const listed =
                runtimes === "all"
                    ? await reader.distributed()
                    : await reader.runFiles([...(imported.get(build) ?? [])]);
            for (const file of listed) {
                files.set(file.digest, { kind: "file", digest: file.digest, size: file.size });
            }
        }

        // add the manifests, then the locks
        const manifests = await Promise.all([...builds].map((build) => store.head(build)));
        const locks = await Promise.all(
            roots.filter((root) => root.kind === "lock").map((root) => store.head(root.digest)),
        );

        return [
            ...files.values(),
            ...manifests.map((file): BuildObject => ({
                kind: "manifest",
                digest: file.digest,
                size: file.size,
            })),
            ...locks.map((file): BuildObject => ({
                kind: "lock",
                digest: file.digest,
                size: file.size,
            })),
        ];
    },

    /** Give blobs as file entries for a write. */
    async *files(
        blobs: readonly { readonly digest: Digest; readonly bytes: Uint8Array<ArrayBuffer> }[],
    ): AsyncIterable<TarballEntry> {
        for (const { digest, bytes } of blobs) {
            yield TarballEntry.of(BuildObject.path({ kind: "file", digest }), bytes);
        }
    },

    /** Build an object's tarball entry path from its kind and digest. */
    path(object: Pick<BuildObject, "kind" | "digest">): string {
        return `${object.kind}/${object.digest}`;
    },

    /** Read an object's kind and digest from its tarball entry's path. */
    parse(path: string): Pick<BuildObject, "kind" | "digest"> {
        // read the kind and the digest, refusing anything else
        const [kind, named] = path.split("/");
        const digest = Digest.safeParse(named);
        if ((kind !== "file" && kind !== "manifest" && kind !== "lock") || !digest.success) {
            throw BuildError.of(
                "OBJECT_INVALID",
                `object '${path}' is invalid: it must be a kind and a digest`,
            );
        }

        return { kind, digest: digest.data };
    },
};

/** An object with its bytes, as a store packs it. */
interface StoredObject {
    /** What the object is. */
    readonly kind: BuildObject["kind"];
    /** The object's digest. */
    readonly digest: Digest;
    /** The object's bytes. */
    readonly bytes: Uint8Array<ArrayBuffer>;
}

/** The blobs a store stores builds in, and the environment it runs in. */
export interface BuildStoreOptions<StoreEnvironment, Storage extends BlobStorage> {
    /** The partition's blobs, whose database declares the store's tables beside its own. */
    readonly blobs: BlobStore<Storage>;
    /** The crypto digesting files and the clock of leases. */
    readonly environment: StoreEnvironment;
    /** The read-only builds the store reads in place, such as a machine's shipped builds. */
    readonly layer?: BuildLayer;
    /** How long a stored build lasts unless a root or a use retains it, by default the blobs'. */
    readonly leaseMilliseconds?: number;
}

/** Immutable package builds in a partition's blobs, kept while any of its rows references them. */
export class BuildStore<
    StoreEnvironment extends Pick<Environment, "crypto" | "clock"> = Pick<
        Environment,
        "crypto" | "clock"
    >,
    Storage extends BlobStorage = BlobStorage,
> {
    /** The database of a standalone store, such as a toolchain's on its machine's disk. */
    static readonly database = defineDatabase({
        name: "builds",
        tables: [...BUILD_STORE_TABLES, ...BLOB_TABLES],
    });

    /** The partition's blobs: the builds' files and manifests. */
    readonly blobs: BlobStore<Storage>;
    /** The crypto and clock the store digests and expires with. */
    readonly environment: StoreEnvironment;
    /** The reader of each build opened so far, by manifest digest, until its build is deleted. */
    readonly #readers = new Map<string, Promise<BuildReader>>();
    /** The locks read so far, by digest, as locks never change. */
    readonly #locks = new Map<Digest, Promise<Lock>>();
    /** The builds of cache keys in flight, so callers asking at once build each key once. */
    readonly #building = new Map<Digest, Promise<Digest>>();
    /** The builds in flight to the store, by manifest digest, each stored once. */
    readonly #putting = new Map<Digest, Promise<Digest>>();
    /** The read-only builds the store reads in place. */
    readonly layer: BuildLayer;
    /** How long a stored build lasts before a root or a use retains it, in milliseconds. */
    readonly leaseMilliseconds: number;

    /** Store builds in a partition's blobs, reading some read-only builds in place. */
    constructor(options: BuildStoreOptions<StoreEnvironment, Storage>) {
        // take the blobs, the environment, the layer and the lease
        this.blobs = options.blobs;
        this.environment = options.environment;
        this.layer = options.layer ?? BuildLayer.empty();
        this.leaseMilliseconds = options.leaseMilliseconds ?? options.blobs.leaseMilliseconds;
    }

    /** Read when a lease taken now ends. */
    get #leasedUntil(): number {
        return this.environment.clock.now() + this.leaseMilliseconds;
    }

    /** The partition's database with the store's index. */
    get database(): DatabaseConnection {
        return this.blobs.database;
    }

    /** Serve the store's manifests, files and archives below a mount, as a host allows each. */
    fetch(request: Request, mount: URL, access: PackageAccess): Promise<Response> {
        return serveStore(this, mount, request, access);
    }

    /** Identify a build by the digest of its manifest, as a store stores it. */
    static async digest(manifest: PackageManifest, crypto: DigestCrypto): Promise<Digest> {
        const bytes = PackageManifest.serialize(manifest);

        return Digest.of(bytes, crypto);
    }

    /** Store each missing file of a build, then its manifest, returning the manifest's digest. */
    put(source: BuildReader, signal?: AbortSignal): Promise<Digest> {
        return this.store(
            source,
            async (files) => {
                await this.blobs.put(
                    files.map((file) => ({
                        digest: file.digest,
                        size: file.size,
                        open: () => source.stream(file.path),
                    })),
                    this.#leasedUntil,
                );
            },
            signal,
        );
    }

    /** Store a build, writing the files the store lacks with a write, joining one in flight. */
    protected async store(
        source: BuildReader,
        write: (files: readonly PackageFile[]) => Promise<void>,
        signal?: AbortSignal,
    ): Promise<Digest> {
        // join the storing of the build in flight, or begin it, forgetting it once it settles
        signal?.throwIfAborted();
        const digest = await source.digest();
        let storing = this.#putting.get(digest);
        if (storing === undefined) {
            storing = this.#storeOnce(source, digest, write);
            this.#putting.set(digest, storing);
            const forget = () => this.#putting.delete(digest);
            void storing.then(forget, forget);
        }

        // wait for it under this caller's own signal, the storing going on for the others
        return await (signal === undefined
            ? storing
            : Promise.race([
                  storing,
                  waitAborted(signal).then(() => {
                      throw signal.reason;
                  }),
              ]));
    }

    /** Store a build unless the store has it, under no single caller's signal. */
    async #storeOnce(
        source: BuildReader,
        digest: Digest,
        write: (files: readonly PackageFile[]) => Promise<void>,
    ): Promise<Digest> {
        return (await this.contains(digest))
            ? digest
            : await this.#storeFiles(
                  source,
                  digest,
                  PackageManifest.serialize(source.manifest),
                  write,
              );
    }

    /** Store a build's missing files, then its manifest and record. */
    async #storeFiles(
        source: BuildReader,
        digest: Digest,
        bytes: Uint8Array<ArrayBuffer>,
        write: (files: readonly PackageFile[]) => Promise<void>,
    ): Promise<Digest> {
        // lease the build's blobs, refusing reserved and duplicate file paths
        const list = await source.distributed();
        requireDistinctPaths(list);
        const files = distinctFiles(list);
        await this.blobs.lease([digest, ...files.map((file) => file.digest)], this.#leasedUntil);

        // store the missing files at once, then the manifest
        const missing = new Set(await this.blobs.missing(files.map((file) => file.digest)));
        await write(files.filter((file) => missing.has(file.digest)));
        await this.#write(digest, bytes);

        // record the build with its files
        const referenced = files.map((file) => file.digest);
        await this.database.transaction((transaction) =>
            BuildStore.#recordManifest(transaction, digest, bytes.byteLength, referenced),
        );

        return digest;
    }

    /** Report whether the store has a build, recorded once every blob it references is stored. */
    async contains(digest: Digest): Promise<boolean> {
        return (await this.#recorded(digest))?.isComplete === true;
    }

    /** Report whether the store has an output of a build: the whole build, or that output. */
    async containsOutput(digest: Digest, output: string): Promise<boolean> {
        // take a whole build as containing each output
        if (await this.contains(digest)) {
            return true;
        }

        // find the output a pull stored
        const [row] = await this.database
            .select({ name: buildOutput.name })
            .from(buildOutput)
            .where(and(eq(buildOutput.manifest, digest), eq(buildOutput.name, output)))
            .limit(1);

        return row !== undefined;
    }

    /** Tell which of some blobs the store has, one flag per digest, its layer's among them. */
    async has(digests: readonly Digest[]): Promise<boolean[]> {
        const located = await this.locate(digests);

        return digests.map((digest) => located.has(digest));
    }

    /** Read objects as tarball entries, in the order given. */
    async *read(
        objects: readonly Pick<BuildObject, "kind" | "digest">[],
    ): AsyncIterable<TarballEntry> {
        // find where the files lie at once
        const located = await this.locate(
            objects.filter((object) => object.kind === "file").map((object) => object.digest),
        );
        let run: { kind: BlobLocation["kind"]; digests: Digest[] } = {
            kind: "packed",
            digests: [],
        };
        const flush = () =>
            run.kind === "packed" ? this.#batch(run.digests) : this.#layered(run.digests);
        for (const object of objects) {
            const location = object.kind === "file" ? located.get(object.digest) : undefined;

            // gather a file with its neighbours that lie alike, read in one run
            if (location !== undefined) {
                if (run.kind !== location.kind) {
                    yield* flush();
                    run = { kind: location.kind, digests: [] };
                }
                run.digests.push(object.digest);
                continue;
            }
            yield* flush();
            run = { kind: "packed", digests: [] };

            // refuse a missing file
            if (object.kind === "file") {
                throw BuildError.of("BLOB_NOT_FOUND", `package file '${object.digest}' not found`);
            }

            // stream a manifest or lock as recorded
            const head = await this.head(object.digest);
            yield { path: BuildObject.path(object), size: head.size, body: this.#stream(head) };
        }
        yield* flush();
    }

    /** Read layered files in order as tarball entries. */
    async *#layered(digests: readonly Digest[]): AsyncIterable<TarballEntry> {
        const files = digests.map((digest) => ({ kind: "file" as const, digest }));
        for await (const { kind, digest, bytes } of this.#readLayered(files)) {
            const body = new ReadableStream<Uint8Array>({
                start(controller) {
                    controller.enqueue(bytes);
                    controller.close();
                },
            });
            yield { path: BuildObject.path({ kind, digest }), size: bytes.byteLength, body };
        }
    }

    /** Read layered objects in order, keeping some reads in flight to overlap them. */
    async *#readLayered(
        objects: readonly Pick<BuildObject, "kind" | "digest">[],
    ): AsyncIterable<StoredObject> {
        // start the first reads, each later one as an earlier one is taken
        const reads: Promise<Uint8Array<ArrayBuffer>>[] = [];
        const start = (digest: Digest) => {
            const reading = present(this.layer.read(digest), `the layered blob ${digest}`);
            reading.catch(() => undefined);
            reads.push(reading);
        };
        for (const object of objects.slice(0, LAYER_READS)) {
            start(object.digest);
        }

        // take each read in order
        for (const [index, object] of objects.entries()) {
            const bytes = await present(reads[index], `the read of ${object.digest}`);
            const next = objects[index + LAYER_READS];
            if (next !== undefined) {
                start(next.digest);
            }
            yield { kind: object.kind, digest: object.digest, bytes };
        }
    }

    /** Read packed files in one batch of their blobs, as tarball entries in order. */
    async *#batch(digests: readonly Digest[]): AsyncIterable<TarballEntry> {
        for await (const { digest, size, body } of this.blobs.batch(digests)) {
            yield { path: BuildObject.path({ kind: "file", digest }), size, body };
        }
    }

    /** Write objects files first, recording the manifests and locks of each pack once stored. */
    async write(entries: AsyncIterable<TarballEntry>): Promise<void> {
        await this.#pack(BuildStore.#parsed(entries), "verify");
    }

    /** Store objects in packs of at most the write limit, then record their manifests and locks. */
    async #pack(objects: AsyncIterable<StoredObject>, digests: "verify" | "trust"): Promise<void> {
        // gather objects into packs of at most the write limit
        let gathered: StoredObject[] = [];
        let size = 0;
        const flush = async () => {
            // store the pack, verifying digests from outside, then record its manifests and locks from their bytes
            if (digests === "verify") {
                await this.#verify(gathered);
            }
            await this.#put(gathered);
            const buffers = new Map(gathered.map(({ digest, bytes }) => [digest, bytes]));
            await this.#recordObjects(
                gathered.filter((object) => object.kind !== "file"),
                buffers,
            );
            gathered = [];
            size = 0;
        };
        for await (const object of objects) {
            // write the pack before an object that would take it past the limit, then gather it
            if (gathered.length > 0 && size + object.bytes.byteLength > MAX_WRITE_BYTES) {
                await flush();
            }
            gathered.push(object);
            size += object.bytes.byteLength;
        }
        await flush();
    }

    /** Read tarball entries as objects, refusing one past the write limit. */
    static async *#parsed(entries: AsyncIterable<TarballEntry>): AsyncIterable<StoredObject> {
        for await (const entry of entries) {
            // refuse an object past the limit
            const object = BuildObject.parse(entry.path);
            if (entry.size > MAX_WRITE_BYTES) {
                throw BuildError.of(
                    "OBJECT_INVALID",
                    `object '${object.digest}' is invalid: it exceeds ${MAX_WRITE_BYTES} bytes`,
                );
            }

            // read its bytes
            yield {
                kind: object.kind,
                digest: object.digest,
                bytes: await TarballEntry.bytes(entry),
            };
        }
    }

    /** Pull a lock's or build's objects for some runtimes from another store, a read per round. */
    async pull(
        source: Pick<BuildStore, "read">,
        root: Pick<BuildObject, "kind" | "digest">,
        runtimes: readonly Runtime[] | "all",
    ): Promise<void> {
        // buffer the objects this store lacks, a read per round
        const buffers = new Map<Digest, Uint8Array<ArrayBuffer>>();
        const gather = async (objects: readonly Pick<BuildObject, "kind" | "digest">[]) => {
            for await (const entry of source.read(await this.#lacking(objects))) {
                buffers.set(BuildObject.parse(entry.path).digest, await TarballEntry.bytes(entry));
            }
        };

        // read the lock, from its buffer where pulled
        if (root.kind === "lock") {
            await gather([root]);
        }
        const bytes = buffers.get(root.digest);
        const lock =
            root.kind !== "lock"
                ? undefined
                : bytes === undefined
                  ? await this.lock(root.digest)
                  : Lock.parse(BuildStore.#json(bytes));

        // open each build, a pulled one over its buffered lists
        const builds = lock === undefined ? [root.digest] : Lock.builds(lock);
        await gather(builds.map((build) => ({ kind: "manifest", digest: build })));
        const readers = new Map<Digest, BuildReader>();
        for (const build of builds) {
            readers.set(build, await this.#pulledReader(build, buffers, readers));
        }

        // pull the lists, refusing a pulled build whose lists lack their formats
        const lists = [...readers.values()].flatMap((reader) => reader.references());
        await gather(lists.map((file) => ({ kind: "file", digest: file.digest })));
        for (const [build, reader] of readers) {
            if (buffers.has(build)) {
                await reader.check();
            }
        }

        // list the objects the runtimes need, the buffered manifests and lock among them
        const view = {
            lock: async () => present(lock, `the lock ${root.digest}`),
            contents: async (build: Digest) => found(readers, build),
            head: async (object: Digest) => {
                const buffer = buffers.get(object);

                return buffer === undefined
                    ? await this.head(object)
                    : BuildStore.#manifestFile(object, buffer.byteLength);
            },
            closure: (
                pulled: Lock,
                runtime: Runtime,
                read?: ManifestRead,
                entries?: ClosureEntries,
            ) =>
                this.closure(
                    pulled,
                    runtime,
                    read ?? (async (build) => found(readers, build).manifest),
                    entries,
                ),
        };
        const objects = await BuildObject.of(view, [root], runtimes);

        // write the files neither stored nor buffered, then the buffered objects
        const stored = await this.has(objects.map((object) => object.digest));
        const files = objects.filter(
            (object, index) =>
                object.kind === "file" && stored[index] !== true && !buffers.has(object.digest),
        );
        await this.write(
            (async function* () {
                yield* source.read(files);
                for (const object of objects.filter((each) => buffers.has(each.digest))) {
                    yield TarballEntry.of(BuildObject.path(object), found(buffers, object.digest));
                }
            })(),
        );
    }

    /** List the objects the store lacks: unstored files, unrecorded manifests and locks. */
    async #lacking(
        objects: readonly Pick<BuildObject, "kind" | "digest">[],
    ): Promise<Pick<BuildObject, "kind" | "digest">[]> {
        const stored = await this.has(objects.map((object) => object.digest));
        const isHere = await Promise.all(
            objects.map(async (object, index) =>
                object.kind === "file"
                    ? stored[index] === true
                    : (await this.#head(object.digest)) !== undefined,
            ),
        );

        return objects.filter((_object, index) => isHere[index] !== true);
    }

    /** Open a pulled build over its buffered lists, else the stored build. */
    async #pulledReader(
        build: Digest,
        buffers: ReadonlyMap<Digest, Uint8Array<ArrayBuffer>>,
        readers: ReadonlyMap<Digest, BuildReader>,
    ): Promise<BuildReader> {
        const bytes = buffers.get(build);
        const stream = async (path: string) => this.#streamList(buffers, readers, build, path);

        return bytes === undefined
            ? await this.contents(build)
            : new BuildReader(
                  PackageManifest.parse(BuildStore.#json(bytes)),
                  stream,
                  this.environment.crypto,
              );
    }

    /** Store a lock once every build it lists is recorded, returning its digest. */
    async putLock(lock: Lock): Promise<Digest> {
        // write the lock's canonical bytes under their digest
        const bytes = Lock.serialize(lock);
        const digest = await Digest.of(bytes, this.environment.crypto);
        await this.write(
            (async function* () {
                yield TarballEntry.of(BuildObject.path({ kind: "lock", digest }), bytes);
            })(),
        );

        return digest;
    }

    /** List the stored manifests and locks referencing a blob or build. */
    async referencing(digest: Digest): Promise<Digest[]> {
        const rows = await this.database
            .select({ manifest: buildReference.manifest })
            .from(buildReference)
            .where(eq(buildReference.digest, digest));

        return rows.map((row) => row.manifest);
    }

    /** Read a stored lock's blob once, as locks never change, recorded or not. */
    lock(digest: Digest): Promise<Lock> {
        // read the lock's bytes, forgetting a failed read
        const known = this.#locks.get(digest);
        if (known !== undefined) {
            return known;
        }
        const reading = (async () => {
            const location = present((await this.locate([digest])).get(digest), `lock '${digest}'`);
            const body = BuildStore.#readable(this.#bytes(digest, location));

            return Lock.deserialize(new Uint8Array(await new Response(body).arrayBuffer()));
        })();
        this.#locks.set(digest, reading);
        reading.catch(() => this.#locks.delete(digest));

        return reading;
    }

    /** Read a lock's builds by package name, its root's among them, each with its package. */
    async entriesOf(lock: Lock): Promise<Record<string, DependencyRelease>> {
        return await Lock.entries(lock, (digest) => this.get(digest));
    }

    /** Record a lock whose blobs the store has, as a moved space's: each build, then the lock. */
    async record(digest: Digest): Promise<void> {
        // read the lock from the blobs
        const stream = BuildStore.#readable(this.blobs.read(digest));
        const bytes = new Uint8Array(await new Response(stream).arrayBuffer());
        const lock = Lock.parse(
            JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes)),
        );

        // record each build, then the lock
        const builds = Lock.builds(lock).map(
            (build) => ({ kind: "manifest", digest: build }) as const,
        );
        await this.#recordObjects([...builds, { kind: "lock", digest }], new Map());
    }

    /** Attach a stored blob to a stored build as its referrer at a path, unless one exists. */
    async attach(subject: Digest, file: PackageFile): Promise<void> {
        // require the build and the blob
        await this.head(subject);
        await this.#requireStored(subject, [file]);

        // record the referrer, unless another writer recorded one first
        await this.database
            .insert(buildReferrer)
            .values({
                subject,
                path: file.path,
                digest: file.digest,
                size: file.size,
                mediaType: file.mediaType,
            })
            .onConflictDoNothing();
    }

    /** List a stored manifest's referrers, as OCI's referrers API lists a subject's artifacts. */
    async referrers(subject: Digest): Promise<PackageFile[]> {
        const rows = await this.database
            .select({
                path: buildReferrer.path,
                digest: buildReferrer.digest,
                size: buildReferrer.size,
                mediaType: buildReferrer.mediaType,
            })
            .from(buildReferrer)
            .where(eq(buildReferrer.subject, subject));

        return rows.toSorted((left, right) => comparePath(left, right));
    }

    /** Read the file attached to a manifest at a path, absent for a manifest without one. */
    async referrer(subject: Digest, path: string): Promise<PackageFile | undefined> {
        const [row] = await this.database
            .select({
                path: buildReferrer.path,
                digest: buildReferrer.digest,
                size: buildReferrer.size,
                mediaType: buildReferrer.mediaType,
            })
            .from(buildReferrer)
            .where(and(eq(buildReferrer.subject, subject), eq(buildReferrer.path, path)));

        return row;
    }

    /** Read a build's manifest, verified and parsed once with its reader. */
    async get(digest: Digest): Promise<PackageManifest> {
        return (await this.contents(digest)).manifest;
    }

    /** Open the root build of a stored lock. */
    async root(lock: Digest): Promise<BuildReader> {
        return await this.contents((await this.lock(lock)).root);
    }

    /** Read a stored build's manifest, absent for a build the store does not record. */
    async find(digest: Digest): Promise<PackageManifest | undefined> {
        return (await this.#find(digest))?.manifest;
    }

    /** Open a stored build for selective reads, one reader per build, as its files never change. */
    async contents(digest: Digest): Promise<BuildReader> {
        const contents = await this.#find(digest);
        if (contents === undefined) {
            throw BuildError.of("MANIFEST_NOT_FOUND", `package manifest '${digest}' not found`);
        }

        return contents;
    }

    /** Open a stored build for selective reads, absent for a build the store does not record. */
    async #find(digest: Digest): Promise<BuildReader | undefined> {
        // answer absent for a build the store no longer records, whose reader may outlive it
        const head = await this.#head(digest);
        if (head === undefined) {
            return undefined;
        }

        // reuse the build's reader, forgetting one whose opening failed
        const known = this.#readers.get(digest);
        if (known !== undefined) {
            return await known;
        }
        const opening = this.#open(head);
        this.#readers.set(digest, opening);
        opening.catch(() => this.#readers.delete(digest));

        return await opening;
    }

    /** Open a stored build's reader over its manifest's listed files. */
    async #open(head: PackageFile): Promise<BuildReader> {
        // read and verify the manifest, streaming each file the reader finds by its path
        const bytes = await this.file(head);
        const reader: BuildReader = new BuildReader(
            PackageManifest.deserialize(bytes),
            async (path, signal) => {
                signal?.throwIfAborted();
                const stream = this.#stream(await reader.file(path));

                return signal ? Pipe.through(stream, new TransformStream(), signal) : stream;
            },
            this.environment.crypto,
        );

        return reader;
    }

    /** Read a cache key's entry, recording its use once a day, absent for a miss. */
    async cached(key: Digest): Promise<CacheEntry | undefined> {
        // read the entry, a root of what it references while it lives
        const [row] = await this.database
            .select({ entry: buildCache.entry, usedAt: buildCache.usedAt })
            .from(buildCache)
            .where(eq(buildCache.key, key));

        // record a hit's use once a day
        const now = this.environment.clock.now();
        if (row !== undefined && now - row.usedAt >= USE_MILLISECONDS) {
            await this.database
                .update(buildCache)
                .set({ usedAt: now })
                .where(eq(buildCache.key, key));
        }

        return row?.entry;
    }

    /** Store an entry under a cache key, once the manifest or files it references are stored. */
    async cache(key: Digest, entry: CacheEntry): Promise<void> {
        // record the entry at the use time
        const usedAt = this.environment.clock.now();
        await this.database.transaction(async (transaction) => {
            await transaction
                .insert(buildCache)
                .values({ key, entry, usedAt })
                .onConflictDoUpdate({ target: buildCache.key, set: { entry, usedAt } });

            // retain what it references under its root
            await BuildStore.#replaceRoot(
                transaction,
                `${CACHE_ROOT_PREFIX}${key}`,
                new Set(BuildStore.#digests(entry)),
            );
        });
    }

    /** Take the build a key caches, else build and cache it once for all callers at once. */
    cachedBuild(key: Digest, build: () => Promise<Digest>): Promise<Digest> {
        // join a build of the key in flight
        const flying = this.#building.get(key);
        if (flying !== undefined) {
            return flying;
        }

        // build the key once, forgetting the build once it settles
        const building = (async () => {
            // take the cached build, else build and cache its manifest
            const entry = await this.cached(key);
            if (entry?.kind === "build") {
                return entry.manifest;
            }
            const built = await build();
            await this.cache(key, { kind: "build", manifest: built });

            return built;
        })();
        this.#building.set(key, building);
        void building.finally(() => this.#building.delete(key)).catch(() => undefined);

        return building;
    }

    /** Lease a build or file until a moment, retaining it and what it references until then. */
    async lease(digest: Digest, expiresAt: Date): Promise<void> {
        await this.blobs.lease([digest], expiresAt.getTime());
    }

    /** Retain builds under a root's name, such as a space's, until it retains others. */
    async retain(name: string, manifests: ReadonlySet<Digest>): Promise<void> {
        await this.database.transaction((transaction) =>
            BuildStore.#replaceRoot(transaction, name, manifests),
        );
    }

    /** Forget a root, leaving what only it retained to a collection. */
    async forget(name: string): Promise<void> {
        await this.retain(name, new Set());
    }

    /** Copy in what builds and locks have only through the layer, as overlayfs does. */
    async copyUp(roots: readonly Pick<BuildObject, "kind" | "digest">[]): Promise<void> {
        // copy nothing without a layer
        if (this.layer.isEmpty || roots.length === 0) {
            return;
        }

        // list each build's objects and each lock's own record, of the roots the store has
        const unique = [...new Map(roots.map((root) => [root.digest, root])).values()];
        const heads = await Promise.all(
            unique.map(async (root) => ({ ...root, head: await this.#head(root.digest) })),
        );
        const builds = heads.filter((root) => root.kind === "manifest" && root.head !== undefined);
        const locks = heads.flatMap((root): BuildObject[] =>
            root.kind === "lock" && root.head !== undefined
                ? [{ kind: "lock", digest: root.digest, size: root.head.size }]
                : [],
        );
        const objects = [...(await BuildObject.of(this, builds, "all")), ...locks];

        // find the layered objects the store lacks: files without a pack, records not complete
        const layered = objects.filter((object) =>
            object.kind === "file"
                ? this.layer.size(object.digest) !== undefined
                : this.layer.head(object.digest) !== undefined,
        );
        const files = layered
            .filter((object) => object.kind === "file")
            .map((object) => object.digest);
        const packed = new Set((await this.blobs.placements(files)).map(({ digest }) => digest));
        const records = layered
            .filter((object) => object.kind !== "file")
            .map((object) => object.digest);
        const rows = new Set<Digest>();
        for (let start = 0; start < records.length; start += BATCH_ROWS) {
            const stored = await this.database
                .select({ digest: buildManifest.digest })
                .from(buildManifest)
                .where(
                    and(
                        inArray(buildManifest.digest, records.slice(start, start + BATCH_ROWS)),
                        eq(buildManifest.isComplete, true),
                    ),
                );
            for (const row of stored) {
                rows.add(row.digest);
            }
        }
        const copied = layered.filter((object) =>
            object.kind === "file" ? !packed.has(object.digest) : !rows.has(object.digest),
        );

        // store them from the layer, trusted as the host's own
        await this.#pack(this.#readLayered(copied), "trust");
    }

    /** Read the builds each root retains, by root name, the cache entries' roots left out. */
    async roots(): Promise<ReadonlyMap<string, ReadonlySet<string>>> {
        // group the root rows by name, the cache entries' roots left out
        const rows = await this.database.select().from(buildRoot);
        const roots = new Map<string, Set<string>>();
        for (const row of rows.filter((each) => !each.name.startsWith(CACHE_ROOT_PREFIX))) {
            const retained = roots.get(row.name) ?? new Set<string>();
            retained.add(row.digest);
            roots.set(row.name, retained);
        }

        return roots;
    }

    /** Drop cache entries unused for a week with their roots, answering the next one's end. */
    async expire(): Promise<number | undefined> {
        // drop the unused entries and their roots, which retires what only they kept
        const now = this.environment.clock.now();
        await this.database.transaction(async (transaction) => {
            const unused = await transaction
                .delete(buildCache)
                .where(lte(buildCache.usedAt, now - CACHE_MILLISECONDS))
                .returning({ key: buildCache.key });
            await transaction.delete(buildRoot).where(
                inArray(
                    buildRoot.name,
                    unused.map(({ key }) => `${CACHE_ROOT_PREFIX}${key}`),
                ),
            );
        });

        // look again once the oldest entry left ends
        const [oldest] = await this.database
            .select({ usedAt: buildCache.usedAt })
            .from(buildCache)
            .orderBy(asc(buildCache.usedAt))
            .limit(1);

        return oldest === undefined ? undefined : oldest.usedAt + CACHE_MILLISECONDS;
    }

    /** Evict the least recently used entries and leases until the blobs fit a size, while alone. */
    async evict(maxBytes: number): Promise<void> {
        while ((await this.blobs.size()) > maxBytes) {
            // take the least recently used: the oldest cache entry, else the blobs a use leased longest ago
            const [entry] = await this.database
                .select({ key: buildCache.key })
                .from(buildCache)
                .orderBy(asc(buildCache.usedAt))
                .limit(1);
            const leased = entry === undefined ? await this.blobs.oldestLeased() : [];
            if (entry === undefined && leased.length === 0) {
                return;
            }

            // drop the entry and its root, or end the leases
            if (entry !== undefined) {
                await this.database.transaction(async (transaction) => {
                    await transaction.delete(buildCache).where(eq(buildCache.key, entry.key));
                    await transaction
                        .delete(buildRoot)
                        .where(eq(buildRoot.name, `${CACHE_ROOT_PREFIX}${entry.key}`));
                });
            } else {
                await this.blobs.release(leased);
            }

            // delete what nothing retains at once, as no other process reads the store
            await this.blobs.collect(0);
        }
    }

    /** Delete every build's index, as the partition leaves its host. */
    async clear(): Promise<void> {
        await this.database.transaction(async (transaction) => {
            for (const table of [
                buildReference,
                buildReferrer,
                buildRoot,
                buildCache,
                buildManifest,
            ]) {
                await transaction.delete(table);
            }
        });
        this.#readers.clear();
    }

    /** Read the builds a lock's root imports on a runtime, from what it runs or every export. */
    async closure(
        lock: Lock,
        runtime: Runtime,
        read: ManifestRead = (digest) => this.get(digest),
        entries: ClosureEntries = "run",
    ): Promise<ReadonlySet<Digest>> {
        // start at each output's workloads and the exports its views mount, or at every export
        const manifests = new Map<Digest, Promise<PackageManifest | undefined>>();
        const once: ManifestRead = (digest) => {
            const reading = manifests.get(digest) ?? read(digest);
            manifests.set(digest, reading);

            return reading;
        };
        const first = await once(lock.root);
        const pending = outputsOn(present(first, `the root of lock ${lock.root}`), runtime).flatMap(
            (output) => {
                const mounted = new Set(Object.values(output.views).map((view) => view.entrypoint));
                const started =
                    entries === "export"
                        ? Object.keys(output.exports)
                        : [
                              ...Object.values(output.workloads).map((each) => each.entrypoint),
                              ...Object.entries(output.exports)
                                  .filter(([, exported]) => mounted.has(exported.path))
                                  .map(([subpath]) => subpath),
                          ];

                return started.map((subpath) => ({ manifest: lock.root, subpath }));
            },
        );

        // follow each export once to the lock's builds its chunks import
        const closure = new Set<Digest>([lock.root]);
        const visited = new Set<string>();
        const expanded = new Set<Digest>();
        const units: Digest[] = [];
        for (let next = pending.pop(); next !== undefined; next = pending.pop()) {
            const key = `${next.manifest} ${next.subpath}`;
            const opened = visited.has(key) ? undefined : await once(next.manifest);
            visited.add(key);
            if (opened !== undefined) {
                // follow every export of the root
                const output = present(
                    outputsOn(opened, runtime)[0],
                    `the ${runtime} output of ${next.manifest}`,
                );
                if (
                    entries === "export" &&
                    next.manifest === lock.root &&
                    !expanded.has(next.manifest)
                ) {
                    expanded.add(next.manifest);
                    for (const subpath of Object.keys(output.exports)) {
                        pending.push({ ...next, subpath });
                    }
                }

                // take each imported package's build
                const { imports } = present(
                    output.exports[next.subpath],
                    `the export ${next.subpath} of ${next.manifest}`,
                );
                for (const specifier of imports) {
                    const build = Lock.resolve(lock, next.manifest, DependencyName.of(specifier));
                    const imported = build === undefined ? undefined : await once(build);
                    if (
                        build !== undefined &&
                        imported !== undefined &&
                        !PackageManifest.isPackage(imported)
                    ) {
                        units.push(build);
                    } else if (build !== undefined) {
                        closure.add(build);
                        pending.push({
                            manifest: build,
                            subpath: DependencyName.subpath(specifier),
                        });
                    }
                }
            }
        }

        // add each imported npm unit with the units it links
        for (let unit = units.pop(); unit !== undefined; unit = units.pop()) {
            if (!closure.has(unit)) {
                closure.add(unit);
                units.push(...Object.values(Lock.links(lock, unit)));
            }
        }

        return closure;
    }

    /** Map a lock's modules on a runtime below a URL prefix, builtins at their linked builds. */
    async imports(
        lock: Lock,
        runtime: Runtime,
        base: string,
        linked: ReadonlyMap<string, Lock>,
        read: (digest: Digest) => Promise<PackageManifest> = async (digest) =>
            present(await this.get(digest), `the manifest of build ${digest}`),
    ): Promise<ImportClosure> {
        // read the builds the root's modules import on the runtime
        const closure = await this.closure(lock, runtime, read);
        const links = [...linked.values()].flatMap((each) => Object.entries(each.builds));
        const builtins = Lock.of(lock.root, Object.fromEntries(links));

        // map each dependency's exports in its dependent's scope
        const map: ImportMap = { imports: {}, scopes: {} };
        const builds = new Set<Digest>([lock.root]);
        const visited = new Set<string>([`${lock.root} app`]);
        const pending = [{ digest: lock.root, scope: map.imports, isLinked: false }];
        for (const { digest, scope, isLinked } of pending) {
            for (const output of outputsOn(await read(digest), runtime)) {
                // take each imported package's build
                const names = new Set(
                    Object.values(output.exports).flatMap((exported) =>
                        exported.imports.map((specifier) => DependencyName.of(specifier)),
                    ),
                );
                const dependencies = [...names].flatMap((name) => {
                    // take the lock's build of a name
                    const locked = isLinked ? undefined : Lock.resolve(lock, digest, name);
                    const isLocked = locked !== undefined && closure.has(locked);
                    const manifest = isLocked
                        ? locked
                        : ((Lock.lists(builtins, digest)
                              ? Lock.resolve(builtins, digest, name)
                              : undefined) ?? linked.get(name)?.root);

                    return manifest === undefined ? [] : [{ name, manifest, isLinked: !isLocked }];
                });
                for (const dependency of dependencies) {
                    // map the dependency's exports to its files below its build's prefix
                    const { name, manifest } = dependency;
                    const prefix = `${base}${manifest}/`;
                    for (const each of outputsOn(await read(manifest), runtime)) {
                        for (const [subpath, exported] of Object.entries(each.exports)) {
                            const specifier = subpath === "." ? name : `${name}${subpath.slice(1)}`;
                            scope[specifier] = `${prefix}${exported.path}`;
                        }
                    }

                    // map the dependency's own imports in its scope once per side
                    const side = `${manifest} ${dependency.isLinked ? "linked" : "app"}`;
                    builds.add(manifest);
                    if (!visited.has(side)) {
                        visited.add(side);
                        pending.push({
                            digest: manifest,
                            scope: (map.scopes[prefix] ??= {}),
                            isLinked: dependency.isLinked,
                        });
                    }
                }
            }
        }

        return { map, builds };
    }

    /** Find a layered build's or lock's directory, which runtimes run in place. */
    layered(digest: Digest): string | undefined {
        return this.layer.directory(digest);
    }

    /** List the real directories the store's layer keeps files in, which its readers read. */
    layerDirectories(): Promise<readonly string[]> {
        return this.layer.directories();
    }

    /** Clone a layered file to a path from its layer, refusing one it lacks. */
    protected async placeLayered(
        file: PackageFile,
        destination: string,
        files: FileSystem,
    ): Promise<void> {
        const layered = present(this.layer.file(file.digest), `the layered file ${file.path}`);
        await files.copyFile(layered, destination);
    }

    /** Read one complete file and verify its exact bytes. */
    async file(file: PackageFile): Promise<Uint8Array<ArrayBuffer>> {
        // read and verify the file bytes
        const bytes = new Uint8Array(await new Response(this.#stream(file)).arrayBuffer());
        await PackageFile.verify(file, bytes, this.environment.crypto);

        return bytes;
    }

    /** Read many stored files whole from where each lies. */
    async *batch(
        files: readonly PackageFile[],
    ): AsyncIterable<{ readonly file: PackageFile; readonly bytes: Uint8Array<ArrayBuffer> }> {
        const located = await this.locate(files.map((file) => file.digest));
        for (const file of files) {
            const location = present(located.get(file.digest), `package file '${file.path}'`);
            const body = BuildStore.#readable(this.#bytes(file.digest, location));
            yield { file, bytes: new Uint8Array(await new Response(body).arrayBuffer()) };
        }
    }

    /** Describe a stored build's manifest file, leaving its bytes unread. */
    async head(digest: Digest): Promise<PackageFile> {
        const head = await this.#head(digest);
        if (head === undefined) {
            throw BuildError.of("MANIFEST_NOT_FOUND", `package manifest '${digest}' not found`);
        }

        return head;
    }

    /** Describe a manifest's or lock's file, absent for one the store does not record. */
    async #head(digest: Digest): Promise<PackageFile | undefined> {
        const record = await this.#recorded(digest);

        return record === undefined ? undefined : BuildStore.#manifestFile(digest, record.size);
    }

    /** Describe a manifest's or lock's file by its digest and size. */
    static #manifestFile(digest: Digest, size: number): PackageFile {
        return { path: MANIFEST_PATH, digest, size, mediaType: "application/json" };
    }

    /** Read a manifest's or lock's record: the layer's, complete, else the store's row. */
    async #recorded(
        digest: Digest,
    ): Promise<{ readonly size: number; readonly isComplete: boolean } | undefined> {
        // take a layered manifest or lock, which the layer ships complete
        const layered = this.layer.head(digest);
        if (layered !== undefined) {
            return { size: layered.size, isComplete: true };
        }

        // read the stored row
        const [stored] = await this.database
            .select({ size: buildManifest.size, isComplete: buildManifest.isComplete })
            .from(buildManifest)
            .where(eq(buildManifest.digest, digest));

        return stored;
    }

    /** Find where each blob lies: the layer first, then the store's packs, absent for neither. */
    protected async locate(digests: readonly Digest[]): Promise<Map<Digest, BlobLocation>> {
        // take the layered blobs, then find the others in the packs
        const located = new Map<Digest, BlobLocation>();
        for (const digest of digests) {
            const size = this.layer.size(digest);
            if (size !== undefined) {
                located.set(digest, { kind: "layered", size });
            }
        }
        const packed = digests.filter((digest) => !located.has(digest));
        for (const placement of await this.blobs.placements(packed)) {
            located.set(placement.digest, { kind: "packed", ...placement });
        }

        return located;
    }

    /** Read a range of a located blob: in place from the layer, else from its pack. */
    async *#bytes(
        digest: Digest,
        location: BlobLocation,
        offset = 0,
        length = Number.POSITIVE_INFINITY,
    ): AsyncIterable<Uint8Array> {
        // cut the range from a layered blob's bytes
        if (location.kind === "layered") {
            const bytes = await present(this.layer.read(digest), `the layered blob ${digest}`);
            yield bytes.subarray(offset, Math.min(bytes.byteLength, offset + length));
        }
        // read the range from its pack
        else {
            const end = Math.min(location.size, offset + length);
            yield* this.blobs.storage.slice(location.pack, location.offset + offset, end - offset);
        }
    }

    /** Stream a stored file or manifest, or a range of it. */
    open(
        file: PackageFile,
        range?: { readonly offset: number; readonly length: number },
    ): ReadableStream<Uint8Array> {
        return range === undefined
            ? this.#stream(file)
            : BuildStore.#readable(this.#slice(file, range.offset, range.length));
    }

    /** Stream a blob from where it lies, refusing a missing one by its file's path. */
    #stream(file: PackageFile): ReadableStream<Uint8Array> {
        return BuildStore.#readable(this.#slice(file, 0, Number.POSITIVE_INFINITY));
    }

    /** Read a range of a blob from where it lies, refusing a missing one by its file's path. */
    async *#slice(file: PackageFile, offset: number, length: number): AsyncIterable<Uint8Array> {
        const location = (await this.locate([file.digest])).get(file.digest);
        if (location === undefined) {
            throw BuildError.of("BLOB_NOT_FOUND", `package file '${file.path}' not found`);
        }
        yield* this.#bytes(file.digest, location, offset, length);
    }

    /** Parse JSON bytes, refusing invalid UTF-8. */
    static #json(bytes: Uint8Array<ArrayBuffer>): unknown {
        return JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes));
    }

    /** Lease and store blobs, refusing bytes of another digest. */
    async #verify(blobs: readonly StoredObject[]): Promise<void> {
        for (const { digest, bytes } of blobs) {
            if ((await Digest.of(bytes, this.environment.crypto)) !== digest) {
                throw BuildError.of(
                    "DIGEST_INVALID",
                    `object is invalid: it does not match its digest '${digest}'`,
                );
            }
        }
    }

    /** Store blobs in one pack, leased until the store's lease ends. */
    async #put(blobs: readonly StoredObject[]): Promise<void> {
        await this.blobs.put(
            blobs.map(({ digest, bytes }) => ({
                digest,
                size: bytes.byteLength,
                open: async () => BuildStore.#chunks(bytes),
            })),
            this.#leasedUntil,
        );
    }

    /** Record stored manifests and locks in order, from buffered bytes where given, at once. */
    async #recordObjects(
        objects: readonly Pick<BuildObject, "kind" | "digest">[],
        buffers: ReadonlyMap<Digest, Uint8Array<ArrayBuffer>>,
    ): Promise<void> {
        // open each manifest over its buffered lists, else the stored ones
        const readers = new Map<Digest, BuildReader>();
        const sizes = new Map<Digest, number>();
        for (const { digest } of objects.filter((object) => object.kind === "manifest")) {
            const bytes = await this.#bytesOf(digest, buffers);
            const manifest = PackageManifest.parse(BuildStore.#json(bytes));
            const stream = async (path: string) => this.#streamList(buffers, readers, digest, path);
            readers.set(digest, new BuildReader(manifest, stream, this.environment.crypto));
            sizes.set(digest, bytes.byteLength);
        }

        // refuse a build whose lists the store lacks
        const lists = [...readers].flatMap(([digest, reader]) =>
            reader.references().map((file) => ({ digest, file })),
        );
        const isListed = await this.has(lists.map(({ file }) => file.digest));
        const unlisted = lists.find((_list, index) => isListed[index] !== true);
        if (unlisted !== undefined) {
            throw BuildError.of(
                "MANIFEST_BLOB_INVALID",
                `build '${unlisted.digest}' is invalid: it lists '${unlisted.file.path}', which was not uploaded`,
            );
        }

        // list each build's files and outputs, refusing a list without its format
        const builds = new Map<Digest, RecordedBuild>();
        for (const [digest, reader] of readers) {
            // refuse duplicate paths
            await reader.check();
            const distributed = await reader.distributed();
            requireDistinctPaths(distributed);

            // list the files each output runs
            const outputs = await Promise.all(
                Object.entries(reader.manifest.outputs).map(async ([name, output]) => ({
                    name,
                    files: await reader.runFiles([output.runtime]),
                })),
            );
            const size = found(sizes, digest);
            builds.set(digest, { size, files: distinctFiles(distributed), outputs });
        }

        // find which of every listed file the store has, at once
        const listed = [...builds.values()].flatMap((build) => [
            ...build.files,
            ...build.outputs.flatMap((output) => output.files),
        ]);
        const digests = [...new Set(listed.map((file) => file.digest))];
        const isStored = await this.has(digests);
        const stored = new Set(digests.filter((_digest, index) => isStored[index] === true));
        const isHere = (files: readonly PackageFile[]) =>
            files.every((file) => stored.has(file.digest));

        // read each lock
        const locks: { readonly digest: Digest; readonly size: number; readonly lock: Lock }[] = [];
        for (const { digest } of objects.filter((object) => object.kind === "lock")) {
            const bytes = await this.#bytesOf(digest, buffers);
            locks.push({
                digest,
                size: bytes.byteLength,
                lock: Lock.parse(BuildStore.#json(bytes)),
            });
        }

        // refuse a lock listing a build recorded neither here nor before
        const outside = locks
            .flatMap(({ lock }) => Lock.builds(lock))
            .filter((build) => !builds.has(build));
        const recorded = await this.#recordedBuilds([...new Set(outside)]);
        for (const { digest, lock } of locks) {
            const absent = Lock.builds(lock).find(
                (build) => !builds.has(build) && !recorded.has(build),
            );
            if (absent !== undefined) {
                throw BuildError.of(
                    "MANIFEST_NOT_FOUND",
                    `build '${absent}' of lock '${digest}' not found`,
                );
            }
        }

        // record each build whole, else each output whose files are all here, then each lock
        await this.database.transaction(async (transaction) => {
            for (const [digest, build] of builds) {
                await BuildStore.#recordBuild(
                    transaction,
                    digest,
                    build,
                    isHere,
                    found(readers, digest),
                );
            }
            for (const { digest, size, lock } of locks) {
                await BuildStore.#recordManifest(transaction, digest, size, Lock.builds(lock));
            }
        });
    }

    /** Record a build in a transaction: whole once every file is here, else each output that is. */
    static async #recordBuild(
        transaction: DatabaseConnection,
        digest: Digest,
        build: RecordedBuild,
        isHere: (files: readonly PackageFile[]) => boolean,
        reader: BuildReader,
    ): Promise<void> {
        // record it whole with its files
        const { size } = build;
        if (isHere(build.files)) {
            const referenced = build.files.map((file) => file.digest);
            await BuildStore.#recordManifest(transaction, digest, size, referenced);
        }
        // record its lists, then each output whose files are all here
        else {
            const lists = reader.references().map((file) => file.digest);
            await BuildStore.#reference(transaction, digest, lists);
            await transaction
                .insert(buildManifest)
                .values({ digest, size, isComplete: false })
                .onConflictDoNothing();
            for (const output of build.outputs.filter((each) => isHere(each.files))) {
                const referenced = [...new Set(output.files.map((file) => file.digest))];
                await BuildStore.#reference(transaction, digest, referenced);
                await transaction
                    .insert(buildOutput)
                    .values({ manifest: digest, name: output.name })
                    .onConflictDoNothing();
            }
        }
    }

    /** Find which builds the store records, its layer's among them. */
    async #recordedBuilds(digests: readonly Digest[]): Promise<Set<Digest>> {
        // take the layered builds, then read the others' rows a batch at a time
        const recorded = new Set(digests.filter((digest) => this.layer.head(digest) !== undefined));
        const others = digests.filter((digest) => !recorded.has(digest));
        for (let start = 0; start < others.length; start += BATCH_ROWS) {
            const rows = await this.database
                .select({ digest: buildManifest.digest })
                .from(buildManifest)
                .where(inArray(buildManifest.digest, others.slice(start, start + BATCH_ROWS)));
            for (const row of rows) {
                recorded.add(row.digest);
            }
        }

        return recorded;
    }

    /** Read a stored object's bytes, buffered ones first. */
    async #bytesOf(
        digest: Digest,
        buffers: ReadonlyMap<Digest, Uint8Array<ArrayBuffer>>,
    ): Promise<Uint8Array<ArrayBuffer>> {
        const bytes = buffers.get(digest);

        return (
            bytes ??
            new Uint8Array(
                await new Response(BuildStore.#readable(this.blobs.read(digest))).arrayBuffer(),
            )
        );
    }

    /** Stream a list of a pulled build from the buffers, else from this store. */
    async #streamList(
        buffers: ReadonlyMap<Digest, Uint8Array<ArrayBuffer>>,
        readers: ReadonlyMap<Digest, BuildReader>,
        build: Digest,
        path: string,
    ): Promise<ReadableStream<Uint8Array>> {
        const file = present(
            found(readers, build)
                .references()
                .find((each) => each.path === path),
            `the list ${path} of build ${build}`,
        );
        const bytes = buffers.get(file.digest);

        return bytes === undefined ? this.#stream(file) : new Blob([bytes]).stream();
    }

    /** Store bytes under their digest unless stored, leased for the row recording them. */
    async #write(digest: Digest, bytes: Uint8Array<ArrayBuffer>): Promise<void> {
        await this.blobs.put(
            [{ digest, size: bytes.byteLength, open: async () => BuildStore.#chunks(bytes) }],
            this.#leasedUntil,
        );
    }

    /** Require the store to have each file a build's manifest lists. */
    async #requireStored(digest: string, files: readonly PackageFile[]): Promise<void> {
        // find the files the store lacks, refusing the first
        const stored = await this.has(files.map((file) => file.digest));
        const absent = files.find((_file, index) => stored[index] !== true);
        if (absent !== undefined) {
            throw BuildError.of(
                "MANIFEST_BLOB_INVALID",
                `build '${digest}' is invalid: it lists '${absent.path}', which was not uploaded`,
            );
        }
    }

    /** Record a whole manifest or lock in a transaction: its distinct references, then it. */
    static async #recordManifest(
        transaction: DatabaseConnection,
        digest: Digest,
        size: number,
        referenced: readonly Digest[],
    ): Promise<void> {
        await BuildStore.#reference(transaction, digest, referenced);
        await transaction
            .insert(buildManifest)
            .values({ digest, size, isComplete: true })
            .onConflictDoUpdate({ target: buildManifest.digest, set: { isComplete: true } });
    }

    /** Record the blobs a build references, a batch of rows at a time. */
    static async #reference(
        transaction: DatabaseConnection,
        manifest: Digest,
        referenced: readonly Digest[],
    ): Promise<void> {
        for (let start = 0; start < referenced.length; start += BATCH_ROWS) {
            const batch = referenced.slice(start, start + BATCH_ROWS);
            await transaction
                .insert(buildReference)
                .values(batch.map((digest) => ({ manifest, digest })))
                .onConflictDoNothing();
        }
    }

    /** Replace the blobs a root retains. */
    static async #replaceRoot(
        transaction: DatabaseConnection,
        name: string,
        digests: ReadonlySet<Digest>,
    ): Promise<void> {
        // read the root's blobs
        const rows = await transaction
            .select({ digest: buildRoot.digest })
            .from(buildRoot)
            .where(eq(buildRoot.name, name));
        const current = new Set(rows.map(({ digest }) => digest));

        // add the new ones and drop the old ones
        const added = [...digests].filter((digest) => !current.has(digest));
        const dropped = [...current].filter((digest) => !digests.has(digest));
        for (let start = 0; start < added.length; start += BATCH_ROWS) {
            const batch = added.slice(start, start + BATCH_ROWS);
            await transaction.insert(buildRoot).values(batch.map((digest) => ({ name, digest })));
        }
        for (let start = 0; start < dropped.length; start += BATCH_ROWS) {
            const batch = dropped.slice(start, start + BATCH_ROWS);
            await transaction
                .delete(buildRoot)
                .where(and(eq(buildRoot.name, name), inArray(buildRoot.digest, batch)));
        }
    }

    /** Stream the chunks of an async iterable, reading one per pull. */
    static #readable(chunks: AsyncIterable<Uint8Array>): ReadableStream<Uint8Array> {
        const iterator = chunks[Symbol.asyncIterator]();

        return new ReadableStream({
            async pull(controller) {
                // enqueue the next chunk, or close after the last
                const next = await iterator.next();
                if (next.done === true) {
                    controller.close();
                } else {
                    controller.enqueue(next.value);
                }
            },
            async cancel() {
                await iterator.return?.();
            },
        });
    }

    /** Read some bytes as one chunk. */
    static async *#chunks(bytes: Uint8Array<ArrayBuffer>): AsyncIterable<Uint8Array> {
        yield bytes;
    }

    /** List the blobs a cache entry references: a manifest, or a file. */
    static #digests(entry: CacheEntry): Digest[] {
        switch (entry.kind) {
            // list a whole build's manifest
            case "build":
                return [entry.manifest];
            // list a transform's file
            case "file":
                return [entry.file.digest];
        }
    }
}

/** A build being recorded: its manifest's size, its distinct files, and each output's run files. */
interface RecordedBuild {
    /** The manifest's size in bytes. */
    readonly size: number;
    /** The distinct files it distributes. */
    readonly files: readonly PackageFile[];
    /** The files each output runs, by name. */
    readonly outputs: readonly { readonly name: string; readonly files: readonly PackageFile[] }[];
}

/** Take the first file of each digest. */
function distinctFiles(list: readonly PackageFile[]): PackageFile[] {
    return [...new Map(list.map((file) => [file.digest, file])).values()];
}

/** Refuse a file list with the reserved manifest path or one path twice. */
function requireDistinctPaths(list: readonly PackageFile[]): void {
    const paths = new Set<string>();
    for (const file of list) {
        if (file.path === MANIFEST_PATH || file.path.startsWith(`${MANIFEST_PATH}/`)) {
            throw BuildError.of(
                "MANIFEST_INVALID",
                `build path '${file.path}' is invalid: it is reserved`,
            );
        }
        if (paths.has(file.path)) {
            throw BuildError.of(
                "MANIFEST_INVALID",
                `package file '${file.path}' is invalid: it is a duplicate`,
            );
        }
        paths.add(file.path);
    }
}

/** A build's closure on a runtime: the import map of its modules and the builds it maps. */
export interface ImportClosure {
    /** The import map of the build's modules and its dependencies' on the runtime. */
    readonly map: ImportMap;
    /** The builds the map's URLs point into: the build itself and its dependencies'. */
    readonly builds: ReadonlySet<Digest>;
}

/** List a build's outputs on a runtime. */
function outputsOn(manifest: PackageManifest, runtime: Runtime): PackageOutput[] {
    return Object.values(manifest.outputs).filter((output) => output.runtime === runtime);
}
