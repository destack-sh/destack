import { Digest } from "@destack/schema";
import type { ContentStore } from "../provider/provider.ts";

/** A snapshot store in memory. */
export class MemoryContentStore implements ContentStore {
    /** The kept blobs, by digest. */
    readonly #blobs = new Map<Digest, Uint8Array>();

    /** Count the kept blobs. */
    get size(): number {
        return this.#blobs.size;
    }

    /** Read a blob's bytes as one chunk. */
    async *read(digest: Digest): AsyncIterable<Uint8Array> {
        const blob = this.#blobs.get(digest);
        if (blob === undefined) {
            throw new TypeError(`blob ${digest} not found`);
        }
        yield blob;
    }

    /** Keep bytes under their digest. */
    async write(body: AsyncIterable<Uint8Array>, expected?: Digest): Promise<Digest> {
        // gather and hash the bytes
        const bytes = await new Blob(
            (await Array.fromAsync(body)).map((chunk) => new Uint8Array(chunk)),
        ).bytes();
        const digest = Digest.parse(
            new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)).toHex(),
        );

        // refuse bytes of another digest, and keep them once
        if (expected !== undefined && digest !== expected) {
            throw new TypeError(`expected blob ${expected}, got ${digest}`);
        }
        this.#blobs.set(digest, bytes);

        return digest;
    }

    /** Keep the missing blobs of some digests from another store. */
    async fetch(digests: readonly Digest[], source: Pick<ContentStore, "read">): Promise<void> {
        for (const digest of new Set(digests)) {
            // copy a missing blob
            if (!this.#blobs.has(digest)) {
                await this.write(source.read(digest), digest);
            }
        }
    }

    /** List the kept blobs' digests. */
    async *digests(): AsyncIterable<Digest> {
        yield* this.#blobs.keys();
    }

    /** Delete a blob. */
    async delete(digest: Digest): Promise<void> {
        this.#blobs.delete(digest);
    }
}
