import type { Environment } from "@destack/environment";
import { Digest, type DigestCrypto, present, schema } from "@destack/schema";
import type { BlobStorage } from "@destack/resource";
import {
    and,
    asc,
    type BlobStore,
    type DatabaseConnection,
    eq,
    inArray,
    lte,
    sql,
} from "@destack/db";
import { DependencyName, type PackageId } from "@destack/package";
import { PackageFile } from "@destack/package/file";
import {
    type BuildContents,
    BuildReader,
    BuildWriter,
    type PackageLocation,
    PackageManifest,
} from "@destack/package/manifest";
import type { Runtime } from "@destack/package/runtime";
import { BuildError } from "../error/index.ts";
import { Tarball } from "@destack/package/archive";
import { comparePath } from "../build/serialization.ts";
import type { CacheEntry } from "../cache/entry.ts";
import { type PackageAccess, serveStore } from "./http.ts";
import { buildCache, buildManifest, buildReference, buildReferrer, buildRoot } from "./table.ts";

/** A row of one blob, as a raw statement answers it. */
const DigestRow = schema.object({ digest: Digest });

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

/** The prefix of a build's runtime builds among its referrers, one per server runtime. */
export const RUNTIME_REFERRER = "runtime";

/** The referrer path of a runtime build's source maps. */
export const MAPS_REFERRER = "maps";

/** The type declaration files a runtime never loads, beside the modules they describe. */
const TYPE_DECLARATION = /\.d\.[cm]?ts$/u;

/** Read a build's manifest at a package's registry, absent where none is stored. */
export type ManifestRead = (
    manifest: Digest,
    packageId: PackageId,
) => Promise<PackageManifest | undefined>;

/** What a collection unrecorded, and when the next lease or cache entry ends. */
export interface Collection {
    /** The builds deleted. */
    readonly manifests: readonly Digest[];
    /** When the next collection can delete more, in UTC epoch milliseconds, absent for never. */
    readonly next: number | undefined;
}

/** The blobs a store keeps builds in, and the environment it runs in. */
export interface PackageStoreOptions<StoreEnvironment, Storage extends BlobStorage> {
    /** The partition's blobs, whose database declares the store's tables beside its own. */
    readonly blobs: BlobStore<Storage>;
    /** The compression streaming archives, the crypto digesting files and the clock of leases. */
    readonly environment: StoreEnvironment;
}

/** Immutable package builds in a partition's blobs, kept while any of its rows references them. */
export class PackageStore<
    StoreEnvironment extends Pick<Environment, "compression" | "crypto" | "clock"> = Pick<
        Environment,
        "compression" | "crypto" | "clock"
    >,
    Storage extends BlobStorage = BlobStorage,
> {
    /** The partition's blobs, keeping the builds' files and manifests. */
    readonly blobs: BlobStore<Storage>;
    /** The compression, crypto and clock the store archives, digests and expires with. */
    readonly environment: StoreEnvironment;
    /** The reader of each build opened so far, by manifest digest, until its build is deleted. */
    readonly #readers = new Map<string, Promise<BuildReader>>();
    /** The builds of cache keys in flight, so callers asking at once build each key once. */
    readonly #building = new Map<string, Promise<Digest>>();
    /** The runtime builds being derived, by build and runtime. */
    readonly #attaching = new Map<string, Promise<Digest>>();

    /** Store builds in a partition's blobs. */
    constructor(options: PackageStoreOptions<StoreEnvironment, Storage>) {
        this.blobs = options.blobs;
        this.environment = options.environment;
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
        const bytes = encodeManifest(PackageManifest.parse(manifest));

        return Digest.of(bytes, crypto);
    }

    /** Store each missing file of a build, then its manifest, returning the manifest's digest. */
    async put(source: BuildReader, signal?: AbortSignal): Promise<Digest> {
        // take a stored build by its manifest
        const bytes = encodeManifest(PackageManifest.parse(source.manifest));
        const digest = await Digest.of(bytes, this.environment.crypto);
        if (await this.contains(digest)) {
            return digest;
        }

        // lease the build's blobs, refusing reserved and duplicate file paths
        const list = await source.distributed();
        requireDistinctPaths(list);
        const files = distinctFiles(list);
        await this.blobs.lease([digest, ...files.map((file) => file.digest)]);

        // store each missing file, a bounded number of streams at a time, then the manifest
        const { storage } = this.blobs;
        const missing = new Set(await storage.missing(files.map((file) => file.digest)));
        await inBatches(
            files.filter((file) => missing.has(file.digest)),
            signal,
            async (file) => {
                await storage.write(await source.stream(file.path, signal), file.digest);
            },
        );
        signal?.throwIfAborted();
        await this.#write(digest, bytes);

        // record the build
        await this.#record(digest, bytes.byteLength, source.manifest, files);

        return digest;
    }

    /** Report whether the store has a build, recorded once every blob it references is stored. */
    async contains(digest: Digest): Promise<boolean> {
        const [found] = await this.database
            .select({ digest: buildManifest.digest })
            .from(buildManifest)
            .where(eq(buildManifest.digest, digest))
            .limit(1);

        return found !== undefined;
    }

    /** Report whether the store has a blob. */
    async has(digest: Digest): Promise<boolean> {
        return (await this.blobs.storage.missing([digest])).length === 0;
    }

    /** Copy a served build by manifest digest unless stored, fetching only missing files. */
    async copy(
        location: PackageLocation,
        fetch: (input: URL, init: RequestInit) => Promise<Response>,
        signal?: AbortSignal,
    ): Promise<void> {
        // return a build the store has
        if (await this.contains(location.manifest)) {
            return;
        }

        // read the served manifest, verified by its digest
        const reader = await BuildReader.open(location, fetch, this.environment.crypto, signal);

        // store each missing file as served, then the manifest
        const digest = await this.put(reader, signal);
        if (digest !== location.manifest) {
            throw BuildError.of(
                "DIGEST_MISMATCH",
                `expected build digest ${location.manifest}, got ${digest}`,
            );
        }
    }

    /** Lease and store an uploaded file under its digest, refusing bytes of another digest. */
    async receive(digest: string, bytes: Uint8Array<ArrayBuffer>): Promise<void> {
        // refuse bytes of another digest
        const expected = Digest.parse(digest);
        if ((await Digest.of(bytes, this.environment.crypto)) !== expected) {
            throw BuildError.of(
                "DIGEST_INVALID",
                `the uploaded file does not match its digest ${digest}`,
            );
        }

        // lease the file for its manifest, then store it
        await this.blobs.put(expected, async () => PackageStore.#chunks(bytes));
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
        await this.blobs.put(digest, async () => PackageStore.#chunks(bytes));
        await this.#record(digest, bytes.byteLength, parsed, distinctFiles(list));
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

    /** List a manifest's referrer builds and theirs in turn, each after its subject. */
    async attached(
        subject: Digest,
    ): Promise<{ readonly subject: Digest; readonly file: PackageFile }[]> {
        // visit each subject's referrers, then theirs in turn, each build once, as a unit may be its own runtime build
        const attached: { readonly subject: Digest; readonly file: PackageFile }[] = [];
        const visited = new Set<Digest>([subject]);
        const pending = [subject];
        for (let next = pending.shift(); next !== undefined; next = pending.shift()) {
            const builds = (await this.referrers(next)).filter(
                (file) =>
                    file.path.startsWith(`${RUNTIME_REFERRER}/`) || file.path === MAPS_REFERRER,
            );
            for (const file of builds) {
                attached.push({ subject: next, file });
                if (!visited.has(file.digest)) {
                    visited.add(file.digest);
                    pending.push(file.digest);
                }
            }
        }

        return attached;
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

    /** Read and verify a build's manifest. */
    async get(digest: Digest): Promise<PackageManifest> {
        const bytes = await this.file(await this.head(digest));

        return PackageManifest.parse(
            JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes)),
        );
    }

    /** Open a stored build for selective reads, one reader per build, as its files never change. */
    contents(digest: Digest): Promise<BuildReader> {
        // reuse the build's reader, forgetting one whose opening failed
        const known = this.#readers.get(digest);
        if (known !== undefined) {
            return known;
        }
        const opening = this.#open(digest);
        this.#readers.set(digest, opening);
        opening.catch(() => this.#readers.delete(digest));

        return opening;
    }

    /** Open a stored build's reader over its manifest's listed files. */
    async #open(digest: Digest): Promise<BuildReader> {
        // index the files the manifest lists directly
        const read = await this.get(digest);
        const reader = new BuildReader(
            read,
            async (path, signal) => {
                signal?.throwIfAborted();
                const stream = this.#stream(await select(path));

                return signal ? stream.pipeThrough(new TransformStream(), { signal }) : stream;
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
            await PackageStore.#replaceRoot(
                transaction,
                `${CACHE_ROOT_PREFIX}${digest}`,
                new Set(PackageStore.#digests(entry)),
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
    async lease(digest: Digest, until: Date): Promise<void> {
        await this.blobs.lease([digest], until.getTime());
    }

    /** Set the builds a root retains, such as a space's, leaving what it drops to a collection. */
    async root(name: string, manifests: ReadonlySet<string>): Promise<void> {
        await this.database.transaction((transaction) =>
            PackageStore.#replaceRoot(transaction, name, manifests),
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

    /** Read the stored builds the roots, leases and cache entries retain, directly or in turn. */
    async retained(): Promise<ReadonlySet<string>> {
        // read what the references retain, then keep the stored builds among it
        const retained = new Set(await this.#retained(this.database, this.environment.clock.now()));
        const stored = await this.database
            .select({ digest: buildManifest.digest })
            .from(buildManifest);

        return new Set(stored.map(({ digest }) => digest).filter((digest) => retained.has(digest)));
    }

    /** Unrecord the builds nothing retains, retiring their blobs, answering what went. */
    async collect(): Promise<Collection> {
        // drop unused cache entries and unrecord unretained builds, retiring what they referenced
        const now = this.environment.clock.now();
        const manifests = await this.database.transaction(async (transaction) => {
            // drop the cache entries unused for a lifetime with their roots
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

            // unrecord the builds nothing retains
            const retained = new Set(await this.#retained(transaction, now));
            const stored = await transaction
                .select({ digest: buildManifest.digest })
                .from(buildManifest);
            const dropped = stored
                .map(({ digest }) => digest)
                .filter((digest) => !retained.has(digest));
            await PackageStore.#unrecord(transaction, dropped);

            return dropped;
        });

        // forget the deleted builds' readers
        for (const digest of manifests) {
            this.#readers.delete(digest);
        }

        // look again once the next lease or cache entry ends
        const [oldest] = await this.database
            .select({ usedAt: buildCache.usedAt })
            .from(buildCache)
            .orderBy(asc(buildCache.usedAt))
            .limit(1);
        const ends = [
            await this.blobs.nextLeaseEnd(now),
            oldest === undefined ? undefined : oldest.usedAt + CACHE_MILLISECONDS,
        ].filter((end) => end !== undefined);

        return { manifests, next: ends.length === 0 ? undefined : Math.min(...ends) };
    }

    /** Delete every build's index, as the partition leaves its host, answering the builds. */
    async clear(): Promise<readonly Digest[]> {
        // read the builds, then delete their index
        const stored = await this.database
            .select({ digest: buildManifest.digest })
            .from(buildManifest);
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

        return stored.map(({ digest }) => digest);
    }

    /** Attach the runtime build of each emitted server output of a stored build. */
    async attachRuntimes(digest: Digest): Promise<void> {
        const { outputs } = await this.get(digest);
        await Promise.all(
            Object.values(outputs)
                .filter((output) => output.emit && output.runtime !== "browser")
                .map((output) => this.attachRuntime(digest, output.runtime)),
        );
    }

    /** Read the runtime build of a stored build, deriving and attaching it once with its maps. */
    attachRuntime(digest: Digest, runtime: Runtime): Promise<Digest> {
        // share a derivation in flight, retrying one that failed
        const key = `${digest} ${runtime}`;
        const known = this.#attaching.get(key);
        if (known !== undefined) {
            return known;
        }
        const attaching = this.#deriveRuntime(digest, runtime);
        this.#attaching.set(key, attaching);
        attaching.catch(() => this.#attaching.delete(key));

        return attaching;
    }

    /** Derive a build's runtime build: its output's loaded files, its maps attached to it apart. */
    async #deriveRuntime(digest: Digest, runtime: Runtime): Promise<Digest> {
        // answer the runtime build attached before
        const path = `${RUNTIME_REFERRER}/${runtime}`;
        const attached = await this.referrer(digest, path);
        if (attached !== undefined) {
            return attached.digest;
        }

        // read the build's output for the runtime, refusing a build without one
        const reader = await this.contents(digest);
        const read = reader.manifest;
        const [name, output] = present(
            Object.entries(read.outputs).find(([, each]) => each.runtime === runtime),
            `the ${runtime} output of ${read.package.name}`,
        );

        // point each dependency at its runtime build, attaching it first
        const dependencies = Object.fromEntries(
            await Promise.all(
                Object.entries(output.dependencies).map(
                    async ([dependency, release]) =>
                        [
                            dependency,
                            {
                                ...release,
                                manifest: await this.attachRuntime(release.manifest, runtime),
                            },
                        ] as const,
                ),
            ),
        );

        // write the output's files a runtime loads with the package.json, native addons and assets included
        const files = await reader.distributed();
        const inside = files.filter((file) => file.path.startsWith(`${output.directory}/`));
        const maps = inside.filter((file) => file.path.endsWith(".map"));
        const loaded = inside.filter(
            (file) => !file.path.endsWith(".map") && !TYPE_DECLARATION.test(file.path),
        );
        const declared = files.filter((file) => file.path === "package.json");
        const { commit } = read;
        const built = await this.#build(reader, [...declared, ...loaded], {
            package: read.package,
            ...(commit === undefined ? {} : { commit }),
            outputs: { [name]: { ...output, dependencies } },
        });

        // attach the maps to the runtime build when the output has any, then the runtime build to the build
        if (maps.length > 0) {
            const mapped = await this.#build(reader, maps, {
                package: read.package,
                ...(commit === undefined ? {} : { commit }),
            });
            await this.attachBuild(built, mapped, MAPS_REFERRER);
        }
        await this.attachBuild(digest, built, path);

        return built;
    }

    /** Store a build of some files of another build, answering its manifest's digest. */
    async #build(
        source: BuildReader,
        files: readonly PackageFile[],
        contents: BuildContents,
    ): Promise<Digest> {
        // write the files in memory, then the manifest listing them
        const written = new Map<string, Uint8Array<ArrayBuffer>>();
        const writer = new BuildWriter(
            { write: async (path, bytes) => void written.set(path, bytes) },
            this.environment.crypto,
        );
        for (const file of files) {
            await writer.write(file.path, await source.load(file.path));
        }
        const built = await writer.finish(contents);

        return this.put(
            new BuildReader(
                built,
                async (path) => new Blob([present(written.get(path), path)]).stream(),
                this.environment.crypto,
            ),
        );
    }

    /** Attach a stored build to another at a path, its manifest as the referrer's blob. */
    async attachBuild(subject: Digest, attached: Digest, path: string): Promise<void> {
        await this.attach(subject, { ...(await this.head(attached)), path });
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

    /** Read the manifests a build runs with, as far as found: itself and its workloads' imports. */
    async closure(
        start: Digest,
        packageId: PackageId,
        read: ManifestRead = (digest) => this.get(digest),
    ): Promise<ReadonlySet<Digest>> {
        // start at each workload's entry of each output, reading each manifest once
        const manifests = new Map<Digest, Promise<PackageManifest | undefined>>();
        const once: ManifestRead = (digest, located) => {
            const reading = manifests.get(digest) ?? read(digest, located);
            manifests.set(digest, reading);

            return reading;
        };
        const first = await once(start, packageId);
        const pending = Object.values(first?.outputs ?? {}).flatMap((output) =>
            Object.values(output.workloads).map((workload) => ({
                manifest: start,
                packageId,
                runtime: output.runtime,
                subpath: workload.entrypoint,
            })),
        );

        // follow each export once to the dependency exports its chunks import
        const units = new Map<Digest, PackageId>();
        const closure = new Set<Digest>([start]);
        const visited = new Set<string>();
        for (let next = pending.pop(); next !== undefined; next = pending.pop()) {
            const key = `${next.manifest} ${next.runtime} ${next.subpath}`;
            const found = visited.has(key) ? undefined : await once(next.manifest, next.packageId);
            visited.add(key);
            if (found !== undefined) {
                const { outputs } = found;
                const output = present(
                    Object.values(outputs).find((each) => each.runtime === next.runtime),
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
                    closure.add(dependency.manifest);
                    if (dependency.kind === "npm") {
                        units.set(dependency.manifest, next.packageId);
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

        // add the npm units each imported unit depends on, found at its dependent's registry
        for (const [unit, located] of units) {
            for (const needed of await this.units(unit, located, once)) {
                closure.add(needed);
            }
        }

        return closure;
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
        // read the manifest's size by its digest
        const [stored] = await this.database
            .select({ size: buildManifest.size })
            .from(buildManifest)
            .where(eq(buildManifest.digest, digest));
        if (!stored) {
            throw BuildError.of("MANIFEST_UNKNOWN", "package manifest not found");
        }

        return { path: "manifest.json", digest, size: stored.size, mediaType: "application/json" };
    }

    /** Stream a stored file or manifest, or a range of it. */
    open(
        file: PackageFile,
        range?: { readonly offset: number; readonly length: number },
    ): ReadableStream<Uint8Array> {
        return range === undefined
            ? this.#stream(file)
            : PackageStore.#readable(
                  this.blobs.storage.slice(file.digest, range.offset, range.length),
              );
    }

    /** Stream a stored build as a tarball of its manifest and files in path order, verified. */
    export(digest: Digest): ReadableStream<Uint8Array<ArrayBuffer>> {
        return Tarball.stream(this.#entries(digest), this.environment.compression);
    }

    /** Read a stored build's manifest, then each file in path order. */
    async *#entries(digest: Digest) {
        // yield the manifest first
        const contents = await this.contents(digest);
        yield {
            path: "manifest.json",
            contents: encodeManifest(contents.manifest),
        };

        // yield each file after verifying its bytes
        const list = await contents.distributed();
        for (const file of list.toSorted(comparePath)) {
            yield { path: file.path, contents: await this.file(file) };
        }
    }

    /** Stream a stored blob, refusing a missing one by its file's path. */
    #stream(file: PackageFile): ReadableStream<Uint8Array> {
        const { storage } = this.blobs;

        return PackageStore.#readable(
            (async function* () {
                // refuse a missing blob, then stream its bytes
                if ((await storage.missing([file.digest])).length > 0) {
                    throw BuildError.of("BLOB_UNKNOWN", `package file not found: ${file.path}`);
                }
                yield* storage.read(file.digest);
            })(),
        );
    }

    /** Store bytes under their digest unless stored. */
    async #write(digest: Digest, bytes: Uint8Array<ArrayBuffer>): Promise<void> {
        if (!(await this.has(digest))) {
            await this.blobs.storage.write(PackageStore.#chunks(bytes), digest);
        }
    }

    /** Require the store to have each file a build's manifest lists. */
    async #requireStored(digest: string, files: readonly PackageFile[]): Promise<void> {
        const missing = new Set(await this.blobs.storage.missing(files.map((file) => file.digest)));
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
            for (let start = 0; start < referenced.length; start += BATCH_ROWS) {
                const batch = referenced.slice(start, start + BATCH_ROWS);
                await transaction
                    .insert(buildReference)
                    .values(batch.map((target) => ({ manifest: digest, digest: target })))
                    .onConflictDoNothing();
            }
            await transaction.insert(buildManifest).values({ digest, size }).onConflictDoNothing();
        });
    }

    /** Read the blobs other rows or leases reference at a moment, and what those reference. */
    async #retained(database: DatabaseConnection, now: number): Promise<Digest[]> {
        const seeds = this.blobs.referenced(now, {
            except: [buildManifest, buildReference, buildReferrer],
        });
        const rows = await database.all(sql`WITH RECURSIVE retained(digest) AS (
                SELECT seed.digest FROM (${seeds}) AS seed
                UNION SELECT edge.digest FROM (
                    SELECT ${buildReference.manifest} AS source, ${buildReference.digest} AS digest FROM ${buildReference}
                    UNION ALL SELECT ${buildReferrer.subject}, ${buildReferrer.digest} FROM ${buildReferrer}
                ) AS edge JOIN retained ON edge.source = retained.digest
            )
            SELECT digest FROM retained`);

        return rows.map((row) => DigestRow.parse(row).digest);
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

    /** Delete the rows of some builds: their references, their referrers and their records. */
    static async #unrecord(
        transaction: DatabaseConnection,
        manifests: readonly Digest[],
    ): Promise<void> {
        for (let start = 0; start < manifests.length; start += BATCH_ROWS) {
            const batch = manifests.slice(start, start + BATCH_ROWS);
            await transaction.delete(buildReference).where(inArray(buildReference.manifest, batch));
            await transaction.delete(buildReferrer).where(inArray(buildReferrer.subject, batch));
            await transaction.delete(buildManifest).where(inArray(buildManifest.digest, batch));
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

    /** List the blobs a cache entry references: a manifest, or files. */
    static #digests(entry: CacheEntry): Digest[] {
        switch (entry.kind) {
            // list a whole build's manifest
            case "build":
                return [entry.manifest];
            // list an output's files
            case "output":
                return entry.output.files.map((file) => file.digest);
            // list a transform's file
            case "file":
                return [entry.file.digest];
        }
    }
}

/** Encode a parsed manifest as its stored bytes, whose digest identifies the build. */
export function encodeManifest(parsed: PackageManifest): Uint8Array<ArrayBuffer> {
    return new TextEncoder().encode(JSON.stringify(parsed));
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
