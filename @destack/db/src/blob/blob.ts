import type { Digest } from "@destack/schema";

/** The content a database's blob columns reference, kept by the SHA-256 digest of its bytes. */
export interface BlobStore {
    /** List the digests the store lacks, in the given order. */
    missing(digests: readonly Digest[]): Promise<readonly Digest[]>;
    /** Read a blob's bytes. */
    read(digest: Digest): AsyncIterable<Uint8Array>;
    /** Keep bytes under their digest and return it, refusing bytes whose digest differs from the expected one. */
    write(body: AsyncIterable<Uint8Array>, expected?: Digest): Promise<Digest>;
    /** Keep the blobs the store lacks among some digests, read from another store. */
    fetch(digests: readonly Digest[], source: Pick<BlobStore, "read">): Promise<void>;
}
