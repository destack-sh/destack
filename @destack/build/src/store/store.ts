import { type Environment, type FileSystem, Pipe, waitAborted } from "@destack/environment";
import { Digest, type DigestCrypto, present } from "@destack/schema";
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
import {
    DependencyName,
    DependencyRelease,
    type DependencyResolution,
    type PackageId,
} from "@destack/package";
import { PackageFile } from "@destack/package/file";
import {
    BuildReader,
    BuildWriter,
    type ImportMap,
    type PackageLocation,
    PackageManifest,
    type PackageOutput,
} from "@destack/package/manifest";
import type { Runtime } from "@destack/package/runtime";
import { BuildError } from "../error/index.ts";
import { Tarball, TarballEntry } from "@destack/package/archive";
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
import { BuildLayer } from "./layer.ts";

/** The concurrent uploads of one store. */
const CONCURRENT_UPLOADS = 8;

/** The rows one statement binds: 500, far under SQLite's 32,766 parameters at two columns. */
const BATCH_ROWS = 500;

/** How long a cache entry retains what it references past its last use: a week, as Nx does. */
const CACHE_MILLISECONDS = 7 * 24 * 60 * 60_000;

/** How often a store records a cache entry's use: daily, a seventh of the entry's lifetime. */
const USE_MILLISECONDS = 24 * 60 * 60_000;

/** The prefix of the root a cache entry retains its blobs by, followed by its key. */
const CACHE_ROOT_PREFIX = "cache:";

/** Where a closure starts: at what a build runs, or at every export a dependent may import. */
export type ClosureEntries = "run" | "export";

/** What a closure does with builtins: follow them, or skip them as the runtime links them. */
export type ClosureBuiltins = "follow" | "link";

/** Read a build's manifest at a package's registry, absent where none is stored. */
export type ManifestRead = (
    manifest: Digest,
    packageId: PackageId,
) => Promise<PackageManifest | undefined>;

/** The blobs a store stores builds in, and the environment it runs in. */
export interface BuildStoreOptions<StoreEnvironment, Storage extends BlobStorage> {
    /** The partition's blobs, whose database declares the store's tables beside its own. */
    readonly blobs: BlobStore<Storage>;
    /** The compression streaming archives, the crypto digesting files and the clock of leases. */
    readonly environment: StoreEnvironment;
    /** The read-only builds the store reads in place, such as a machine's shipped builds. */
    readonly layer?: BuildLayer;
    /** How long a stored build lasts unless a root or a use retains it, by default the blobs'. */
    readonly leaseMilliseconds?: number;
}

/** Immutable package builds in a partition's blobs, kept while any of its rows references them. */
export class BuildStore<
    StoreEnvironment extends Pick<Environment, "compression" | "crypto" | "clock"> = Pick<
        Environment,
        "compression" | "crypto" | "clock"
    >,
    Storage extends BlobStorage = BlobStorage,
> {
    /** The database of a store of its own, such as a toolchain's on its machine's disk. */
    static readonly database = defineDatabase({
        name: "builds",
        tables: [...BUILD_STORE_TABLES, ...BLOB_TABLES],
    });

    /** The partition's blobs: the builds' files and manifests. */
    readonly blobs: BlobStore<Storage>;
    /** The compression, crypto and clock the store archives, digests and expires with. */
    readonly environment: StoreEnvironment;
    /** The reader of each build opened so far, by manifest digest, until its build is deleted. */
    readonly #readers = new Map<string, Promise<BuildReader>>();
    /** The builds of cache keys in flight, so callers asking at once build each key once. */
    readonly #building = new Map<string, Promise<Digest>>();
    /** The builds in flight to the store, by manifest digest, each stored once. */
    readonly #putting = new Map<Digest, Promise<Digest>>();
    /** The read-only builds the store reads in place. */
    readonly #layer: BuildLayer;
    /** How long a stored build lasts before a root or a use retains it, in milliseconds. */
    readonly leaseMilliseconds: number;

    /** Store builds in a partition's blobs, reading some read-only builds in place. */
    constructor(options: BuildStoreOptions<StoreEnvironment, Storage>) {
        // take the blobs, the environment, the layer and the lease
        this.blobs = options.blobs;
        this.environment = options.environment;
        this.#layer = options.layer ?? BuildLayer.empty();
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

        // record the build
        await this.#record(digest, bytes.byteLength, source.manifest, files);

        return digest;
    }

    /** Report whether the store has a build, recorded once every blob it references is stored. */
    async contains(digest: Digest): Promise<boolean> {
        // take a layered build as contained
        if (this.#layer.hasBuild(digest)) {
            return true;
        }

        // find a complete stored build
        const [found] = await this.database
            .select({ digest: buildManifest.digest })
            .from(buildManifest)
            .where(and(eq(buildManifest.digest, digest), eq(buildManifest.isComplete, true)))
            .limit(1);

        return found !== undefined;
    }

    /** Report whether the store has an output of a build: the whole build, or that output. */
    async containsOutput(digest: Digest, output: string): Promise<boolean> {
        // take a whole build as containing each output
        if (await this.contains(digest)) {
            return true;
        }

        // find the output a pull stored
        const [found] = await this.database
            .select({ name: buildOutput.name })
            .from(buildOutput)
            .where(and(eq(buildOutput.manifest, digest), eq(buildOutput.name, output)))
            .limit(1);

        return found !== undefined;
    }

    /** Report whether the store has a blob. */
    async has(digest: Digest): Promise<boolean> {
        return (await this.missing([digest])).length === 0;
    }

    /** List the blobs among some digests the store lacks, its layer's counted as stored. */
    async missing(digests: readonly Digest[]): Promise<Digest[]> {
        const unlayered = digests.filter((digest) => !this.#layer.has(digest));
        const missing = new Set(await this.blobs.missing(unlayered));

        return unlayered.filter((digest) => missing.has(digest));
    }

    /** Copy a served build by manifest digest unless stored, in one request for its archive. */
    async copy(
        location: PackageLocation,
        fetch: (input: URL, init: RequestInit) => Promise<Response>,
        signal?: AbortSignal,
    ): Promise<void> {
        // return a build the store has
        if (await this.contains(location.manifest)) {
            return;
        }

        // read the served build whole from its archive, its manifest verified by its digest
        const reader = await BuildReader.archive(location, fetch, this.environment, { signal });

        // store each missing file as served, then the manifest
        const digest = await this.put(reader, signal);
        if (digest !== location.manifest) {
            throw BuildError.of(
                "DIGEST_MISMATCH",
                `expected build digest ${location.manifest}, got ${digest}`,
            );
        }
    }

    /** Pull one output of a served build with its maps, in one request for its archive. */
    async pull(
        location: PackageLocation,
        fetch: (input: URL, init: RequestInit) => Promise<Response>,
        output: string,
        signal?: AbortSignal,
    ): Promise<void> {
        // return an output the store has
        if (await this.containsOutput(location.manifest, output)) {
            return;
        }

        // read the output's archive, its manifest verified by its digest, and list what it carries
        const reader = await BuildReader.archive(location, fetch, this.environment, {
            signal,
            output,
        });
        const files = await reader.outputFiles(output);

        // store the files as served in one pack, then the manifest
        signal?.throwIfAborted();
        await this.blobs.put(
            files.map((file) => ({
                digest: file.digest,
                size: file.size,
                open: () => reader.stream(file.path),
            })),
            this.#leasedUntil,
        );
        const bytes = PackageManifest.serialize(reader.manifest);
        await this.#write(location.manifest, bytes);

        // record the output beside the references it adds
        await this.#recordOutput(
            location.manifest,
            bytes.byteLength,
            reader.manifest,
            files,
            output,
        );
    }

    /** Lease and store uploaded files in one pack, refusing bytes of another digest. */
    async receive(
        uploads: readonly { readonly digest: string; readonly bytes: Uint8Array<ArrayBuffer> }[],
    ): Promise<void> {
        // refuse bytes of another digest
        const files = [];
        for (const { digest, bytes } of uploads) {
            const expected = Digest.parse(digest);
            if ((await Digest.of(bytes, this.environment.crypto)) !== expected) {
                throw BuildError.of(
                    "DIGEST_INVALID",
                    `the uploaded file does not match its digest ${digest}`,
                );
            }
            files.push({
                digest: expected,
                size: bytes.byteLength,
                open: async () => BuildStore.#chunks(bytes),
            });
        }

        // lease the files for their manifest, then store them
        await this.blobs.put(files, this.#leasedUntil);
    }

    /** Store an uploaded manifest with all its files, refusing other digests and missing files. */
    async adopt(digest: Digest, bytes: Uint8Array<ArrayBuffer>): Promise<void> {
        // read and parse the manifest by its digest
        if ((await Digest.of(bytes, this.environment.crypto)) !== digest) {
            throw BuildError.of(
                "DIGEST_INVALID",
                `the uploaded manifest does not match its digest ${digest}`,
            );
        }
        const parsed = PackageManifest.parse(
            JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes)),
        );

        // require the files the manifest lists, then each listed file under a distinct path
        const reader = new BuildReader(
            parsed,
            async (path) => this.#stream(present(references.get(path), `the listed file ${path}`)),
            this.environment.crypto,
        );
        const references = new Map(reader.references().map((file) => [file.path, file]));
        await this.#requireStored(digest, reader.references());
        await this.#requireStored(digest, await reader.files());
        const list = await reader.distributed();
        requireDistinctPaths(list);

        // lease and store the manifest, then record the build
        await this.blobs.put(
            [{ digest, size: bytes.byteLength, open: async () => BuildStore.#chunks(bytes) }],
            this.#leasedUntil,
        );
        await this.#record(digest, bytes.byteLength, parsed, distinctFiles(list));
    }

    /** Record a build whose manifest and files the blobs already have, as a moved space's. */
    async index(digest: Digest): Promise<void> {
        // read the manifest's bytes from the blobs, then record the build as an uploaded one
        const stream = BuildStore.#readable(this.blobs.read(digest));
        const bytes = new Uint8Array(await new Response(stream).arrayBuffer());
        await this.adopt(digest, bytes);
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
        const [found] = await this.database
            .select({
                path: buildReferrer.path,
                digest: buildReferrer.digest,
                size: buildReferrer.size,
                mediaType: buildReferrer.mediaType,
            })
            .from(buildReferrer)
            .where(and(eq(buildReferrer.subject, subject), eq(buildReferrer.path, path)));

        return found;
    }

    /** Read a build's manifest, verified and parsed once with its reader. */
    async get(digest: Digest): Promise<PackageManifest> {
        return (await this.contents(digest)).manifest;
    }

    /** Read a stored build's manifest, absent for a build the store does not record. */
    async find(digest: Digest): Promise<PackageManifest | undefined> {
        return (await this.#find(digest))?.manifest;
    }

    /** Open a stored build for selective reads, one reader per build, as its files never change. */
    async contents(digest: Digest): Promise<BuildReader> {
        const contents = await this.#find(digest);
        if (contents === undefined) {
            throw BuildError.of("MANIFEST_UNKNOWN", `package manifest ${digest} not found`);
        }

        return contents;
    }

    /** Open a stored build for selective reads, absent for a build the store does not record. */
    async #find(digest: Digest): Promise<BuildReader | undefined> {
        // read a layered build in place, the files it leaves out from the store's blobs
        const layered = this.#layer.build(digest, async (path, signal) =>
            (await this.#open(await this.head(digest))).stream(path, signal),
        );
        if (layered !== undefined) {
            return await layered;
        }

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
        // read and verify the manifest, then index the files it lists directly
        const bytes = await this.file(head);
        const read = PackageManifest.parse(
            JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes)),
        );
        const reader = new BuildReader(
            read,
            async (path, signal) => {
                signal?.throwIfAborted();
                const stream = this.#stream(await select(path));

                return signal ? Pipe.through(stream, new TransformStream(), signal) : stream;
            },
            this.environment.crypto,
        );
        const references = new Map(reader.references().map((file) => [file.path, file]));
        let list: Promise<PackageFile[]> | undefined;
        const select = async (path: string) => {
            // look the path up among direct references, then in the list
            let file = references.get(path);
            if (!file) {
                list ??= reader.distributed();
                for (const candidate of await list) {
                    references.set(candidate.path, candidate);
                }
                file = references.get(path);
            }
            if (!file) {
                throw BuildError.of("BLOB_UNKNOWN", `package file not found: ${path}`);
            }

            return file;
        };

        return reader;
    }

    /** Read a cache key's entry, recording its use once a day, absent for a miss. */
    async cached(key: string): Promise<CacheEntry | undefined> {
        // read the entry, a root of what it references while it lives
        const digest = Digest.parse(key);
        const [found] = await this.database
            .select({ entry: buildCache.entry, usedAt: buildCache.usedAt })
            .from(buildCache)
            .where(eq(buildCache.key, digest));

        // record a hit's use once a day
        const now = this.environment.clock.now();
        if (found !== undefined && now - found.usedAt >= USE_MILLISECONDS) {
            await this.database
                .update(buildCache)
                .set({ usedAt: now })
                .where(eq(buildCache.key, digest));
        }

        return found?.entry;
    }

    /** Store an entry under a cache key, once the manifest or files it references are stored. */
    async cache(key: string, entry: CacheEntry): Promise<void> {
        // read the key's digest and the use time
        const digest = Digest.parse(key);
        const usedAt = this.environment.clock.now();
        await this.database.transaction(async (transaction) => {
            // record the entry
            await transaction
                .insert(buildCache)
                .values({ key: digest, entry, usedAt })
                .onConflictDoUpdate({ target: buildCache.key, set: { entry, usedAt } });

            // retain what it references under its root
            await BuildStore.#replaceRoot(
                transaction,
                `${CACHE_ROOT_PREFIX}${digest}`,
                new Set(BuildStore.#digests(entry)),
            );
        });
    }

    /** Take the build a key caches, else build and cache it once for all callers at once. */
    cachedBuild(key: string, build: () => Promise<Digest>): Promise<Digest> {
        // join a build of the key in flight
        const flying = this.#building.get(key);
        if (flying !== undefined) {
            return flying;
        }

        // take the cached build, else build it and cache its manifest
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

    /** Set the builds a root retains, such as a space's, leaving what it drops to a collection. */
    async root(name: string, manifests: ReadonlySet<string>): Promise<void> {
        await this.database.transaction((transaction) =>
            BuildStore.#replaceRoot(transaction, name, manifests),
        );
    }

    /** Forget a root, leaving what only it retained to a collection. */
    async forget(name: string): Promise<void> {
        await this.root(name, new Set());
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

    /** Store a build again with its Destack dependencies pinned to other builds. */
    async relink(
        digest: Digest,
        dependencies: Readonly<Record<string, DependencyResolution>>,
    ): Promise<Digest> {
        // pin each output's Destack dependencies to the given builds, keeping its npm units
        const reader = await this.contents(digest);
        const pin = (name: string, pinned: DependencyRelease): DependencyRelease =>
            pinned.kind === "npm"
                ? pinned
                : DependencyRelease.parse(present(dependencies[name], `the build of ${name}`));
        const outputs = Object.fromEntries(
            Object.entries(reader.manifest.outputs).map(([name, output]) => [
                name,
                {
                    ...output,
                    dependencies: Object.fromEntries(
                        Object.entries(output.dependencies).map(([each, pinned]) => [
                            each,
                            pin(each, pinned),
                        ]),
                    ),
                },
            ]),
        );

        // pin the dependency list's Destack releases alike, by package name
        const listed = Object.fromEntries(
            Object.entries(await reader.dependencies()).map(([key, listing]) => [
                key,
                listing.kind === "destack"
                    ? present(dependencies[listing.package.name], `the build of ${key}`)
                    : listing,
            ]),
        );

        // write the manifest anew, streaming the build's files from the stored one
        const written = new Map<string, Uint8Array<ArrayBuffer>>();
        const manifest = await BuildWriter.relink(
            reader,
            { write: async (path, bytes) => void written.set(path, bytes) },
            this.environment.crypto,
            { outputs, dependencies: listed },
        );

        return await this.#putWritten(reader, written, manifest);
    }

    /** Store a build with its new files in memory, streaming the rest from its source build. */
    async #putWritten(
        source: BuildReader,
        written: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
        manifest: PackageManifest,
    ): Promise<Digest> {
        return await this.put(
            new BuildReader(
                manifest,
                async (path, signal) => {
                    const bytes = written.get(path);

                    return bytes === undefined
                        ? await source.stream(path, signal)
                        : new Blob([bytes]).stream();
                },
                this.environment.crypto,
            ),
        );
    }

    /** List the npm units a build depends on, directly or through other units, as far as found. */
    async units(
        root: Digest,
        packageId: PackageId,
        read: ManifestRead = (digest) => this.get(digest),
    ): Promise<Digest[]> {
        // visit each unit once, from the build's outputs through each found unit's outputs, at the build's registry
        const units = new Set<Digest>();
        const pending = [root];
        for (let next = pending.pop(); next !== undefined; next = pending.pop()) {
            const found = await read(next, packageId);
            const recorded = Object.values(found?.outputs ?? {}).flatMap((output) =>
                Object.values(output.dependencies),
            );
            for (const dependency of recorded) {
                if (dependency.kind === "npm" && !units.has(dependency.manifest)) {
                    units.add(dependency.manifest);
                    pending.push(dependency.manifest);
                }
            }
        }

        return [...units];
    }

    /** Read the manifests a build's entries import on a runtime: what it runs, or every export. */
    async closure(
        start: Digest,
        packageId: PackageId,
        runtime: Runtime,
        read: ManifestRead = (digest) => this.get(digest),
        entries: ClosureEntries = "run",
        builtins: ClosureBuiltins = "follow",
    ): Promise<ReadonlySet<Digest>> {
        // start at what each output runs, its workloads and the exports its views mount, or at every export
        const manifests = new Map<Digest, Promise<PackageManifest | undefined>>();
        const once: ManifestRead = (digest, located) => {
            const reading = manifests.get(digest) ?? read(digest, located);
            manifests.set(digest, reading);

            return reading;
        };
        const first = await once(start, packageId);
        const outputs = Object.values(first?.outputs ?? {}).filter(
            (output) => output.runtime === runtime,
        );
        const pending = outputs.flatMap((output) => {
            const mounted = new Set(Object.values(output.views).map((view) => view.entrypoint));
            const started =
                entries === "export"
                    ? Object.keys(output.exports)
                    : [
                          ...Object.values(output.workloads).map((workload) => workload.entrypoint),
                          ...Object.entries(output.exports)
                              .filter(([, exported]) => mounted.has(exported.path))
                              .map(([subpath]) => subpath),
                      ];

            return started.map((subpath) => ({
                manifest: start,
                packageId,
                runtime: output.runtime,
                subpath,
            }));
        });

        // follow each export once to the dependency exports its chunks import, keeping each npm unit's importer
        const units = new Map<
            Digest,
            {
                readonly located: PackageId;
                readonly beside: Readonly<Record<string, DependencyRelease>>;
            }
        >();
        const closure = new Set<Digest>([start]);
        const visited = new Set<string>();
        for (let next = pending.pop(); next !== undefined; next = pending.pop()) {
            const key = `${next.manifest} ${next.runtime} ${next.subpath}`;
            const found = visited.has(key) ? undefined : await once(next.manifest, next.packageId);
            visited.add(key);
            if (found !== undefined) {
                const output = present(
                    Object.values(found.outputs).find((each) => each.runtime === next.runtime),
                    `the ${next.runtime} output of ${next.manifest}`,
                );
                const { imports } = present(
                    output.exports[next.subpath],
                    `the export ${next.subpath} of ${next.manifest}`,
                );
                for (const specifier of imports) {
                    const dependency = present(
                        output.dependencies[DependencyName.of(specifier)],
                        `the dependency of ${specifier} in ${next.manifest}`,
                    );
                    // skip builtins, which the runtime links
                    const isLinked =
                        builtins === "link" &&
                        dependency.kind !== "npm" &&
                        (await once(dependency.manifest, dependency.package.id))?.role ===
                            "builtin";
                    if (isLinked) {
                        continue;
                    }

                    // collect npm units, follow Destack exports
                    closure.add(dependency.manifest);
                    if (dependency.kind === "npm") {
                        units.set(dependency.manifest, {
                            located: next.packageId,
                            beside: output.dependencies,
                        });
                    } else {
                        pending.push({
                            manifest: dependency.manifest,
                            packageId: dependency.package.id,
                            runtime: next.runtime,
                            subpath: DependencyName.subpath(specifier),
                        });
                    }
                }
            }
        }

        // add the npm units each imported unit depends on, found at its dependent's registry, and the peers its importer provides
        const peered = new Set<Digest>();
        for (const [unit, { located, beside }] of units) {
            const reaching = [unit];
            for (let next = reaching.pop(); next !== undefined; next = reaching.pop()) {
                // add the unit and those it depends on once, then each peer they declare
                for (const needed of [next, ...(await this.units(next, located, once))]) {
                    closure.add(needed);
                    if (!peered.has(needed)) {
                        peered.add(needed);
                        const declared = Object.values(
                            (await once(needed, located))?.outputs ?? {},
                        );
                        for (const peer of declared.flatMap((each) => Object.keys(each.peers))) {
                            const provided = beside[peer];
                            if (provided?.kind === "npm" && !closure.has(provided.manifest)) {
                                reaching.push(provided.manifest);
                            }
                        }
                    }
                }
            }
        }

        return closure;
    }

    /** Map a build's browser modules and its dependencies' below a URL prefix. */
    async browser(
        start: Digest,
        base: string,
        read: (digest: Digest) => Promise<PackageManifest> = async (digest) =>
            present(await this.get(digest), `the manifest of build ${digest}`),
    ): Promise<BrowserClosure> {
        // read the builds its views' modules reach
        const started = await read(start);
        const reached = await this.closure(
            start,
            PackageManifest.package(started).id,
            "browser",
            read,
        );

        // map each reached dependency's exports in its dependent's scope, visiting each build once
        const map: ImportMap = { imports: {}, scopes: {} };
        const builds = new Set<Digest>([start]);
        const pending = [{ digest: start, scope: map.imports }];
        for (const { digest, scope } of pending) {
            for (const output of browserOutputs(await read(digest))) {
                const dependencies = Object.entries(output.dependencies).filter(([, release]) =>
                    reached.has(release.manifest),
                );
                for (const [name, release] of dependencies) {
                    // map the dependency's exports to its files below its build's prefix
                    const prefix = `${base}${release.manifest}/`;
                    for (const each of browserOutputs(await read(release.manifest))) {
                        for (const [subpath, exported] of Object.entries(each.exports)) {
                            const specifier = subpath === "." ? name : `${name}${subpath.slice(1)}`;
                            scope[specifier] = `${prefix}${exported.path}`;
                        }
                    }

                    // map the dependency's own imports in its scope
                    if (!builds.has(release.manifest)) {
                        builds.add(release.manifest);
                        map.scopes[prefix] = {};
                        pending.push({ digest: release.manifest, scope: map.scopes[prefix] });
                    }
                }
            }
        }

        return { map, builds };
    }

    /** Find a layered build's directory, which runtimes run in place, absent for another build. */
    layered(digest: Digest): string | undefined {
        return this.#layer.directory(digest);
    }

    /** List the real directories the store's layer keeps files in, which its readers read. */
    layerDirectories(): Promise<readonly string[]> {
        return this.#layer.directories();
    }

    /** Find the dependency tree the layer ships for a tree root's name. */
    layeredTree(name: string): string | undefined {
        return this.#layer.tree(name);
    }

    /** Clone a layered file to a path from its layer, refusing one it lacks. */
    protected async placeLayered(
        file: PackageFile,
        destination: string,
        files: FileSystem,
    ): Promise<void> {
        const layered = present(this.#layer.file(file.digest), `the layered file ${file.path}`);
        await files.copyFile(layered, destination);
    }

    /** Read one complete file and verify its exact bytes. */
    async file(file: PackageFile): Promise<Uint8Array<ArrayBuffer>> {
        // read and verify the file bytes
        const bytes = new Uint8Array(await new Response(this.#stream(file)).arrayBuffer());
        await PackageFile.verify(file, bytes, this.environment.crypto);

        return bytes;
    }

    /** Describe a stored build's manifest file, leaving its bytes unread. */
    async head(digest: Digest): Promise<PackageFile> {
        const head = await this.#head(digest);
        if (head === undefined) {
            throw BuildError.of("MANIFEST_UNKNOWN", `package manifest ${digest} not found`);
        }

        return head;
    }

    /** Describe a build's manifest file, absent for a build the store does not record. */
    async #head(digest: Digest): Promise<PackageFile | undefined> {
        // describe a layered build's manifest
        const layered = this.#layer.head(digest);
        if (layered !== undefined) {
            return layered;
        }

        // read the manifest's size by its digest
        const [stored] = await this.database
            .select({ size: buildManifest.size })
            .from(buildManifest)
            .where(eq(buildManifest.digest, digest));

        return stored === undefined
            ? undefined
            : { path: "manifest.json", digest, size: stored.size, mediaType: "application/json" };
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

    /** Stream a stored build, or one output of it, as a tarball of its manifest and files. */
    export(digest: Digest, output?: string): ReadableStream<Uint8Array<ArrayBuffer>> {
        return Tarball.stream(this.#entries(digest, output), this.environment.compression);
    }

    /** Read a stored build's manifest, then its files or one output's, in path order. */
    async *#entries(digest: Digest, output: string | undefined) {
        // yield the manifest first, as the bytes its digest keys
        const contents = await this.contents(digest);
        yield TarballEntry.of("manifest.json", await this.file(await this.head(digest)));

        // read every stored file's placement at once, then stream each file to its verifying receiver
        const list =
            output === undefined
                ? await contents.distributed()
                : await contents.outputFiles(output);
        const placed = new Map(
            (await this.blobs.placements(list.map((file) => file.digest))).map((placement) => [
                placement.digest,
                placement,
            ]),
        );
        for (const file of list.toSorted(comparePath)) {
            // stream a stored file from where it lies, else through the layer
            const placement = placed.get(file.digest);
            const body =
                placement === undefined
                    ? this.#stream(file)
                    : BuildStore.#readable(
                          this.blobs.storage.slice(
                              placement.pack,
                              placement.offset,
                              placement.size,
                          ),
                      );
            yield { path: file.path, size: file.size, body };
        }
    }

    /** Stream a layered or stored blob, refusing a missing one by its file's path. */
    #stream(file: PackageFile): ReadableStream<Uint8Array> {
        const { blobs } = this;
        const layer = this.#layer;

        return BuildStore.#readable(
            (async function* () {
                // stream a layered blob in place
                const layered = layer.stream(file.digest);
                if (layered !== undefined) {
                    yield* layered;

                    return;
                }

                // stream its bytes from where they lie, refusing a missing blob
                const [placement] = await blobs.placements([file.digest]);
                if (placement === undefined) {
                    throw BuildError.of("BLOB_UNKNOWN", `package file not found: ${file.path}`);
                }
                yield* blobs.storage.slice(placement.pack, placement.offset, placement.size);
            })(),
        );
    }

    /** Read a range of a layered or stored blob. */
    async *#slice(file: PackageFile, offset: number, length: number): AsyncIterable<Uint8Array> {
        // cut a layered blob's range from its bytes
        const layered = this.#layer.stream(file.digest);
        if (layered !== undefined) {
            const bytes = new Uint8Array(await new Response(layered).arrayBuffer());
            yield bytes.subarray(offset, offset + length);

            return;
        }

        // read a stored blob's range
        yield* this.blobs.slice(file.digest, offset, length);
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
        // find the files neither layered nor stored, refusing the first
        const layer = this.#layer;
        const unlayered = files.filter((file) => !layer.has(file.digest));
        const missing = new Set(await this.blobs.missing(unlayered.map((file) => file.digest)));
        const absent = files.find((file) => missing.has(file.digest));
        if (absent !== undefined) {
            throw BuildError.of(
                "MANIFEST_BLOB_UNKNOWN",
                `build ${digest} lists ${absent.path}, which was not uploaded`,
            );
        }
    }

    /** Record a stored build and the blobs its manifest references, all in one transaction. */
    async #record(
        digest: Digest,
        size: number,
        recorded: PackageManifest,
        files: readonly PackageFile[],
    ): Promise<void> {
        // list the build's files and the builds it depends on
        const dependencies = Object.values(recorded.outputs).flatMap((output) =>
            Object.values(output.dependencies).map((dependency) => dependency.manifest),
        );
        const referenced = [...new Set([...files.map((file) => file.digest), ...dependencies])];

        // record the references, then the build
        await this.database.transaction(async (transaction) => {
            await BuildStore.#reference(transaction, digest, referenced);
            await transaction
                .insert(buildManifest)
                .values({ digest, size, isComplete: true })
                .onConflictDoUpdate({ target: buildManifest.digest, set: { isComplete: true } });
        });
    }

    /** Record a pulled output with the references to its files and dependency builds. */
    async #recordOutput(
        digest: Digest,
        size: number,
        recorded: PackageManifest,
        files: readonly PackageFile[],
        output: string,
    ): Promise<void> {
        // list the output's files and the builds it depends on
        const dependencies = Object.values(
            present(recorded.outputs[output], `the output ${output}`).dependencies,
        ).map((dependency) => dependency.manifest);
        const referenced = [...new Set([...files.map((file) => file.digest), ...dependencies])];

        // record the references, then the output
        await this.database.transaction(async (transaction) => {
            await BuildStore.#reference(transaction, digest, referenced);
            await transaction
                .insert(buildManifest)
                .values({ digest, size, isComplete: false })
                .onConflictDoNothing();
            await transaction
                .insert(buildOutput)
                .values({ manifest: digest, name: output })
                .onConflictDoNothing();
        });
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
        digests: ReadonlySet<string>,
    ): Promise<void> {
        // read the root's blobs, then add the new ones and drop the old ones
        const rows = await transaction
            .select({ digest: buildRoot.digest })
            .from(buildRoot)
            .where(eq(buildRoot.name, name));
        const current = new Set(rows.map(({ digest }) => digest));
        const wanted = new Set([...digests].map((digest) => Digest.parse(digest)));
        const added = [...wanted].filter((digest) => !current.has(digest));
        const dropped = [...current].filter((digest) => !wanted.has(digest));
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

/** Take the first file of each digest. */
export function distinctFiles(list: readonly PackageFile[]): PackageFile[] {
    return [...new Map(list.map((file) => [file.digest, file])).values()];
}

/** Run a task per file, a bounded number at a time, checking the signal before each batch. */
export async function inBatches(
    files: readonly PackageFile[],
    signal: AbortSignal | undefined,
    task: (file: PackageFile) => Promise<void>,
): Promise<void> {
    for (let start = 0; start < files.length; start += CONCURRENT_UPLOADS) {
        signal?.throwIfAborted();
        await Promise.all(files.slice(start, start + CONCURRENT_UPLOADS).map(task));
    }
}

/** Refuse a file list with the reserved manifest path or one path twice. */
function requireDistinctPaths(list: readonly PackageFile[]): void {
    const paths = new Set<string>();
    for (const file of list) {
        if (file.path === "manifest.json" || file.path.startsWith("manifest.json/")) {
            throw BuildError.of("MANIFEST_INVALID", `reserved build path: ${file.path}`);
        }
        if (paths.has(file.path)) {
            throw BuildError.of("MANIFEST_INVALID", `duplicate package file: ${file.path}`);
        }
        paths.add(file.path);
    }
}

/** A build's browser closure: the import map of its modules and the builds the map reaches. */
export interface BrowserClosure {
    /** The import map of the build's browser modules and its dependencies'. */
    readonly map: ImportMap;
    /** The builds the map's URLs reach: the browser runtime build and its dependencies'. */
    readonly builds: ReadonlySet<Digest>;
}

/** List a build's browser outputs. */
function browserOutputs(manifest: PackageManifest): PackageOutput[] {
    return Object.values(manifest.outputs).filter((output) => output.runtime === "browser");
}
