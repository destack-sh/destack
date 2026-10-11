import type { R2Object } from "@cloudflare/workers-types";
import {
    type BucketEntry,
    BucketFile,
    ContentDigest,
    DIGEST_ALGORITHMS,
    type DigestAlgorithm,
    StorageClass,
} from "../bucket/index.ts";

/** The storage class of files stored without one, which R2 reports and its local simulator leaves empty. */
const DEFAULT_STORAGE_CLASS = "Standard";

/** File metadata an R2 binding returns. */
export const R2File = { describe, list };

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
        new ContentDigest(digests(entry)),
        storageClass(entry),
        entry.ssecKeyMd5,
    );
}

/** Convert R2 file metadata to a listing entry. */
function list(entry: R2Object): BucketEntry {
    return {
        key: entry.key,
        size: entry.size,
        etag: entry.etag,
        uploaded: entry.uploaded,
        storageClass: storageClass(entry),
    };
}

/** Read the stored digests of the kept algorithms. */
function digests(entry: R2Object): Partial<Record<DigestAlgorithm, string>> {
    // keep the hexadecimal digest of each kept algorithm R2 stores
    const checksums = entry.checksums.toJSON();
    const kept: Partial<Record<DigestAlgorithm, string>> = {};
    for (const algorithm of DIGEST_ALGORITHMS) {
        const value = checksums[algorithm];
        if (value !== undefined) {
            kept[algorithm] = value;
        }
    }

    return kept;
}

/** Read the storage class, which R2's local simulator leaves empty for the default. */
function storageClass(entry: R2Object): StorageClass {
    return StorageClass.read(
        entry.storageClass === "" ? DEFAULT_STORAGE_CLASS : entry.storageClass,
    );
}
