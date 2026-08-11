import type { Blob } from "../_generated/core/blob.js";
import type { BlobInput, BlobRange, BlobStore } from "../blob/blob.js";

/** One exact encoded Destack Program. */
export class Program {
    /** Immutable Program Blob. */
    readonly blob: Blob;
    /** Blob operations for this Program image. */
    readonly blobs: BlobStore;

    /** Bind one Program Blob to its BlobStore. */
    constructor(blobs: BlobStore, blob: Blob) {
        this.blobs = blobs;
        this.blob = blob;
    }

    /** Store exact encoded Program bytes. */
    static async put(blobs: BlobStore, input: BlobInput): Promise<Program> {
        const blob = await blobs.put(input);

        return new Program(blobs, blob);
    }

    /** Stream this Program or one exact byte range. */
    read(range: BlobRange = {}) {
        return this.blobs.read(this.blob, range);
    }

    /** Read this complete Program or one exact byte range. */
    bytes(range: BlobRange = {}): Promise<Uint8Array> {
        return this.blobs.bytes(this.blob, range);
    }
}

export type { Blob, BlobInput, BlobRange };
