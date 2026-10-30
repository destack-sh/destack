import type { Crypto } from "@destack/environment";
import { Digest } from "@destack/schema";
import { ResourceError } from "../error/error.ts";
import type { BlobEntry, BlobStorage } from "./provider.ts";

/** Blob storage in memory. */
export class MemoryBlobStorage implements BlobStorage {
    /** The kept blobs, by digest. */
    readonly #blobs = new Map<Digest, Uint8Array>();
    /** The crypto digesting the kept blobs. */
    readonly #crypto: Pick<Crypto, "digest">;

    /** Keep blobs in memory, digesting them with some crypto. */
    constructor(crypto: Pick<Crypto, "digest">) {
        this.#crypto = crypto;
    }

    /** Count the kept blobs. */
    get size(): number {
        return this.#blobs.size;
    }

    /** Read a blob's bytes as one chunk. */
    async *read(digest: Digest): AsyncIterable<Uint8Array> {
        const blob = this.#blobs.get(digest);
        if (blob === undefined) {
            throw ResourceError.of("BLOB_NOT_FOUND", `blob ${digest} not found`);
        }
        yield blob;
    }

    /** Read many blobs in order. */
    async *batch(digests: readonly Digest[]): AsyncIterable<BlobEntry> {
        for (const digest of digests) {
            const blob = this.#blobs.get(digest);
            if (blob === undefined) {
                throw ResourceError.of("BLOB_NOT_FOUND", `blob ${digest} not found`);
            }
            yield { digest, size: blob.byteLength, body: this.read(digest) };
        }
    }

    /** Read a range of a blob's bytes as one chunk. */
    async *slice(digest: Digest, offset: number, length: number): AsyncIterable<Uint8Array> {
        for await (const blob of this.read(digest)) {
            yield blob.subarray(offset, offset + length);
        }
    }

    /** List the digests the store lacks, in the given order. */
    async missing(digests: readonly Digest[]): Promise<readonly Digest[]> {
        return digests.filter((digest) => !this.#blobs.has(digest));
    }

    /** Keep bytes under their digest. */
    async write(body: AsyncIterable<Uint8Array>, expected?: Digest): Promise<Digest> {
        // gather and hash the bytes
        const bytes = await new Blob(
            (await Array.fromAsync(body)).map((chunk) => new Uint8Array(chunk)),
        ).bytes();
        const digest = await Digest.of(bytes, this.#crypto);

        // refuse bytes of another digest, and keep them once
        if (expected !== undefined && digest !== expected) {
            throw ResourceError.of("BAD_DIGEST", `expected blob ${expected}, got ${digest}`);
        }
        this.#blobs.set(digest, bytes);

        return digest;
    }

    /** Keep the missing blobs of some digests from another store. */
    async fetch(digests: readonly Digest[], source: Pick<BlobStorage, "batch">): Promise<void> {
        for await (const { digest, body } of source.batch(
            await this.missing([...new Set(digests)]),
        )) {
            await this.write(body, digest);
        }
    }

    /** List the kept blobs' digests. */
    async *digests(): AsyncIterable<Digest> {
        yield* this.#blobs.keys();
    }

    /** Delete a blob, if stored. */
    async delete(digest: Digest): Promise<void> {
        this.#blobs.delete(digest);
    }

    /** Clean up after interrupted writes, which memory never leaves. */
    async clean(): Promise<void> {}
}
