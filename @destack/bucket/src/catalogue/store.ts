import type { ContentStore } from "@destack/resource";
import type { Digest } from "@destack/schema";

/** The content-addressed store of a bucket's blobs. */
export interface BlobStore extends ContentStore {
    /** List the digests the store lacks, in the given order. */
    readonly missing: (digests: readonly Digest[]) => Promise<readonly Digest[]>;
    /** Read a range of a blob's bytes. */
    readonly slice: (digest: Digest, offset: number, length: number) => AsyncIterable<Uint8Array>;
    /** Delete the unfinished writes interrupted writers left behind, while no writer runs. */
    readonly clean: () => Promise<void>;
}
