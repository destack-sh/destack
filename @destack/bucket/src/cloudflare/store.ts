import { createHash, type Hash } from "node:crypto";
import type { ContentStore } from "@destack/resource";
import { Digest, present } from "@destack/schema";
import type { Bucket, UploadedPart } from "../bucket/index.ts";
import type { BlobStore } from "../catalogue/index.ts";
import { MAX_BATCH_FILES } from "../bucket/list.ts";
import { BucketError } from "../error/index.ts";

/** The bytes of each part of a blob written in parts: 8 MiB, above R2's 5 MiB minimum, since R2 takes parts of one size. */
const PART_BYTES = 8 * 1024 * 1024;

/** The directory below a bucket's prefix keeping the bodies of writes whose digest is not known yet. */
const STAGING = "staging/";

/**
 * A bucket's blobs as files of a residency's R2 bucket below the bucket's prefix, each named by its digest.
 *
 * A body longer than one part is staged under a random name, since R2 renames no file, and written under its digest once hashed.
 */
export class R2BlobStore implements BlobStore {
    /** The residency's bucket. */
    readonly #files: Bucket;
    /** The prefix of the bucket's blobs, such as `bucket-…/`. */
    readonly #prefix: string;

    /** Keep a bucket's blobs below a prefix of the residency's bucket. */
    constructor(files: Bucket, prefix: string) {
        this.#files = files;
        this.#prefix = prefix;
    }

    /** List the digests the store lacks, in the given order. */
    async missing(digests: readonly Digest[]): Promise<readonly Digest[]> {
        const kept = await Promise.all(
            digests.map((digest) => this.#files.head(this.#key(digest))),
        );

        return digests.filter((_, index) => kept[index] === null);
    }

    /** Read a blob's bytes. */
    async *read(digest: Digest): AsyncIterable<Uint8Array> {
        const file = await this.#files.get(this.#key(digest));
        yield* present(file, digest).body;
    }

    /** Read a range of a blob's bytes. */
    async *slice(digest: Digest, offset: number, length: number): AsyncIterable<Uint8Array> {
        const file = await this.#files.get(this.#key(digest), { range: { offset, length } });
        yield* present(file, digest).body;
    }

    /** Keep bytes under their digest and return it, refusing bytes whose digest differs from the expected one. */
    async write(body: AsyncIterable<Uint8Array>, expected?: Digest): Promise<Digest> {
        // read up to two parts to tell a body of one part from a longer one
        const hash = createHash("sha256");
        const parts = partsOf(body, hash);
        const first = await parts.next();
        const second = await parts.next();

        // keep a body of one part under its digest at once
        if (first.done === true || second.done === true) {
            const bytes = first.done === true ? new Uint8Array() : first.value;
            const digest = verified(hash, expected);
            if ((await this.missing([digest])).length > 0) {
                await this.#files.put(this.#key(digest), bytes, { sha256: digest });
            } else {
                await this.#touch(digest);
            }

            return digest;
        }

        // stage a longer body in parts, then keep it under its digest once
        const staged = `${this.#prefix}${STAGING}${crypto.randomUUID()}`;
        try {
            await this.#stage(staged, resumed([first.value, second.value], parts));
            const digest = verified(hash, expected);
            if ((await this.missing([digest])).length > 0) {
                const file = present(await this.#files.get(staged), staged);
                await this.#files.put(this.#key(digest), file.body, { sha256: digest });
            } else {
                await this.#touch(digest);
            }

            return digest;
        } finally {
            await this.#files.delete(staged);
        }
    }

    /** Keep the blobs the store lacks among some digests, read from another store, and mark the kept ones used now. */
    async fetch(digests: readonly Digest[], source: Pick<ContentStore, "read">): Promise<void> {
        const unique = [...new Set(digests)];
        const missing = new Set(await this.missing(unique));
        for (const digest of unique) {
            // copy a missing blob
            if (missing.has(digest)) {
                await this.write(source.read(digest), digest);
            }
            // mark a kept blob used
            else {
                await this.#touch(digest);
            }
        }
    }

    /** Delete a blob. */
    async delete(digest: Digest): Promise<void> {
        await this.#files.delete(this.#key(digest));
    }

    /** List the kept blobs' digests, directly below the prefix. */
    async *digests(): AsyncIterable<Digest> {
        for await (const keys of this.#pages(this.#prefix, "/")) {
            for (const key of keys) {
                const name = key.slice(this.#prefix.length);
                if (Digest.safeParse(name).success) {
                    yield name;
                }
            }
        }
    }

    /** Delete the blobs outside a retained set last used before a moment. */
    async sweep(retained: ReadonlySet<Digest>, before: Date): Promise<void> {
        for await (const digest of this.digests()) {
            // leave retained blobs and blobs used since the moment
            if (retained.has(digest)) {
                continue;
            }
            const file = await this.#files.head(this.#key(digest));
            if (file !== null && file.uploaded < before) {
                await this.delete(digest);
            }
        }
    }

    /** Delete the staged bodies a stopped host left behind, a page at a time. */
    async clean(): Promise<void> {
        for await (const keys of this.#pages(`${this.#prefix}${STAGING}`)) {
            await this.#files.delete(keys);
        }
    }

    /** Delete every file below the prefix, such as when the bucket is destroyed. */
    async clear(): Promise<void> {
        for await (const keys of this.#pages(this.#prefix)) {
            await this.#files.delete(keys);
        }
    }

    /** Mark a kept blob used now by writing it onto itself, since R2 keeps a file's upload time as its only time. */
    async #touch(digest: Digest): Promise<void> {
        const key = this.#key(digest);
        const file = present(await this.#files.get(key), digest);
        await this.#files.put(key, file.body, { sha256: digest });
    }

    /** Upload a body in parts of one size under a staging name, skipping an empty last part. */
    async #stage(staged: string, parts: AsyncIterable<Uint8Array<ArrayBuffer>>): Promise<void> {
        const upload = await this.#files.createMultipartUpload(staged);
        try {
            const uploaded: UploadedPart[] = [];
            for await (const part of parts) {
                if (part.byteLength > 0) {
                    uploaded.push(await upload.uploadPart(uploaded.length + 1, part));
                }
            }
            await upload.complete(uploaded);
        } catch (error) {
            // abort the upload, keeping the failure beside a failure to abort
            try {
                await upload.abort();
            } catch (cleanup) {
                throw new AggregateError([error, cleanup], "blob upload and abort failed", {
                    cause: cleanup,
                });
            }
            throw error;
        }
    }

    /** List the keys below a prefix a page at a time, without those below the delimiter when given. */
    async *#pages(prefix: string, delimiter?: string): AsyncIterable<string[]> {
        let cursor: string | undefined;
        do {
            const page = await this.#files.list({
                prefix,
                limit: MAX_BATCH_FILES,
                ...(delimiter === undefined ? {} : { delimiter }),
                ...(cursor === undefined ? {} : { cursor }),
            });
            if (page.files.length > 0) {
                yield page.files.map((file) => file.key);
            }
            cursor = page.cursor;
        } while (cursor !== undefined);
    }

    /** Name a blob's file. */
    #key(digest: Digest): string {
        if (!Digest.safeParse(digest).success) {
            throw new BucketError("INVALID_KEY", `not a blob digest: ${digest}`);
        }

        return `${this.#prefix}${digest}`;
    }
}

/** Split a body into parts of one size and a last shorter or empty one, hashing it on the way. */
async function* partsOf(
    body: AsyncIterable<Uint8Array>,
    hash: Hash,
): AsyncGenerator<Uint8Array<ArrayBuffer>, void> {
    // buffer chunks until they fill a part
    let buffered: Uint8Array[] = [];
    let size = 0;
    for await (const chunk of body) {
        // hash the chunk, and pass on each full part
        hash.update(chunk);
        let rest = chunk;
        while (size + rest.byteLength >= PART_BYTES) {
            const taken = PART_BYTES - size;
            buffered.push(rest.subarray(0, taken));
            yield joined(buffered, PART_BYTES);
            buffered = [];
            size = 0;
            rest = rest.subarray(taken);
        }

        // keep the rest for the next part
        if (rest.byteLength > 0) {
            buffered.push(rest);
            size += rest.byteLength;
        }
    }
    yield joined(buffered, size);
}

/** Pass on the parts read so far, then the rest. */
async function* resumed(
    read: readonly Uint8Array<ArrayBuffer>[],
    rest: AsyncGenerator<Uint8Array<ArrayBuffer>, void>,
): AsyncIterable<Uint8Array<ArrayBuffer>> {
    yield* read;
    yield* rest;
}

/** Join chunks into one array of their total size. */
function joined(chunks: readonly Uint8Array[], size: number): Uint8Array<ArrayBuffer> {
    // copy each chunk after the one before
    const bytes = new Uint8Array(size);
    let offset = 0;
    for (const chunk of chunks) {
        bytes.set(chunk, offset);
        offset += chunk.byteLength;
    }

    return bytes;
}

/** Finish a body's digest, refusing one that differs from the expected digest. */
function verified(hash: Hash, expected: Digest | undefined): Digest {
    const digest = hash.digest("hex");
    if (expected !== undefined && digest !== expected) {
        throw new BucketError("INVALID_CHECKSUM", `blob ${expected} read as ${digest}`);
    }

    return digest;
}
