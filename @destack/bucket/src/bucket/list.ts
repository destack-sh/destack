import { schema } from "@destack/schema";
import type { BucketFile, StorageClass } from "./file.ts";
import { BucketError } from "../error/error.ts";

/** The most files one listing page or one delete takes, the S3 and R2 limit. */
export const MAX_BATCH_FILES = 1000;

/** A file-list continuation with the backend and selection it continues. */
const Cursor = schema.object({
    backend: schema.string(),
    selection: schema.string(),
    continuation: schema.string(),
});

/** A listed file: the metadata a listing has of it, as S3's ListObjectsV2 lists it. */
export interface BucketEntry {
    /** The exact file key. */
    readonly key: string;
    /** The complete file length in bytes. */
    readonly size: number;
    /** The unquoted entity tag. */
    readonly etag: string;
    /** The upload time. */
    readonly uploaded: Date;
    /** The storage class. */
    readonly storageClass: StorageClass;
}

/** A listed file: the metadata a listing has of it, as S3's ListObjectsV2 lists it. */
export const BucketEntry = { of };

/** A lexicographically ordered page of files. */
export type BucketListing = {
    /** The listed files. */
    files: BucketEntry[];
    /** Common key prefixes omitted from the file list. */
    delimitedPrefixes: string[];
} & (
    | {
          /** More entries remain. */
          truncated: true;
          /** The next page. */
          cursor: string;
      }
    | {
          /** The listing is complete. */
          truncated: false;
          /** No further page exists. */
          cursor?: undefined;
      }
);

/** A lexicographically ordered page of files. */
export const BucketListing = { checkLimit, encodeCursor, decodeCursor };

/** Describe a file as a listing lists it. */
function of(file: Pick<BucketFile, keyof BucketEntry>): BucketEntry {
    return {
        key: file.key,
        size: file.size,
        etag: file.etag,
        uploaded: file.uploaded,
        storageClass: file.storageClass,
    };
}

/** Check a listing page size. */
function checkLimit(limit: number): void {
    if (!Number.isInteger(limit) || limit < 1 || limit > MAX_BATCH_FILES) {
        throw BucketError.of(
            "INVALID_ARGUMENT",
            "file listing limits must be integers from 1 through 1000",
        );
    }
}

/** Encode a file-list continuation with its backend and selection. */
function encodeCursor(backend: string, selection: string, continuation: string): string {
    const bytes = new TextEncoder().encode(JSON.stringify({ backend, selection, continuation }));

    return bytes.toBase64();
}

/** Read a continuation for the same backend and selection. */
function decodeCursor(value: string, backend: string, selection: string): string {
    try {
        const cursor = Cursor.parse(
            JSON.parse(
                new TextDecoder("utf-8", { fatal: true }).decode(Uint8Array.fromBase64(value)),
            ),
        );
        if (cursor.backend !== backend || cursor.selection !== selection) {
            throw BucketError.of("INVALID_ARGUMENT", "invalid file listing cursor");
        }

        return cursor.continuation;
    } catch (cause) {
        if (BucketError.is(cause)) {
            throw cause;
        }
        throw BucketError.of("INVALID_ARGUMENT", "invalid file listing cursor", { cause });
    }
}
