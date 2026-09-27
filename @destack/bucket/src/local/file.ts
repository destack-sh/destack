import { BucketChecksums, BucketFile, type BucketHttpMetadata } from "../bucket/index.ts";
import { StorageError } from "../error/index.ts";
import type { file } from "./stack/index.ts";

/** A catalogue row describing the current file at one key. */
export type LocalFile = typeof file.$inferSelect;

/** A catalogue row describing the current file at one key. */
export const LocalFile = { describe, encodeHttpMetadata, rejectCustomerKey };

/** Convert a catalogue row to portable file metadata. */
function describe(entry: LocalFile): BucketFile {
    const { cacheExpiry, ...headers } = entry.httpMetadata;
    const httpMetadata =
        cacheExpiry === undefined ? headers : { ...headers, cacheExpiry: new Date(cacheExpiry) };

    return new BucketFile(
        entry.key,
        entry.version,
        entry.size,
        entry.etag,
        new Date(entry.uploaded),
        httpMetadata,
        entry.customMetadata,
        new BucketChecksums(entry.checksums),
        entry.storageClass,
    );
}

/** Encode HTTP metadata for the catalogue, with the expiration as an ISO date. */
function encodeHttpMetadata(metadata: BucketHttpMetadata): LocalFile["httpMetadata"] {
    const { cacheExpiry, ...headers } = metadata;

    return cacheExpiry === undefined
        ? headers
        : { ...headers, cacheExpiry: cacheExpiry.toISOString() };
}

/** Reject a customer encryption key, since local content files and their digests stay readable on disk. */
function rejectCustomerKey(ssecKey: ArrayBuffer | string | undefined): void {
    if (ssecKey !== undefined) {
        throw new StorageError(
            "UNSUPPORTED",
            "local buckets do not support customer-provided encryption keys",
        );
    }
}
