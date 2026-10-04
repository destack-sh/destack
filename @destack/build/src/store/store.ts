import { Digest, Instant, present, schema } from "@destack/schema";
import {
    type Bucket,
    type BucketBody,
    type BucketFile,
    type BucketRange,
    MAX_BATCH_FILES,
} from "@destack/bucket";
import { PackageFile } from "@destack/package/file";
import {
    BuildReader,
    type PackageDistribution,
    type PackageLocation,
    PackageManifest,
} from "@destack/package/manifest";
import { ServiceError } from "@destack/service/error";
import { Tarball } from "@destack/package/archive";
import { comparePath } from "../build/serialization.ts";
import { CacheEntry } from "../cache/index.ts";

/** The uploads one store runs at once: at tens of milliseconds per R2 upload, eight store 2000 files in about 10 seconds. */
const CONCURRENT_UPLOADS = 8;

/** The key of the marks a sweep leaves for the next one: each dropped object's key to the instant a sweep first marked it. */
const MARKS_KEY = "marks";

/** The marks of the objects a sweep drops, by key. */
const Marks = schema.record(schema.string(), Instant);

/** Immutable package builds in a bucket: files by digest, and manifests by digest. */
export class PackageStore {
    /** The bucket storing the builds. */
    readonly bucket: Bucket;

    /** Store builds in a bucket. */
    constructor(bucket: Bucket) {
        this.bucket = bucket;
    }

    /** Store each file of a build, then its manifest, returning the manifest's digest. */
    async put(source: PackageDistribution, signal?: AbortSignal): Promise<string> {
        // parse the manifest and refuse reserved and duplicate file paths
        const manifest = PackageManifest.parse(source.manifest);
        const list = await source.reader.distributed();
        requireDistinctPaths(list);

        // store each distinct file, a bounded number of streams at a time
        await inBatches(distinctFiles(list), signal, (file) =>
            this.putFile(file, () => source.open(file.path, signal)),
        );

        // store the manifest last
        signal?.throwIfAborted();
        const bytes = new TextEncoder().encode(JSON.stringify(manifest));
        const file = await PackageFile.describe("manifest.json", "application/json", bytes);
        await this.#put(`manifests/${file.digest}`, bytes, file);

        return file.digest;
    }

    /** Report whether the store keeps a build, whose manifest it stores after every file. */
    async contains(digest: string): Promise<boolean> {
        return (await this.bucket.head(`manifests/${Digest.parse(digest)}`)) !== null;
    }

    /** Copy a build a package server serves by its manifest's digest, fetching only the files the store lacks. */
    async copy(
        location: PackageLocation,
        fetch: (input: URL, init: RequestInit) => Promise<Response>,
        signal?: AbortSignal,
    ): Promise<void> {
        // read the served manifest, verified by its digest
        const reader = await BuildReader.open(location, fetch, signal);
        const base = new URL(location.url.endsWith("/") ? location.url : `${location.url}/`);

        // store each missing file as served, then the manifest
        const open = (path: string) => fetchFile(fetch, base, path, location.manifest, signal);
        const digest = await this.put({ manifest: reader.manifest, reader, open }, signal);
        if (digest !== location.manifest) {
            throw new ServiceError("BAD_GATEWAY", {
                message: `build ${location.manifest} was stored as ${digest}`,
            });
        }
    }

    /** Upload a kept build to the package server at a URL: each file the server lacks, then the manifest. */
    async push(
        digest: string,
        url: string,
        fetch: (request: Request) => Promise<Response>,
        signal?: AbortSignal,
    ): Promise<void> {
        // upload each distinct file the server lacks, a bounded number at a time
        const { reader } = await this.contents(digest);
        const files = distinctFiles(await reader.distributed());
        const base = url.endsWith("/") ? url : `${url}/`;
        await inBatches(files, signal, (file) =>
            this.#offer(fetch, `${base}files/${file.digest}`, file),
        );

        // upload the manifest last
        signal?.throwIfAborted();
        const manifest = new Uint8Array(await (await this.manifest(digest)).arrayBuffer());
        await upload(
            fetch,
            new Request(`${base}${digest}/manifest.json`, {
                method: "PUT",
                body: manifest,
                headers: { "content-type": "application/json" },
            }),
        );
    }

    /** Store an uploaded file under its digest, refusing bytes of another digest. */
    async receive(
        digest: string,
        bytes: Uint8Array<ArrayBuffer>,
        mediaType: string,
    ): Promise<void> {
        if ((await Digest.of(bytes)) !== Digest.parse(digest)) {
            throw new ServiceError("BAD_REQUEST", {
                message: `the uploaded file does not match its digest ${digest}`,
            });
        }
        await this.putFile(
            { path: `files/${digest}`, digest, size: bytes.byteLength, mediaType },
            async () => bytes,
        );
    }

    /** Store an uploaded manifest whose files the store holds, refusing bytes of another digest and a build lacking a file. */
    async adopt(digest: string, bytes: Uint8Array<ArrayBuffer>): Promise<void> {
        // read the manifest its digest names
        if ((await Digest.of(bytes)) !== Digest.parse(digest)) {
            throw new ServiceError("BAD_REQUEST", {
                message: `the uploaded manifest does not match its digest ${digest}`,
            });
        }
        const manifest = PackageManifest.parse(
            JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes)),
        );

        // require the files the manifest names, then each file its lists name under a distinct path
        const reader = new BuildReader(manifest, (path) =>
            this.file(present(references.get(path), `the listed file ${path}`)),
        );
        const references = new Map(reader.references().map((file) => [file.path, file]));
        await this.#requireStored(digest, reader.references());
        await this.#requireStored(digest, await reader.files());
        requireDistinctPaths(await reader.distributed());

        // store the manifest last
        await this.#put(`manifests/${digest}`, bytes, {
            path: "manifest.json",
            digest,
            size: bytes.byteLength,
            mediaType: "application/json",
        });
    }

    /** Store a file under its digest unless the bucket has it, opening its bytes only when needed. */
    async putFile(file: PackageFile, open: () => Promise<BucketBody>): Promise<void> {
        const key = `files/${file.digest}`;
        if (!(await this.#has(key, file))) {
            await this.#put(key, await open(), file);
        }
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

    /** Open a stored build for selective reads. */
    async contents(digest: string): Promise<PackageDistribution> {
        // index the files the manifest lists directly
        const manifest = await this.get(digest);
        const reader = new BuildReader(manifest, async (path) => this.file(await select(path)));
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
                throw new ServiceError("NOT_FOUND", { message: `package file not found: ${path}` });
            }

            return file;
        };

        return {
            manifest,
            reader,
            open: async (path, signal) => {
                signal?.throwIfAborted();
                const object = await this.open(await select(path));

                return signal
                    ? object.body.pipeThrough(new TransformStream(), { signal })
                    : object.body;
            },
        };
    }

    /** Read the entry a cache key names, a miss when it names none or names something the store no longer keeps. */
    async cached(key: string): Promise<CacheEntry | undefined> {
        // read the entry
        const object = await this.bucket.get(`cache/${Digest.parse(key)}`);
        if (!object) {
            return undefined;
        }
        const entry = CacheEntry.parse(await object.json());

        // miss an entry naming a manifest or file retention removed since
        const named =
            entry.kind === "build"
                ? [`manifests/${entry.manifest}`]
                : entry.output.files.map((file) => `files/${file.digest}`);
        const stored = await Promise.all(named.map((name) => this.bucket.head(name)));

        return stored.every((file) => file !== null) ? entry : undefined;
    }

    /** Name an entry by a cache key, once the manifest or files it names are stored. */
    async cache(key: string, entry: CacheEntry): Promise<void> {
        await this.bucket.put(`cache/${Digest.parse(key)}`, JSON.stringify(entry), {
            httpMetadata: { contentType: "application/json" },
        });
    }

    /** Lease a manifest or file by its digest until a moment, keeping it and the files a leased manifest lists from sweeps. */
    async lease(digest: string, until: Date): Promise<void> {
        await this.bucket.put(`leases/${Digest.parse(digest)}`, JSON.stringify(until.getTime()), {
            httpMetadata: { contentType: "application/json" },
        });
    }

    /**
     * Delete what retention drops, stored before a moment: unretained manifests, files no kept manifest lists, and cache entries naming either.
     *
     * Manifests stored since the moment stay, with every file they list.
     * Retained files stay too, such as archives no manifest lists.
     * Leased manifests and files stay until their lease expires, with every file a leased manifest lists.
     * A sweep marks each object it drops, and a later sweep deletes it once it was marked at or before the marking moment.
     */
    async sweep(
        retained: ReadonlySet<string>,
        before: Date,
        files: ReadonlySet<string> = new Set(),
        now: Date = new Date(),
        marked: Date = now,
    ): Promise<void> {
        // list the objects retention drops
        const candidates = await this.#candidates(retained, before, files);

        // read the marks
        const marks = await this.#marks();

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
        await this.#writeMarks(next);

        // delete the cache entries naming a deleted manifest or file
        await this.#deleteDanglingEntries();
    }

    /** List the unretained manifests and the unlisted files stored before a moment, by key. */
    async #candidates(
        retained: ReadonlySet<string>,
        before: Date,
        files: ReadonlySet<string>,
    ): Promise<Map<string, string>> {
        // keep the retained manifests and the manifests stored since the moment
        const manifests = new Set(retained);
        const candidates = new Map<string, string>();
        for await (const file of this.#list("manifests/")) {
            const digest = file.key.slice("manifests/".length);
            if (file.uploaded >= before) {
                manifests.add(digest);
            } else if (!retained.has(digest)) {
                candidates.set(file.key, digest);
            }
        }

        // keep the files a kept manifest lists, the retained files and the files stored since the moment
        const listed = new Set([...files, ...(await this.#listedFiles(manifests))]);
        for await (const file of this.#list("files/")) {
            const digest = file.key.slice("files/".length);
            if (!listed.has(digest) && file.uploaded < before) {
                candidates.set(file.key, digest);
            }
        }

        return candidates;
    }

    /** Replace the sweep marks, deleting the marks object when no mark remains. */
    async #writeMarks(marks: Readonly<Record<string, number>>): Promise<void> {
        // delete the marks object without marks
        if (Object.keys(marks).length === 0) {
            await this.bucket.delete(MARKS_KEY);
        }
        // write the remaining marks
        else {
            await this.bucket.put(MARKS_KEY, JSON.stringify(marks), {
                httpMetadata: { contentType: "application/json" },
            });
        }
    }

    /** Delete the cache entries naming a deleted manifest or file. */
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
        const stored = await this.bucket.head(`manifests/${digest}`);
        if (!stored) {
            throw new ServiceError("NOT_FOUND", { message: "package manifest not found" });
        }

        return { path: "manifest.json", digest, size: stored.size, mediaType: "application/json" };
    }

    /** Stream a manifest by its digest. */
    async manifest(digest: string, range?: BucketRange) {
        // read the manifest by its validated digest
        Digest.parse(digest);
        const object = await this.bucket.get(
            `manifests/${digest}`,
            range === undefined ? {} : { range },
        );
        if (!object) {
            throw new ServiceError("NOT_FOUND", { message: "package manifest not found" });
        }

        return object;
    }

    /** Stream a stored file, refusing one whose size or digest differs. */
    async open(file: PackageFile, range?: BucketRange) {
        // read the object and refuse a size or digest mismatch
        const object = await this.bucket.get(
            `files/${file.digest}`,
            range === undefined ? {} : { range },
        );
        if (!object) {
            throw new ServiceError("NOT_FOUND", {
                message: `package file not found: ${file.path}`,
            });
        }
        if (!isStored(object, file)) {
            await object.body.cancel();
            throw new ServiceError("INTERNAL_SERVER_ERROR", {
                message: `stored file differs: ${file.path}`,
            });
        }

        return object;
    }

    /** Stream a stored build as a tarball of its manifest and its files in path order, each verified before it is written. */
    export(digest: string): ReadableStream<Uint8Array<ArrayBuffer>> {
        return Tarball.stream(this.#entries(digest));
    }

    /** Read a stored build's manifest, then each file in path order. */
    async *#entries(digest: string) {
        // yield the manifest first
        const contents = await this.contents(digest);
        yield {
            path: "manifest.json",
            contents: new TextEncoder().encode(JSON.stringify(contents.manifest)),
        };

        // yield each file after verifying its bytes
        const list = await contents.reader.distributed();
        for (const file of list.toSorted(comparePath)) {
            yield { path: file.path, contents: await this.file(file) };
        }
    }

    /** Upload a file to a package server unless it holds the file. */
    async #offer(
        fetch: (request: Request) => Promise<Response>,
        target: string,
        file: PackageFile,
    ): Promise<void> {
        // ask whether the server holds the file
        const held = await fetch(new Request(target, { method: "HEAD" }));

        // upload a file the server lacks
        if (held.status === 404) {
            const body = await this.file(file);
            const headers = { "content-type": file.mediaType };
            await upload(fetch, new Request(target, { method: "PUT", body, headers }));
        }
        // refuse a failed probe
        else if (!held.ok) {
            throw new ServiceError("BAD_GATEWAY", {
                message: `cannot probe ${new URL(target).pathname}: HTTP ${held.status}`,
            });
        }
    }

    /** Require the store to hold each file a build's manifest lists. */
    async #requireStored(digest: string, files: readonly PackageFile[]): Promise<void> {
        for (const file of files) {
            if (!(await this.#has(`files/${file.digest}`, file))) {
                throw new ServiceError("CONFLICT", {
                    message: `build ${digest} lists ${file.path}, which was not uploaded`,
                });
            }
        }
    }

    /** Collect the digests of the files the retained manifests the store holds list. */
    async #listedFiles(retained: ReadonlySet<string>): Promise<Set<string>> {
        const kept = new Set<string>();
        for (const digest of retained) {
            // skip a retained manifest the store lacks
            if ((await this.bucket.head(`manifests/${Digest.parse(digest)}`)) === null) {
                continue;
            }

            // keep every file the manifest lists
            const { reader } = await this.contents(digest);
            for (const file of await reader.distributed()) {
                kept.add(file.digest);
            }
        }

        return kept;
    }

    /** Read the marks the last sweep left, by key. */
    async #marks(): Promise<Record<string, number>> {
        const object = await this.bucket.get(MARKS_KEY);

        return object === null ? {} : Marks.parse(await object.json());
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
    async *#list(prefix: string): AsyncGenerator<BucketFile> {
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

    /** Delete files in batches the bucket accepts. */
    async #delete(keys: readonly string[]): Promise<void> {
        for (let start = 0; start < keys.length; start += MAX_BATCH_FILES) {
            await this.bucket.delete(keys.slice(start, start + MAX_BATCH_FILES));
        }
    }

    /** Decide whether the bucket has a file's exact bytes under its key, refusing different bytes stored there. */
    async #has(key: string, file: PackageFile): Promise<boolean> {
        const stored = await this.bucket.head(key);
        if (stored === null) {
            return false;
        } else if (!isStored(stored, file)) {
            throw new ServiceError("INTERNAL_SERVER_ERROR", {
                message: `stored object differs: ${key}`,
            });
        }

        return true;
    }

    /** Create an immutable object whose checksum the bucket verifies. */
    async #put(key: string, body: BucketBody, file: PackageFile): Promise<void> {
        const created = await this.bucket.put(key, body, {
            sha256: file.digest,
            httpMetadata: { contentType: file.mediaType },
            onlyIf: { etagDoesNotMatch: "*" },
        });
        const stored = created ?? (await this.bucket.head(key));
        if (!stored || !isStored(stored, file)) {
            throw new ServiceError("INTERNAL_SERVER_ERROR", {
                message: `stored object differs: ${key}`,
            });
        }
    }
}

/** Report whether a bucket object holds a file's exact size and digest. */
function isStored(object: BucketFile, file: PackageFile): boolean {
    return object.size === file.size && object.checksums.toJSON().sha256 === file.digest;
}

/** Keep the first file of each digest. */
function distinctFiles(list: readonly PackageFile[]): PackageFile[] {
    return [...new Map(list.map((file) => [file.digest, file])).values()];
}

/** Run a task per file, a bounded number at a time, checking the signal before each batch. */
async function inBatches(
    files: readonly PackageFile[],
    signal: AbortSignal | undefined,
    task: (file: PackageFile) => Promise<void>,
): Promise<void> {
    for (let start = 0; start < files.length; start += CONCURRENT_UPLOADS) {
        signal?.throwIfAborted();
        await Promise.all(files.slice(start, start + CONCURRENT_UPLOADS).map(task));
    }
}

/** Fetch one file of a build below its served URL, refusing a failed read. */
async function fetchFile(
    fetch: (input: URL, init: RequestInit) => Promise<Response>,
    base: URL,
    path: string,
    manifest: string,
    signal: AbortSignal | undefined,
): Promise<ReadableStream<Uint8Array>> {
    // fetch the file by its encoded path
    const encoded = path.split("/").map(encodeURIComponent).join("/");
    const response = await fetch(new URL(`files/${encoded}`, base), {
        redirect: "error",
        ...(signal === undefined ? {} : { signal }),
    });

    // refuse a failed read
    if (!response.ok || response.body === null) {
        throw new ServiceError("BAD_GATEWAY", {
            message: `cannot read ${path} of build ${manifest}: HTTP ${response.status}`,
        });
    }

    return response.body;
}

/** Refuse a file list naming the reserved manifest path or one path twice. */
function requireDistinctPaths(list: readonly PackageFile[]): void {
    const paths = new Set<string>();
    for (const file of list) {
        if (file.path === "manifest.json" || file.path.startsWith("manifest.json/")) {
            throw new ServiceError("BAD_REQUEST", { message: `reserved build path: ${file.path}` });
        }
        if (paths.has(file.path)) {
            throw new ServiceError("BAD_REQUEST", {
                message: `duplicate package file: ${file.path}`,
            });
        }
        paths.add(file.path);
    }
}

/** Send an upload, refusing a failed one. */
async function upload(
    fetch: (request: Request) => Promise<Response>,
    request: Request,
): Promise<void> {
    const response = await fetch(request);
    if (!response.ok) {
        throw new ServiceError("BAD_GATEWAY", {
            message: `cannot upload ${new URL(request.url).pathname}: HTTP ${response.status}`,
        });
    }
}
