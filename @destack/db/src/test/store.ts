import { Digest } from "@destack/schema";
import type { ContentStore } from "@destack/resource";

/** A snapshot store in memory, keeping each blob's last use, as a cell's store sweeps them. */
export class MemoryContentStore implements ContentStore {
    /** The kept blobs, by digest. */
    readonly #blobs = new Map<Digest, { readonly bytes: Uint8Array; used: Date }>();

    /** Count the kept blobs. */
    get size(): number {
        return this.#blobs.size;
    }

    /** Read a blob's bytes as one chunk. */
    async *read(digest: Digest): AsyncIterable<Uint8Array> {
        const blob = this.#blobs.get(digest);
        if (blob === undefined) {
            throw new TypeError(`no blob ${digest}`);
        }
        yield blob.bytes;
    }

    /** Keep bytes under their digest, marking a kept blob used now. */
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
            throw new TypeError(`blob ${expected} read as ${digest}`);
        }
        this.#blobs.set(digest, { bytes, used: new Date() });

        return digest;
    }

    /** Keep the missing blobs of some digests from another store, marking the kept ones used. */
    async fetch(digests: readonly Digest[], source: Pick<ContentStore, "read">): Promise<void> {
        for (const digest of new Set(digests)) {
            const blob = this.#blobs.get(digest);
            // copy a missing blob
            if (blob === undefined) {
                await this.write(source.read(digest), digest);
            }
            // mark a kept blob used
            else {
                blob.used = new Date();
            }
        }
    }

    /** Delete the blobs outside a retained set last used before a moment. */
    async sweep(retained: ReadonlySet<Digest>, before: Date): Promise<void> {
        for (const [digest, blob] of this.#blobs) {
            if (!retained.has(digest) && blob.used < before) {
                this.#blobs.delete(digest);
            }
        }
    }
}
