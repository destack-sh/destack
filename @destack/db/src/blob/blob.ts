import type { Digest } from "@destack/schema";

/**
 * The content a database's blob columns reference, kept by the SHA-256 digest of its bytes.
 *
 * Its owner deletes the blobs no row references, so a writer holds it while it writes blobs and retires the blobs of the rows it deletes.
 */
export interface BlobStore {
    /** List the digests the store lacks, in the given order. */
    missing(digests: readonly Digest[]): Promise<readonly Digest[]>;
    /** Read a blob's bytes. */
    read(digest: Digest): AsyncIterable<Uint8Array>;
    /** Keep bytes under their digest and return it, refusing bytes whose digest differs from the expected one. */
    write(body: AsyncIterable<Uint8Array>, expected?: Digest): Promise<Digest>;
    /** Keep the blobs the store lacks among some digests, read from another store. */
    fetch(digests: readonly Digest[], source: Pick<BlobStore, "read">): Promise<void>;
    /** Keep every blob from deletion until the hold is released, since rows reference written blobs only later. */
    hold(): Promise<AsyncDisposable>;
    /** Release the blobs deleted rows referenced, deleting those no row references any longer. */
    retire(digests: readonly Digest[]): Promise<void>;
}
