import type { BlobStore } from "@destack/db/blob";
import type { Digest } from "@destack/schema";

/** The store of a bucket's blobs, which the catalogue's segments and parts reference by digest. */
export interface ContentStore extends Omit<BlobStore, "hold" | "retire"> {
    /** Read a range of a blob's bytes. */
    slice(digest: Digest, offset: number, length: number): AsyncIterable<Uint8Array>;
    /** Delete a blob. */
    delete(digest: Digest): Promise<void>;
    /** List the kept blobs' digests. */
    digests(): AsyncIterable<Digest>;
    /** Delete the unfinished writes interrupted writers left behind, while no writer runs. */
    clean(): Promise<void>;
}
