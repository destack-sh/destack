import { Digest } from "@destack/schema";
import type { ContentStore } from "@destack/resource";

/** A snapshot store in memory, keeping each part with the moment it was last used, as a cell's store sweeps them. */
export class MemoryContentStore implements ContentStore {
    /** The kept parts, by digest. */
    readonly #parts = new Map<Digest, { readonly bytes: Uint8Array; used: Date }>();

    /** Count the kept parts. */
    get size(): number {
        return this.#parts.size;
    }

    /** Read a part's bytes as one chunk. */
    async *read(digest: Digest): AsyncIterable<Uint8Array> {
        const part = this.#parts.get(digest);
        if (part === undefined) {
            throw new TypeError(`no part ${digest}`);
        }
        yield part.bytes;
    }

    /** Keep bytes under their digest, marking a kept part used now. */
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
            throw new TypeError(`part ${expected} read as ${digest}`);
        }
        this.#parts.set(digest, { bytes, used: new Date() });

        return digest;
    }

    /** Keep the parts the store lacks among some digests, read from another store, marking the kept ones used now. */
    async fetch(digests: readonly Digest[], source: Pick<ContentStore, "read">): Promise<void> {
        for (const digest of new Set(digests)) {
            const part = this.#parts.get(digest);
            // copy a missing part
            if (part === undefined) {
                await this.write(source.read(digest), digest);
            }
            // mark a kept part used
            else {
                part.used = new Date();
            }
        }
    }

    /** Delete the parts outside a retained set last used before a moment. */
    async sweep(retained: ReadonlySet<Digest>, before: Date): Promise<void> {
        for (const [digest, part] of this.#parts) {
            if (!retained.has(digest) && part.used < before) {
                this.#parts.delete(digest);
            }
        }
    }
}
