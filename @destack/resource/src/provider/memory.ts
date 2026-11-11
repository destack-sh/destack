import type { Crypto } from "@destack/environment";
import { Digest } from "@destack/schema";
import { ResourceError } from "../error/error.ts";
import { BlobPack } from "./pack.ts";
import type { BlobBody, BlobEntry, BlobPlacement, BlobStorage, ContentStore } from "./provider.ts";

/** Blob storage in memory, a pack per write. */
export class MemoryBlobStorage implements BlobStorage {
    /** The stored packs, by key. */
    readonly #packs = new Map<string, Uint8Array>();
    /** The crypto digesting the stored blobs. */
    readonly #crypto: Pick<Crypto, "digest">;
    /** The packs of fresh name stored so far, naming the next. */
    #named = 0;

    /** Store blobs in memory, digesting them with some crypto. */
    constructor(crypto: Pick<Crypto, "digest">) {
        this.#crypto = crypto;
    }

    /** Count the stored packs. */
    get size(): number {
        return this.#packs.size;
    }

    /** Store the bodies in one pack. */
    async write(bodies: readonly BlobBody[]): Promise<readonly BlobPlacement[]> {
        // read, digest and verify each body's bytes
        const blobs: { readonly digest: Digest; readonly size: number }[] = [];
        const chunks: Uint8Array[] = [];
        for (const body of bodies) {
            const bytes = joined(await Array.fromAsync(body.body));
            const digest = await Digest.of(bytes, this.#crypto);
            BlobPack.verify(body, digest, bytes.byteLength);
            blobs.push({ digest, size: bytes.byteLength });
            chunks.push(bytes);
        }

        // store the pack under its key
        const pack = BlobPack.key(bodies, () => String((this.#named += 1)));
        this.#packs.set(pack, joined(chunks));

        return BlobPack.place(pack, blobs);
    }

    /** Read a range of a pack's bytes as one chunk. */
    async *slice(pack: string, offset: number, length: number): AsyncIterable<Uint8Array> {
        const bytes = this.#packs.get(pack);
        if (bytes === undefined) {
            throw ResourceError.of("BLOB_NOT_FOUND", `pack '${pack}' not found`);
        }
        yield bytes.subarray(offset, offset + length);
    }

    /** List the stored packs. */
    async *packs(): AsyncIterable<string> {
        yield* this.#packs.keys();
    }

    /** Delete a pack, if stored. */
    async delete(pack: string): Promise<void> {
        this.#packs.delete(pack);
    }

    /** Clean up after interrupted writes, which memory never leaves. */
    async clean(): Promise<void> {}
}

/** Blobs by digest in memory, for copies such as snapshots that need no index. */
export class MemoryContentStore implements ContentStore {
    /** The stored blobs, by digest. */
    readonly #blobs = new Map<Digest, Uint8Array>();
    /** The crypto digesting the stored blobs. */
    readonly #crypto: Pick<Crypto, "digest">;

    /** Store blobs in memory, digesting them with some crypto. */
    constructor(crypto: Pick<Crypto, "digest">) {
        this.#crypto = crypto;
    }

    /** Count the stored blobs. */
    get size(): number {
        return this.#blobs.size;
    }

    /** Read a blob's bytes as one chunk. */
    async *read(digest: Digest): AsyncIterable<Uint8Array> {
        const blob = this.#blobs.get(digest);
        if (blob === undefined) {
            throw ResourceError.of("BLOB_NOT_FOUND", `blob '${digest}' not found`);
        }
        yield blob;
    }

    /** Read many blobs in order. */
    async *batch(digests: readonly Digest[]): AsyncIterable<BlobEntry> {
        for (const digest of digests) {
            const blob = this.#blobs.get(digest);
            if (blob === undefined) {
                throw ResourceError.of("BLOB_NOT_FOUND", `blob '${digest}' not found`);
            }
            yield { digest, size: blob.byteLength, body: this.read(digest) };
        }
    }

    /** Store bytes under their digest, refusing another than an expected one. */
    async write(body: AsyncIterable<Uint8Array>, expected?: Digest): Promise<Digest> {
        // read and digest the bytes
        const bytes = joined(await Array.fromAsync(body));
        const digest = await Digest.of(bytes, this.#crypto);
        if (expected !== undefined && digest !== expected) {
            throw ResourceError.of(
                "DIGEST_INVALID",
                `blob '${digest}' is invalid: its digest must be '${expected}'`,
            );
        }
        this.#blobs.set(digest, bytes);

        return digest;
    }

    /** Store the missing blobs of some digests from another store. */
    async fetch(digests: readonly Digest[], source: Pick<ContentStore, "batch">): Promise<void> {
        for await (const { digest, body } of source.batch(
            await this.missing([...new Set(digests)]),
        )) {
            await this.write(body, digest);
        }
    }

    /** List the digests the store lacks, in the given order. */
    async missing(digests: readonly Digest[]): Promise<readonly Digest[]> {
        return digests.filter((digest) => !this.#blobs.has(digest));
    }
}

/** Join chunks into one array. */
function joined(chunks: readonly Uint8Array[]): Uint8Array<ArrayBuffer> {
    // copy each chunk after the ones before it
    const bytes = new Uint8Array(chunks.reduce((sum, chunk) => sum + chunk.byteLength, 0));
    let offset = 0;
    for (const chunk of chunks) {
        bytes.set(chunk, offset);
        offset += chunk.byteLength;
    }

    return bytes;
}
