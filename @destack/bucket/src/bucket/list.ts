import type { BucketFile } from "./file.ts";
import { StorageError } from "../error/index.ts";

/** The most files one listing page or one delete names, the S3 and R2 limit. */
export const MAX_BATCH_FILES = 1000;

/** A lexicographically ordered page of files. */
export type BucketListing = {
    /** The listed metadata. */
    files: BucketFile[];
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

/** Check a listing page size. */
function checkLimit(limit: number): void {
    if (!Number.isInteger(limit) || limit < 1 || limit > MAX_BATCH_FILES) {
        throw new StorageError(
            "INVALID_LIMIT",
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
        const cursor = JSON.parse(
            new TextDecoder("utf-8", { fatal: true }).decode(Uint8Array.fromBase64(value)),
        );
        if (
            cursor.backend !== backend ||
            cursor.selection !== selection ||
            typeof cursor.continuation !== "string"
        ) {
            throw new StorageError("INVALID_CURSOR", "invalid file listing cursor");
        }

        return cursor.continuation;
    } catch (cause) {
        if (cause instanceof StorageError) {
            throw cause;
        }
        throw new StorageError("INVALID_CURSOR", "invalid file listing cursor", { cause });
    }
}
