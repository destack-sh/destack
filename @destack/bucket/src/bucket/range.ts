import { StorageError } from "../error/index.ts";

/** An inclusive starting byte and optional byte count, or a trailing byte count. */
export type BucketRange =
    | { offset: number; length?: number }
    | { offset?: number; length: number }
    | { suffix: number };

/** An inclusive starting byte and optional byte count, or a trailing byte count. */
export const BucketRange = { resolve, check };

/** Resolve a requested range against the stored file length. */
function resolve(size: number, range: BucketRange): { offset: number; length: number } {
    // resolve the start and length, then require them within the file
    const offset = "suffix" in range ? Math.max(0, size - range.suffix) : (range.offset ?? 0);
    const requested = "suffix" in range ? range.suffix : (range.length ?? size - offset);
    if (
        !Number.isSafeInteger(offset) ||
        offset < 0 ||
        offset >= size ||
        !Number.isSafeInteger(requested) ||
        requested <= 0
    ) {
        throw new StorageError("INVALID_RANGE", "the requested file range is not satisfiable");
    }

    return { offset, length: Math.min(requested, size - offset) };
}

/** Check range arguments before asking a backend to read a file. */
function check(range: BucketRange): void {
    const valid =
        "suffix" in range
            ? Number.isSafeInteger(range.suffix) && range.suffix > 0
            : (range.offset === undefined ||
                  (Number.isSafeInteger(range.offset) && range.offset >= 0)) &&
              (range.length === undefined ||
                  (Number.isSafeInteger(range.length) && range.length > 0));
    if (!valid) {
        throw new StorageError("INVALID_RANGE", "the requested file range is not satisfiable");
    }
}
