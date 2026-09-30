import type { Bucket, BucketBody, BucketRange } from "@destack/bucket";
import { describeFile, Digest, verifyFile, type PackageFile } from "@destack/package/file";
import { PackageManifest, BuildReader, type PackageDistribution } from "@destack/package/manifest";
import { ServiceError } from "@destack/service/error";
import { Tarball } from "@destack/package/archive";
import { comparePath } from "../build/serialization.ts";

/** The uploads one store runs at once: at tens of milliseconds per R2 upload, eight store 2000 files in about 10 seconds. */
const CONCURRENT_UPLOADS = 8;

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
        const inventory = await source.reader.inventory();
        const paths = new Set<string>();
        for (const file of inventory) {
            if (file.path === "manifest.json" || file.path.startsWith("manifest.json/")) {
                throw new ServiceError("BAD_REQUEST", {
                    message: `reserved build path: ${file.path}`,
                });
            }
            if (paths.has(file.path)) {
                throw new ServiceError("BAD_REQUEST", {
                    message: `duplicate package file: ${file.path}`,
                });
            }
            paths.add(file.path);
        }

        // store each distinct file, a bounded number of streams at a time
        const distinct = [...new Map(inventory.map((file) => [file.digest, file])).values()];
        for (let start = 0; start < distinct.length; start += CONCURRENT_UPLOADS) {
            signal?.throwIfAborted();
            await Promise.all(
                distinct
                    .slice(start, start + CONCURRENT_UPLOADS)
                    .map((file) => this.putFile(file, () => source.open(file.path, signal))),
            );
        }

        // store the manifest last
        signal?.throwIfAborted();
        const bytes = new TextEncoder().encode(JSON.stringify(manifest));
        const file = await describeFile("manifest.json", "application/json", bytes);
        await this.#put(`manifests/${file.digest}`, bytes, file);

        return file.digest;
    }

    /** Store a file under its digest unless the bucket has it, opening its bytes only when needed. */
    async putFile(file: PackageFile, open: () => Promise<BucketBody>): Promise<void> {
        const key = `files/${file.digest}`;
        if (!(await this.#holds(key, file))) {
            await this.#put(key, await open(), file);
        }
    }

    /** Read and verify a build's manifest. */
    async get(digest: string): Promise<PackageManifest> {
        // read and verify the manifest bytes
        const object = await this.manifest(digest);
        const bytes = new Uint8Array(await object.arrayBuffer());
        await verifyFile(
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
        let inventory: Promise<PackageFile[]> | undefined;
        const select = async (path: string) => {
            // look the path up among direct references, then in the inventory
            let file = references.get(path);
            if (!file) {
                inventory ??= reader.inventory();
                for (const candidate of await inventory) {
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

    /** Read one complete file and verify its exact bytes. */
    async file(file: PackageFile): Promise<Uint8Array<ArrayBuffer>> {
        // read and verify the file bytes
        const object = await this.open(file);
        const bytes = new Uint8Array(await object.arrayBuffer());
        await verifyFile(file, bytes);

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
        const object = await this.bucket.get(`manifests/${digest}`, { range });
        if (!object) {
            throw new ServiceError("NOT_FOUND", { message: "package manifest not found" });
        }

        return object;
    }

    /** Stream a stored file, refusing one whose size or digest differs. */
    async open(file: PackageFile, range?: BucketRange) {
        // read the object and refuse a size or digest mismatch
        const object = await this.bucket.get(`files/${file.digest}`, { range });
        if (!object) {
            throw new ServiceError("NOT_FOUND", {
                message: `package file not found: ${file.path}`,
            });
        }
        if (object.size !== file.size || object.checksums.toJSON().sha256 !== file.digest) {
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
        const inventory = await contents.reader.inventory();
        for (const file of inventory.toSorted(comparePath)) {
            yield { path: file.path, contents: await this.file(file) };
        }
    }

    /** Decide whether the bucket has a file's exact bytes under its key, refusing different bytes stored there. */
    async #holds(key: string, file: PackageFile): Promise<boolean> {
        const stored = await this.bucket.head(key);
        if (stored === null) {
            return false;
        } else if (stored.size !== file.size || stored.checksums.toJSON().sha256 !== file.digest) {
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
        if (
            !stored ||
            stored.size !== file.size ||
            stored.checksums.toJSON().sha256 !== file.digest
        ) {
            throw new ServiceError("INTERNAL_SERVER_ERROR", {
                message: `stored object differs: ${key}`,
            });
        }
    }
}
