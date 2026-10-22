import { Digest, Instant, nth, present, schema } from "@destack/schema";
import {
    type Bucket,
    type BucketBody,
    type BucketEntry,
    type BucketFile,
    BucketPreconditionError,
    type BucketRange,
    MAX_BATCH_FILES,
} from "@destack/bucket";
import { PackageFile } from "@destack/package/file";
import { BuildReader, type PackageLocation, PackageManifest } from "@destack/package/manifest";
import { CallError } from "@destack/object";
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

/** The key of the instant the last sweep began on its machine's clock, kept close by NTP. */
const SWEPT_KEY = "swept";

/** The marks of the objects a sweep drops, by key. */
const Marks = schema.record(schema.string(), Instant);

/** The prefix of the keys of the roots, each retaining manifests under its name. */
const ROOTS_PREFIX = "roots/";

/** The manifests one root retains. */
const Roots = schema.array(Digest);

/** Decide which of some stored digests a sweep keeps, read in batches the bucket lists. */
export type Retention = (digests: readonly string[]) => Promise<ReadonlySet<string>>;

/** The retentions of a sweep. */
export const Retention = {
    /** Keep the digests of a known set. */
    of(kept: Iterable<string>): Retention {
        const set = new Set(kept);

        return async (digests) => new Set(digests.filter((digest) => set.has(digest)));
    },

    /** Keep none of the digests. */
    none: (async () => new Set<string>()) satisfies Retention,
};

/** Immutable package builds in a bucket: files by digest, and manifests by digest. */
export class PackageStore {
    /** The bucket storing the builds. */
    readonly bucket: Bucket;
    /** The reader of each build opened so far, by manifest digest, until its build is deleted. */
    readonly #readers = new Map<string, Promise<BuildReader>>();
    /** The builds of cache keys in flight, so callers asking at once build each key once. */
    readonly #building = new Map<string, Promise<string>>();

    /** Store builds in a bucket. */
    constructor(bucket: Bucket) {
        this.bucket = bucket;
    }

    /** Build the bucket key of a stored file from its digest. */
    static fileKey(digest: string): string {
        return `${FILES_PREFIX}${digest}`;
    }

    /** Serve the store's manifests, files and archives below a mount, as a host allows each. */
    fetch(request: Request, mount: URL, access: PackageAccess): Promise<Response> {
        return serveStore(this, mount, request, access);
    }

    /** Name a build by the digest of its manifest, as a store keeps it. */
    static async digest(manifest: PackageManifest): Promise<Digest> {
        const bytes = encodeManifest(PackageManifest.parse(manifest));

        return Digest.parse(
            (await PackageFile.describe("manifest.json", "application/json", bytes)).digest,
        );
    }

    /** Store each file of a build, then its manifest, returning the manifest's digest. */
    async put(source: BuildReader, signal?: AbortSignal): Promise<string> {
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
        const file = await PackageFile.describe("manifest.json", "application/json", bytes);
        await this.#put(`${MANIFESTS_PREFIX}${file.digest}`, bytes, file);

        return file.digest;
    }

    /** Report whether the store keeps a build, whose manifest it stores after every file. */
    async contains(digest: string): Promise<boolean> {
        return (await this.bucket.head(`${MANIFESTS_PREFIX}${Digest.parse(digest)}`)) !== null;
    }

    /** Copy a served build by manifest digest unless kept, fetching only missing files. */
    async copy(
        location: PackageLocation,
        fetch: (input: URL, init: RequestInit) => Promise<Response>,
        signal?: AbortSignal,
    ): Promise<void> {
        // keep a build the store has
        if (await this.contains(location.manifest)) {
            return;
        }

        // read the served manifest, verified by its digest
        const reader = await BuildReader.open(location, fetch, signal);

        // store each missing file as served, then the manifest
        const digest = await this.put(reader, signal);
        if (digest !== location.manifest) {
            throw new CallError("BAD_GATEWAY", {
                message: `build ${location.manifest} was stored as ${digest}`,
            });
        }
    }

    /** Store an uploaded file under its digest, refusing bytes of another digest. */
    async receive(
        digest: string,
        bytes: Uint8Array<ArrayBuffer>,
        mediaType: string,
    ): Promise<void> {
        if ((await Digest.of(bytes)) !== Digest.parse(digest)) {
            throw new CallError("BAD_REQUEST", {
                message: `the uploaded file does not match its digest ${digest}`,
            });
        }
        await this.putFile(
            { path: PackageStore.fileKey(digest), digest, size: bytes.byteLength, mediaType },
            async () => bytes,
        );
    }

    /** Store an uploaded manifest with all its files, refusing other digests and missing files. */
    async adopt(digest: string, bytes: Uint8Array<ArrayBuffer>): Promise<void> {
        // read the manifest its digest names
        if ((await Digest.of(bytes)) !== Digest.parse(digest)) {
            throw new CallError("BAD_REQUEST", {
                message: `the uploaded manifest does not match its digest ${digest}`,
            });
        }
        const manifest = PackageManifest.parse(
            JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes)),
        );

        // require the files the manifest names, then each file its lists name under a distinct path
        const reader = new BuildReader(
            manifest,
            async (path) =>
                (await this.open(present(references.get(path), `the listed file ${path}`))).body,
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

    /** Attach a stored file to a stored manifest as its referrer at a path, keeping a first one. */
    async attach(subject: string, file: PackageFile): Promise<void> {
        // require the manifest and the file
        await this.head(subject);
        await this.#requireStored(subject, [file]);

        // record the referrer, keeping the one another writer recorded first
        const key = PackageStore.#referrerKey(subject, file.path);
        await this.bucket
            .put(key, JSON.stringify(file), {
                onlyIf: { etagDoesNotMatch: "*" },
                httpMetadata: { contentType: "application/json" },
            })
            .catch((error: unknown) => {
                if (!(error instanceof BucketPreconditionError)) {
                    throw error;
                }
            });
    }

    /** Read the file attached to a manifest at a path, absent for a manifest without one. */
    async referrer(subject: string, path: string): Promise<PackageFile | undefined> {
        const object = await this.bucket.get(PackageStore.#referrerKey(subject, path));

        return object === null ? undefined : PackageFile.parse(await object.json());
    }

    /** Read and verify a build's manifest. */
    async get(digest: string): Promise<PackageManifest> {
        // read and verify the manifest bytes
        const object = await this.manifest(digest);
        const bytes = new Uint8Array(await object.arrayBuffer());
        await PackageFile.verify(
            { path: "manifest.json", digest, size: object.size, mediaType: "application/json" },
            bytes,
        );

        return PackageManifest.parse(
            JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes)),
        );
    }

    /** Open a stored build for selective reads, one reader per build, as its files never change. */
    contents(digest: string): Promise<BuildReader> {
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
    async #open(digest: string): Promise<BuildReader> {
        // index the files the manifest lists directly
        const manifest = await this.get(digest);
        const reader = new BuildReader(manifest, async (path, signal) => {
            signal?.throwIfAborted();
            const object = await this.open(await select(path));

            return signal
                ? object.body.pipeThrough(new TransformStream(), { signal })
                : object.body;
        });
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
                throw new CallError("NOT_FOUND", { message: `package file not found: ${path}` });
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
    cachedBuild(key: string, build: () => Promise<string>): Promise<string> {
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

    /** Lease a manifest or file until a moment, keeping it and its listed files from sweeps. */
    async lease(digest: string, until: Date): Promise<void> {
        await this.bucket.put(`leases/${Digest.parse(digest)}`, JSON.stringify(until.getTime()), {
            httpMetadata: { contentType: "application/json" },
        });
    }

    /** Keep the manifests a root retains, such as a space's, replacing what it retained before. */
    async root(name: string, manifests: ReadonlySet<string>): Promise<void> {
        // forget a root that retains nothing
        const key = `${ROOTS_PREFIX}${name}`;
        if (manifests.size === 0) {
            await this.bucket.delete(key);

            return;
        }

        // keep the root's manifests
        await this.bucket.put(key, JSON.stringify([...manifests].toSorted()), {
            httpMetadata: { contentType: "application/json" },
        });
    }

    /** Read the manifests each root retains, by root name. */
    async roots(): Promise<ReadonlyMap<string, ReadonlySet<string>>> {
        const roots = new Map<string, ReadonlySet<string>>();
        for await (const file of this.#list(ROOTS_PREFIX)) {
            // skip a root forgotten since the listing
            const object = await this.bucket.get(file.key);
            if (object !== null) {
                roots.set(
                    file.key.slice(ROOTS_PREFIX.length),
                    new Set(Roots.parse(await object.json())),
                );
            }
        }

        return roots;
    }

    /** Read the manifests every root retains. */
    async retained(): Promise<ReadonlySet<string>> {
        const roots = await this.roots();

        return new Set([...roots.values()].flatMap((manifests) => [...manifests]));
    }

    /** Report whether a lease keeps a manifest or file at a moment. */
    async leased(digest: string, at: Date): Promise<boolean> {
        const object = await this.bucket.get(`leases/${Digest.parse(digest)}`);

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
            if (error instanceof BucketPreconditionError) {
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
        // keep the manifests stored since the moment and those retention keeps, by batch
        const kept = new Set<string>();
        const candidates = new Map<string, string>();
        await this.#batches(MANIFESTS_PREFIX, async (batch) => {
            const old = batch
                .filter((file) => file.uploaded < before)
                .map((file) => PackageStore.#digest(file));
            const retainedDigests = await retained(old);
            for (const file of batch) {
                const digest = PackageStore.#digest(file);
                if (file.uploaded >= before || retainedDigests.has(digest)) {
                    kept.add(digest);
                } else {
                    candidates.set(file.key, digest);
                }
            }
        });

        // drop the referrers of dropped manifests
        for await (const file of this.#list(REFERRERS_PREFIX)) {
            const subject = nth(file.key.slice(REFERRERS_PREFIX.length).split("/"), 0);
            if (!kept.has(subject)) {
                candidates.set(file.key, subject);
            }
        }

        // keep the files of kept manifests, the retained files and the files stored since the moment
        const listed = await this.#listedFiles(kept);
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
                if (!(error instanceof BucketPreconditionError)) {
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
        await PackageFile.verify(file, bytes);

        return bytes;
    }

    /** Describe a build's manifest file from the bucket's metadata, leaving its bytes unread. */
    async head(digest: string): Promise<PackageFile> {
        // read the manifest's metadata by its validated digest
        Digest.parse(digest);
        const stored = await this.bucket.head(`${MANIFESTS_PREFIX}${digest}`);
        if (!stored) {
            throw new CallError("NOT_FOUND", { message: "package manifest not found" });
        }

        return { path: "manifest.json", digest, size: stored.size, mediaType: "application/json" };
    }

    /** Stream a manifest by its digest. */
    async manifest(digest: string, range?: BucketRange) {
        // read the manifest by its validated digest
        Digest.parse(digest);
        const object = await this.bucket.get(
            `${MANIFESTS_PREFIX}${digest}`,
            range === undefined ? {} : { range },
        );
        if (!object) {
            throw new CallError("NOT_FOUND", { message: "package manifest not found" });
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
            throw new CallError("NOT_FOUND", {
                message: `package file not found: ${file.path}`,
            });
        }
        if (!isStored(object, file)) {
            await object.body.cancel();
            throw new CallError("INTERNAL_SERVER_ERROR", {
                message: `stored file differs: ${file.path}`,
            });
        }

        return object;
    }

    /** Stream a stored build as a tarball of its manifest and files in path order, verified. */
    export(digest: string): ReadableStream<Uint8Array<ArrayBuffer>> {
        return Tarball.stream(this.#entries(digest));
    }

    /** Read a stored build's manifest, then each file in path order. */
    async *#entries(digest: string) {
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

    /** Require the store to hold each file a build's manifest lists. */
    async #requireStored(digest: string, files: readonly PackageFile[]): Promise<void> {
        for (const file of files) {
            if (!(await this.#has(PackageStore.fileKey(file.digest), file))) {
                throw new CallError("CONFLICT", {
                    message: `build ${digest} lists ${file.path}, which was not uploaded`,
                });
            }
        }
    }

    /** Collect the digests of the files the retained manifests the store holds list or attach. */
    async #listedFiles(retained: ReadonlySet<string>): Promise<Set<string>> {
        const kept = new Set<string>();
        for (const digest of retained) {
            // skip a retained manifest the store lacks
            if ((await this.bucket.head(`${MANIFESTS_PREFIX}${Digest.parse(digest)}`)) === null) {
                continue;
            }

            // keep every file the manifest lists and every file attached to it
            const reader = await this.contents(digest);
            for (const file of await reader.distributed()) {
                kept.add(file.digest);
            }
            for await (const entry of this.#list(`${REFERRERS_PREFIX}${digest}/`)) {
                // skip a referrer a concurrent sweep deleted since the listing
                const object = await this.bucket.get(entry.key);
                if (object !== null) {
                    kept.add(PackageFile.parse(await object.json()).digest);
                }
            }
        }

        return kept;
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
            throw new CallError("INTERNAL_SERVER_ERROR", {
                message: `stored object differs: ${key}`,
            });
        }

        return true;
    }

    /** Create an immutable checksum-verified object, or keep one another writer created first. */
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
                if (error instanceof BucketPreconditionError) {
                    return error.current;
                }
                // rethrow any other failure
                else {
                    throw error;
                }
            });

        // require the stored object to hold the file
        if (!stored || !isStored(stored, file)) {
            throw new CallError("INTERNAL_SERVER_ERROR", {
                message: `stored object differs: ${key}`,
            });
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

/** Report whether a bucket object holds a file's exact size and digest. */
function isStored(object: BucketFile, file: PackageFile): boolean {
    return object.size === file.size && object.digest.toJSON().sha256 === file.digest;
}

/** Keep the first file of each digest. */
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

/** Refuse a file list naming the reserved manifest path or one path twice. */
function requireDistinctPaths(list: readonly PackageFile[]): void {
    const paths = new Set<string>();
    for (const file of list) {
        if (file.path === "manifest.json" || file.path.startsWith("manifest.json/")) {
            throw new CallError("BAD_REQUEST", { message: `reserved build path: ${file.path}` });
        }
        if (paths.has(file.path)) {
            throw new CallError("BAD_REQUEST", {
                message: `duplicate package file: ${file.path}`,
            });
        }
        paths.add(file.path);
    }
}
