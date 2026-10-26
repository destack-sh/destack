import type { Environment } from "@destack/environment";
import { Digest, type DigestCrypto, Instant, nth, present, schema } from "@destack/schema";
import {
    type Bucket,
    type BucketBody,
    type BucketEntry,
    BucketFile,
    BucketError,
    type BucketRange,
    MAX_BATCH_FILES,
} from "@destack/bucket";
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
import { CacheEntry } from "../cache/entry.ts";
import { type PackageAccess, serveStore } from "./http.ts";

/** The concurrent uploads of one store. */
const CONCURRENT_UPLOADS = 8;

/** The prefix of the keys storing files, each followed by its digest. */
const FILES_PREFIX = "files/";

/** The prefix of the keys storing manifests, each followed by its digest. */
const MANIFESTS_PREFIX = "manifests/";

/** The prefix of the keys storing referrers, each followed by its subject's digest and its path. */
const REFERRERS_PREFIX = "referrers/";

/** The key of a sweep's marks: each dropped object's key to the instant a sweep marked it. */
const MARKS_KEY = "marks";

/** The key of the instant the last sweep began on its machine's clock, close to others by NTP. */
const SWEPT_KEY = "swept";

/** The marks of the objects a sweep drops, by key. */
const Marks = schema.record(schema.string(), Instant);

/** The prefix of a build's runtime builds among its referrers, one per server runtime. */
export const RUNTIME_REFERRER = "runtime";

/** The referrer path of a runtime build's source maps. */
export const MAPS_REFERRER = "maps";

/** The type declaration files a runtime never loads, beside the modules they describe. */
const TYPE_DECLARATION = /\.d\.[cm]?ts$/u;

/** The prefix of the keys of the roots, each retaining manifests under its name. */
const ROOTS_PREFIX = "roots/";

/** A root's record: when it was written and the manifests it retains. */
const Root = schema.object({
    /** When the root was written, in UTC epoch milliseconds on the writer's clock. */
    rootedAt: Instant,
    /** The manifests the root retains. */
    manifests: schema.array(Digest),
});

/** Decide which of some stored digests a sweep retains, read in batches the bucket lists. */
export type Retention = (digests: readonly string[]) => Promise<ReadonlySet<string>>;

/** The retentions of a sweep. */
export const Retention = {
    /** Retain the digests of a known set. */
    of(retained: Iterable<string>): Retention {
        const set = new Set(retained);

        return async (digests) => new Set(digests.filter((digest) => set.has(digest)));
    },

    /** Retain none of the digests. */
    none: (async () => new Set<string>()) satisfies Retention,
};

/** Read a build's manifest at a package's registry, absent where none is stored. */
export type ManifestRead = (
    manifest: Digest,
    packageId: PackageId,
) => Promise<PackageManifest | undefined>;

/** Immutable package builds in a bucket: files by digest, and manifests by digest. */
export class PackageStore<
    StoreEnvironment extends Pick<Environment, "compression" | "crypto"> = Pick<
        Environment,
        "compression" | "crypto"
    >,
> {
    /** The bucket storing the builds. */
    readonly bucket: Bucket;
    /** The compression streaming the store's archives, and the crypto digesting its files. */
    readonly environment: StoreEnvironment;
    /** The reader of each build opened so far, by manifest digest, until its build is deleted. */
    readonly #readers = new Map<string, Promise<BuildReader>>();
    /** The builds of cache keys in flight, so callers asking at once build each key once. */
    readonly #building = new Map<string, Promise<Digest>>();
    /** The runtime builds being derived, by build and runtime. */
    readonly #attaching = new Map<string, Promise<Digest>>();

    /** Store builds in a bucket, streaming archives and digesting files in an environment. */
    constructor(bucket: Bucket, environment: StoreEnvironment) {
        this.bucket = bucket;
        this.environment = environment;
    }

    /** Build the bucket key of a stored file from its digest. */
    static fileKey(digest: Digest): string {
        return `${FILES_PREFIX}${digest}`;
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

    /** Store each file of a build, then its manifest, returning the manifest's digest. */
    async put(source: BuildReader, signal?: AbortSignal): Promise<Digest> {
        // parse the manifest and refuse reserved and duplicate file paths
        const manifest = PackageManifest.parse(source.manifest);
        const list = await source.distributed();
        requireDistinctPaths(list);

        // store each distinct file, a bounded number of streams at a time
        await inBatches(distinctFiles(list), signal, (file) =>
            this.putFile(file, () => source.stream(file.path, signal)),
        );

        // store the manifest last
        signal?.throwIfAborted();
        const bytes = encodeManifest(manifest);
        const file = await PackageFile.describe(
            "manifest.json",
            "application/json",
            bytes,
            this.environment.crypto,
        );
        await this.#put(`${MANIFESTS_PREFIX}${file.digest}`, bytes, file);

        return file.digest;
    }

    /** Report whether the store has a build, whose manifest it stores after every file. */
    async contains(digest: Digest): Promise<boolean> {
        return (await this.bucket.head(`${MANIFESTS_PREFIX}${digest}`)) !== null;
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

    /** Store an uploaded file under its digest, refusing bytes of another digest. */
    async receive(
        digest: string,
        bytes: Uint8Array<ArrayBuffer>,
        mediaType: string,
    ): Promise<void> {
        // refuse bytes of another digest
        const expected = Digest.parse(digest);
        if ((await Digest.of(bytes, this.environment.crypto)) !== expected) {
            throw BuildError.of(
                "DIGEST_INVALID",
                `the uploaded file does not match its digest ${digest}`,
            );
        }
        await this.putFile(
            {
                path: PackageStore.fileKey(expected),
                digest: expected,
                size: bytes.byteLength,
                mediaType,
            },
            async () => bytes,
        );
    }

    /** Store an uploaded manifest with all its files, refusing other digests and missing files. */
    async adopt(digest: Digest, bytes: Uint8Array<ArrayBuffer>): Promise<void> {
        // read the manifest its digest names
        if ((await Digest.of(bytes, this.environment.crypto)) !== digest) {
            throw BuildError.of(
                "DIGEST_INVALID",
                `the uploaded manifest does not match its digest ${digest}`,
            );
        }
        const manifest = PackageManifest.parse(
            JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes)),
        );

        // require the files the manifest names, then each file its lists name under a distinct path
        const reader = new BuildReader(
            manifest,
            async (path) =>
                (await this.open(present(references.get(path), `the listed file ${path}`))).body,
            this.environment.crypto,
        );
        const references = new Map(reader.references().map((file) => [file.path, file]));
        await this.#requireStored(digest, reader.references());
        await this.#requireStored(digest, await reader.files());
        requireDistinctPaths(await reader.distributed());

        // store the manifest last
        await this.#put(`${MANIFESTS_PREFIX}${digest}`, bytes, {
            path: "manifest.json",
            digest,
            size: bytes.byteLength,
            mediaType: "application/json",
        });
    }

    /** Store a file under its digest unless the bucket has it, opening its bytes only if needed. */
    async putFile(file: PackageFile, open: () => Promise<BucketBody>): Promise<void> {
        const key = PackageStore.fileKey(file.digest);
        if (!(await this.#has(key, file))) {
            await this.#put(key, await open(), file);
        }
    }

    /** Attach a stored file to a stored manifest as its referrer at a path, unless one exists. */
    async attach(subject: Digest, file: PackageFile): Promise<void> {
        // require the manifest and the file
        await this.head(subject);
        await this.#requireStored(subject, [file]);

        // record the referrer, unless another writer recorded one first
        const key = PackageStore.#referrerKey(subject, file.path);
        await this.bucket
            .put(key, JSON.stringify(file), {
                onlyIf: { etagDoesNotMatch: "*" },
                httpMetadata: { contentType: "application/json" },
            })
            .catch((error: unknown) => {
                if (!BucketError.is(error, "PRECONDITION_UNMET")) {
                    throw error;
                }
            });
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
        // list the subject's referrers, then read their descriptions at once
        const keys: string[] = [];
        for await (const entry of this.#list(`${REFERRERS_PREFIX}${subject}/`)) {
            keys.push(entry.key);
        }
        const read = await Promise.all(keys.map((key) => this.bucket.get(key)));

        return Promise.all(
            read.flatMap((object) =>
                object === null ? [] : [object.json().then((json) => PackageFile.parse(json))],
            ),
        );
    }

    /** Read the file attached to a manifest at a path, absent for a manifest without one. */
    async referrer(subject: Digest, path: string): Promise<PackageFile | undefined> {
        const object = await this.bucket.get(PackageStore.#referrerKey(subject, path));

        return object === null ? undefined : PackageFile.parse(await object.json());
    }

    /** Read and verify a build's manifest. */
    async get(digest: Digest): Promise<PackageManifest> {
        // read and verify the manifest bytes
        const object = await this.manifest(digest);
        const bytes = new Uint8Array(await object.arrayBuffer());
        await PackageFile.verify(
            { path: "manifest.json", digest, size: object.size, mediaType: "application/json" },
            bytes,
            this.environment.crypto,
        );

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
        const manifest = await this.get(digest);
        const reader = new BuildReader(
            manifest,
            async (path, signal) => {
                signal?.throwIfAborted();
                const object = await this.open(await select(path));

                return signal
                    ? object.body.pipeThrough(new TransformStream(), { signal })
                    : object.body;
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

    /** Read a cache key's entry, a miss when absent or when it refers to dropped objects. */
    async cached(key: string): Promise<CacheEntry | undefined> {
        // read the entry
        const object = await this.bucket.get(`cache/${Digest.parse(key)}`);
        if (!object) {
            return undefined;
        }
        const entry = CacheEntry.parse(await object.json());

        // miss an entry of a manifest or file retention removed since
        const named = PackageStore.#objects(entry);
        const stored = await Promise.all(named.map((name) => this.bucket.head(name)));

        return stored.every((file) => file !== null) ? entry : undefined;
    }

    /** Name an entry by a cache key, once the manifest or files it names are stored. */
    async cache(key: string, entry: CacheEntry): Promise<void> {
        await this.bucket.put(`cache/${Digest.parse(key)}`, JSON.stringify(entry), {
            httpMetadata: { contentType: "application/json" },
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
            const manifest = await build();
            await this.cache(key, { kind: "build", manifest });

            return manifest;
        })();
        this.#building.set(key, building);
        void building.finally(() => this.#building.delete(key)).catch(() => undefined);

        return building;
    }

    /** Lease a manifest or file until a moment, protecting it and its listed files from sweeps. */
    async lease(digest: Digest, until: Date): Promise<void> {
        await this.bucket.put(`leases/${digest}`, JSON.stringify(until.getTime()), {
            httpMetadata: { contentType: "application/json" },
        });
    }

    /** Set the manifests a root retains at a moment, such as a space's, replacing earlier ones. */
    async root(name: string, manifests: ReadonlySet<string>, now: Date): Promise<void> {
        const root = { rootedAt: now.getTime(), manifests: [...manifests].toSorted() };
        await this.bucket.put(`${ROOTS_PREFIX}${name}`, JSON.stringify(root), {
            httpMetadata: { contentType: "application/json" },
        });
    }

    /** Forget a root, retaining nothing by it. */
    async forget(name: string): Promise<void> {
        await this.bucket.delete(`${ROOTS_PREFIX}${name}`);
    }

    /** Read when a root was last written, in UTC epoch milliseconds, absent without one. */
    async rootedAt(name: string): Promise<number | undefined> {
        const object = await this.bucket.get(`${ROOTS_PREFIX}${name}`);

        return object === null ? undefined : Root.parse(await object.json()).rootedAt;
    }

    /** Read the manifests each root retains, by root name. */
    async roots(): Promise<ReadonlyMap<string, ReadonlySet<string>>> {
        const roots = new Map<string, ReadonlySet<string>>();
        for await (const file of this.#list(ROOTS_PREFIX)) {
            // skip a root forgotten since the listing
            const object = await this.bucket.get(file.key);
            if (object !== null) {
                const { manifests } = Root.parse(await object.json());
                roots.set(file.key.slice(ROOTS_PREFIX.length), new Set(manifests));
            }
        }

        return roots;
    }

    /** Read the manifests every root retains: with its attached builds and what they run with. */
    async retained(): Promise<ReadonlySet<string>> {
        // read the manifests the roots list, and each stored manifest once across them
        const roots = await this.roots();
        const listed = new Set([...roots.values()].flatMap((manifests) => [...manifests]));
        const manifests = new Map<Digest, Promise<PackageManifest | undefined>>();
        const stored = (digest: Digest) => {
            const reading =
                manifests.get(digest) ??
                this.contains(digest).then((isHere) => (isHere ? this.get(digest) : undefined));
            manifests.set(digest, reading);

            return reading;
        };
        const read: ManifestRead = (digest) => stored(digest);

        // retain each listed build, the builds attached to it, and the closures of all of them among the stored builds
        const retained = new Set<string>(listed);
        const pending = [...listed].map((listing) => Digest.parse(listing));
        for (let next = pending.pop(); next !== undefined; next = pending.pop()) {
            const found = await stored(next);
            if (found !== undefined) {
                const packageId = PackageManifest.package(found).id;
                const closure = await this.closure(next, packageId, read);
                const attached = await this.attached(next);
                for (const needed of [...closure, ...attached.map(({ file }) => file.digest)]) {
                    if (!retained.has(needed)) {
                        retained.add(needed);
                        pending.push(needed);
                    }
                }
            }
        }

        return retained;
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
        const { manifest } = reader;
        const [name, output] = present(
            Object.entries(manifest.outputs).find(([, each]) => each.runtime === runtime),
            `the ${runtime} output of ${manifest.package.name}`,
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
        const { commit } = manifest;
        const built = await this.#write(reader, [...declared, ...loaded], {
            package: manifest.package,
            ...(commit === undefined ? {} : { commit }),
            outputs: { [name]: { ...output, dependencies } },
        });

        // attach the maps to the runtime build when the output has any, then the runtime build to the build
        if (maps.length > 0) {
            const mapped = await this.#write(reader, maps, {
                package: manifest.package,
                ...(commit === undefined ? {} : { commit }),
            });
            await this.attachBuild(built, mapped, MAPS_REFERRER);
        }
        await this.attachBuild(digest, built, path);

        return built;
    }

    /** Store a build of some files of another build, answering its manifest's digest. */
    async #write(
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
        const manifest = await writer.finish(contents);

        return this.put(
            new BuildReader(
                manifest,
                async (path) => new Blob([present(written.get(path), path)]).stream(),
                this.environment.crypto,
            ),
        );
    }

    /** Attach a stored build to another at a path, storing its manifest as the referrer's file. */
    async attachBuild(subject: Digest, referrer: Digest, path: string): Promise<void> {
        const described = await this.head(referrer);
        await this.putFile(described, async () => (await this.manifest(referrer)).body);
        await this.attach(subject, { ...described, path });
    }

    /** List the npm units a build depends on, directly or through other units, as far as found. */
    async units(
        manifest: Digest,
        packageId: PackageId,
        read: ManifestRead = (digest) => this.get(digest),
    ): Promise<Digest[]> {
        // visit each unit once, from the build's outputs through each found unit's outputs, at the build's registry
        const units = new Set<Digest>();
        const pending = [manifest];
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
        manifest: Digest,
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
        const root = await once(manifest, packageId);
        const pending = Object.values(root?.outputs ?? {}).flatMap((output) =>
            Object.values(output.workloads).map((workload) => ({
                manifest,
                packageId,
                runtime: output.runtime,
                subpath: workload.entrypoint,
            })),
        );

        // follow each export once to the dependency exports its chunks import
        const units = new Map<Digest, PackageId>();
        const closure = new Set<Digest>([manifest]);
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

    /** Report whether a lease covers a manifest or file at a moment. */
    async leased(digest: Digest, at: Date): Promise<boolean> {
        const object = await this.bucket.get(`leases/${digest}`);

        return object !== null && Instant.parse(await object.json()) > at.getTime();
    }

    /** Delete what retention drops and was stored before a moment. */
    async sweep(
        retained: Retention,
        before: Date,
        files: Retention,
        now: Date,
        marked: Date = now,
    ): Promise<void> {
        // list the objects retention drops
        const candidates = await this.#candidates(retained, before, files);

        // read the marks
        const { marks, etag } = await this.#marks();

        // read the leases last
        const leases = await this.#leases();
        const leased = new Set(
            [...leases].filter(([, until]) => until > now).map(([digest]) => digest),
        );

        // spare the leased objects and the files a leased manifest lists
        const spared = new Set([...leased, ...(await this.#listedFiles(leased))]);

        // drop the expired leases
        const dropped = [...leases.keys()]
            .filter((digest) => !leased.has(digest))
            .map((digest) => `leases/${digest}`);

        // drop the unleased objects marked by the marking moment
        const next: Record<string, number> = {};
        for (const [key, digest] of candidates) {
            // clear the mark of a leased object
            if (spared.has(digest)) {
                continue;
            }

            // delete an object an earlier sweep marked by the marking moment
            const mark = marks[key];
            if (mark !== undefined && mark <= marked.getTime()) {
                dropped.push(key);
            }
            // mark any other object now
            else {
                next[key] = mark ?? now.getTime();
            }
        }
        await this.#delete(dropped);

        // replace the marks
        await this.#writeMarks(next, etag);

        // delete the cache entries of a deleted manifest or file
        await this.#deleteDanglingEntries();
    }

    /** Sweep what no root retains once an interval passed, answering the retained builds. */
    async sweepIfDue(
        before: Date,
        now: Date,
        interval: number,
    ): Promise<ReadonlySet<string> | undefined> {
        // leave a sweep that began within the interval
        const last = await this.bucket.get(SWEPT_KEY);
        const began = last === null ? undefined : Instant.parse(await last.json());
        if (began !== undefined && now.getTime() - began < interval) {
            return undefined;
        }

        // claim this sweep unless a concurrent one claimed it first
        try {
            await this.bucket.put(SWEPT_KEY, JSON.stringify(now.getTime()), {
                onlyIf: last === null ? { etagDoesNotMatch: "*" } : { etagMatches: last.etag },
                httpMetadata: { contentType: "application/json" },
            });
        } catch (error) {
            if (BucketError.is(error, "PRECONDITION_UNMET")) {
                return undefined;
            }
            throw error;
        }

        // sweep what the roots no longer retain, releasing the claim of a failed sweep for a retry
        try {
            const retained = await this.retained();
            await this.sweep(Retention.of(retained), before, Retention.none, now);

            return retained;
        } catch (error) {
            await this.bucket.delete(SWEPT_KEY);
            throw error;
        }
    }

    /** List the unretained manifests and the unlisted files stored before a moment, by key. */
    async #candidates(
        retained: Retention,
        before: Date,
        files: Retention,
    ): Promise<Map<string, string>> {
        // retain the manifests stored since the moment and those retention lists, by batch
        const live = new Set<string>();
        const candidates = new Map<string, string>();
        await this.#batches(MANIFESTS_PREFIX, async (batch) => {
            const old = batch
                .filter((file) => file.uploaded < before)
                .map((file) => PackageStore.#digest(file));
            const retainedDigests = await retained(old);
            for (const file of batch) {
                const digest = PackageStore.#digest(file);
                if (file.uploaded >= before || retainedDigests.has(digest)) {
                    live.add(digest);
                } else {
                    candidates.set(file.key, digest);
                }
            }
        });

        // drop the referrers of dropped manifests
        for await (const file of this.#list(REFERRERS_PREFIX)) {
            const subject = nth(file.key.slice(REFERRERS_PREFIX.length).split("/"), 0);
            if (!live.has(subject)) {
                candidates.set(file.key, subject);
            }
        }

        // retain the files of live manifests, the retained files and the files stored since the moment
        const listed = await this.#listedFiles(live);
        await this.#batches(FILES_PREFIX, async (batch) => {
            const old = batch.filter(
                (file) => file.uploaded < before && !listed.has(PackageStore.#digest(file)),
            );
            const retainedDigests = await files(old.map((file) => PackageStore.#digest(file)));
            for (const file of old) {
                if (!retainedDigests.has(PackageStore.#digest(file))) {
                    candidates.set(file.key, PackageStore.#digest(file));
                }
            }
        });

        return candidates;
    }

    /** Hand on the entries below a prefix in batches the bucket deletes at once. */
    async #batches(
        prefix: string,
        handle: (batch: readonly BucketEntry[]) => Promise<void>,
    ): Promise<void> {
        // collect each batch, handing on the last one too
        let batch: BucketEntry[] = [];
        for await (const file of this.#list(prefix)) {
            batch.push(file);
            if (batch.length === MAX_BATCH_FILES) {
                await handle(batch);
                batch = [];
            }
        }
        if (batch.length > 0) {
            await handle(batch);
        }
    }

    /** Replace the sweep marks, deleting the marks object when no mark remains. */
    async #writeMarks(
        marks: Readonly<Record<string, number>>,
        etag: string | undefined,
    ): Promise<void> {
        // delete the marks object without marks
        if (Object.keys(marks).length === 0) {
            if (etag !== undefined) {
                await this.bucket.delete(MARKS_KEY);
            }
        }
        // replace the marks this sweep read, leaving them to an overlapping sweep that wrote first
        else {
            try {
                await this.bucket.put(MARKS_KEY, JSON.stringify(marks), {
                    onlyIf: etag === undefined ? { etagDoesNotMatch: "*" } : { etagMatches: etag },
                    httpMetadata: { contentType: "application/json" },
                });
            } catch (error) {
                if (!BucketError.is(error, "PRECONDITION_UNMET")) {
                    throw error;
                }
            }
        }
    }

    /** Delete the cache entries of a deleted manifest or file. */
    async #deleteDanglingEntries(): Promise<void> {
        const dangling: string[] = [];
        for await (const file of this.#list("cache/")) {
            if ((await this.cached(file.key.slice("cache/".length))) === undefined) {
                dangling.push(file.key);
            }
        }
        await this.#delete(dangling);
    }

    /** Read one complete file and verify its exact bytes. */
    async file(file: PackageFile): Promise<Uint8Array<ArrayBuffer>> {
        // read and verify the file bytes
        const object = await this.open(file);
        const bytes = new Uint8Array(await object.arrayBuffer());
        await PackageFile.verify(file, bytes, this.environment.crypto);

        return bytes;
    }

    /** Describe a build's manifest file from the bucket's metadata, leaving its bytes unread. */
    async head(digest: Digest): Promise<PackageFile> {
        // read the manifest's metadata by its digest
        const stored = await this.bucket.head(`${MANIFESTS_PREFIX}${digest}`);
        if (!stored) {
            throw BuildError.of("MANIFEST_UNKNOWN", "package manifest not found");
        }

        return { path: "manifest.json", digest, size: stored.size, mediaType: "application/json" };
    }

    /** Stream a manifest by its digest. */
    async manifest(digest: Digest, range?: BucketRange) {
        // read the manifest by its digest
        const object = await this.bucket.get(
            `${MANIFESTS_PREFIX}${digest}`,
            range === undefined ? {} : { range },
        );
        if (!object) {
            throw BuildError.of("MANIFEST_UNKNOWN", "package manifest not found");
        }

        return object;
    }

    /** Stream a stored file, refusing one whose size or digest differs. */
    async open(file: PackageFile, range?: BucketRange) {
        // read the object and refuse a size or digest mismatch
        const object = await this.bucket.get(
            PackageStore.fileKey(file.digest),
            range === undefined ? {} : { range },
        );
        if (!object) {
            throw BuildError.of("BLOB_UNKNOWN", `package file not found: ${file.path}`);
        }
        if (!isStored(object, file)) {
            await object.body.cancel();
            throw BuildError.of("STORED_CONTENT_DIFFERS", `stored file differs: ${file.path}`);
        }

        return object;
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

    /** Require the store to have each file a build's manifest lists. */
    async #requireStored(digest: string, files: readonly PackageFile[]): Promise<void> {
        for (const file of files) {
            if (!(await this.#has(PackageStore.fileKey(file.digest), file))) {
                throw BuildError.of(
                    "MANIFEST_BLOB_UNKNOWN",
                    `build ${digest} lists ${file.path}, which was not uploaded`,
                );
            }
        }
    }

    /** Collect the digests of the files the store's retained manifests list or attach. */
    async #listedFiles(retained: ReadonlySet<string>): Promise<Set<string>> {
        const referenced = new Set<string>();
        for (const retainedDigest of retained) {
            // skip a retained manifest the store lacks
            const digest = Digest.parse(retainedDigest);
            if ((await this.bucket.head(`${MANIFESTS_PREFIX}${digest}`)) === null) {
                continue;
            }

            // retain every file the manifest lists and every file attached to it
            const reader = await this.contents(digest);
            for (const file of await reader.distributed()) {
                referenced.add(file.digest);
            }
            for await (const entry of this.#list(`${REFERRERS_PREFIX}${digest}/`)) {
                // skip a referrer a concurrent sweep deleted since the listing
                const object = await this.bucket.get(entry.key);
                if (object !== null) {
                    referenced.add(PackageFile.parse(await object.json()).digest);
                }
            }
        }

        return referenced;
    }

    /** Read the marks the last sweep left, by key. */
    async #marks(): Promise<{
        readonly marks: Record<string, number>;
        readonly etag: string | undefined;
    }> {
        const object = await this.bucket.get(MARKS_KEY);

        return object === null
            ? { marks: {}, etag: undefined }
            : { marks: Marks.parse(await object.json()), etag: object.etag };
    }

    /** Read every lease's expiry by its digest. */
    async #leases(): Promise<Map<string, Date>> {
        const leases = new Map<string, Date>();
        for await (const file of this.#list("leases/")) {
            // skip a lease another sweep deleted since the listing, which only deletes expired ones
            const object = await this.bucket.get(file.key);
            if (object === null) {
                continue;
            }
            const until = Instant.parse(await object.json());
            leases.set(file.key.slice("leases/".length), new Date(until));
        }

        return leases;
    }

    /** List the bucket's files below a prefix, page by page. */
    async *#list(prefix: string): AsyncGenerator<BucketEntry> {
        let cursor: string | undefined;
        do {
            const page = await this.bucket.list({
                prefix,
                ...(cursor === undefined ? {} : { cursor }),
            });
            yield* page.files;
            cursor = page.cursor;
        } while (cursor !== undefined);
    }

    /** Delete files in batches the bucket accepts, forgetting the readers of deleted builds. */
    async #delete(keys: readonly string[]): Promise<void> {
        // forget the readers of the builds whose manifests go
        for (const key of keys.filter((each) => each.startsWith(MANIFESTS_PREFIX))) {
            this.#readers.delete(key.slice(MANIFESTS_PREFIX.length));
        }
        for (let start = 0; start < keys.length; start += MAX_BATCH_FILES) {
            await this.bucket.delete(keys.slice(start, start + MAX_BATCH_FILES));
        }
    }

    /** List the objects a cache entry names: a manifest, or files. */
    static #objects(entry: CacheEntry): string[] {
        switch (entry.kind) {
            // key a whole build's manifest
            case "build":
                return [`${MANIFESTS_PREFIX}${entry.manifest}`];
            // key an output's files
            case "output":
                return entry.output.files.map((file) => PackageStore.fileKey(file.digest));
            // name a transform's file
            case "file":
                return [PackageStore.fileKey(entry.file.digest)];
        }
    }

    /** Decide whether the bucket has a file's exact bytes at its key, refusing other bytes. */
    async #has(key: string, file: PackageFile): Promise<boolean> {
        const stored = await this.bucket.head(key);
        if (stored === null) {
            return false;
        } else if (!isStored(stored, file)) {
            throw BuildError.of("STORED_CONTENT_DIFFERS", `stored object differs: ${key}`);
        }

        return true;
    }

    /** Create an immutable checksum-verified object, or take one another writer created first. */
    async #put(key: string, body: BucketBody, file: PackageFile): Promise<void> {
        // create the object, or read the object another writer created first
        const stored = await this.bucket
            .put(key, body, {
                sha256: file.digest,
                httpMetadata: { contentType: file.mediaType },
                onlyIf: { etagDoesNotMatch: "*" },
            })
            .catch((error: unknown) => {
                // read the existing object
                if (BucketError.is(error, "PRECONDITION_UNMET")) {
                    const { current } = error.data;

                    return current === null ? null : BucketFile.read(current);
                }
                // rethrow any other failure
                else {
                    throw error;
                }
            });

        // require the stored object to match the file
        if (!stored || !isStored(stored, file)) {
            throw BuildError.of("STORED_CONTENT_DIFFERS", `stored object differs: ${key}`);
        }
    }

    /** Key the referrer of a manifest at a path. */
    static #referrerKey(subject: string, path: string): string {
        return `${REFERRERS_PREFIX}${Digest.parse(subject)}/${path}`;
    }

    /** Read the digest of a manifest or file entry from its key. */
    static #digest(file: BucketEntry): string {
        return file.key.slice(file.key.indexOf("/") + 1);
    }
}

/** Encode a parsed manifest as its stored bytes, whose digest identifies the build. */
export function encodeManifest(manifest: PackageManifest): Uint8Array<ArrayBuffer> {
    return new TextEncoder().encode(JSON.stringify(manifest));
}

/** Report whether a bucket object has a file's exact size and digest. */
function isStored(object: BucketFile, file: PackageFile): boolean {
    return object.size === file.size && object.digest.toJSON().sha256 === file.digest;
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
