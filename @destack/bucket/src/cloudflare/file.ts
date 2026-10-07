import type { R2Object } from "@cloudflare/workers-types";
import { BucketChecksums, BucketFile, StorageClass } from "../bucket/index.ts";

/** The storage class of files stored without one, which R2 reports and its local simulator leaves empty. */
const DEFAULT_STORAGE_CLASS = "Standard";

/** File metadata an R2 binding returns. */
export const R2File = { describe };

/** Convert R2 file metadata to Destack file metadata. */
function describe(entry: R2Object): BucketFile {
    return new BucketFile(
        entry.key,
        entry.version,
        entry.size,
        entry.etag,
        entry.uploaded,
        entry.httpMetadata,
        entry.customMetadata,
        new BucketChecksums(entry.checksums.toJSON()),
        StorageClass.read(entry.storageClass === "" ? DEFAULT_STORAGE_CLASS : entry.storageClass),
        entry.ssecKeyMd5,
    );
}
