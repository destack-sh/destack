import { BucketChecksums, BucketFile, type BucketHttpMetadata } from "../bucket/index.ts";
import type { file } from "./stack/index.ts";

/** A catalogue row describing the current file at one key. */
export type CatalogueFile = typeof file.$inferSelect;

/** A catalogue row describing the current file at one key. */
export const CatalogueFile = { describe, encodeHttpMetadata };

/** Convert a catalogue row to portable file metadata. */
function describe(entry: CatalogueFile): BucketFile {
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
        entry.ssecKeyMd5 ?? undefined,
        entry.retainUntil === null ? undefined : new Date(entry.retainUntil),
    );
}

/** Encode HTTP metadata for the catalogue, with the expiration as an ISO date. */
function encodeHttpMetadata(metadata: BucketHttpMetadata): CatalogueFile["httpMetadata"] {
    const { cacheExpiry, ...headers } = metadata;

    return cacheExpiry === undefined
        ? headers
        : { ...headers, cacheExpiry: cacheExpiry.toISOString() };
}
