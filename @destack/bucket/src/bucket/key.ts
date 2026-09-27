import { StorageError } from "../error/index.ts";

/** The longest file key in UTF-8 bytes, the S3 and R2 limit. */
const MAX_KEY_BYTES = 1024;
/** The highest Unicode code point. */
const MAX_CODE_POINT = 0x10ffff;
/** The last code point before the surrogate range, which UTF-8 strings skip. */
const LAST_BEFORE_SURROGATES = 0xd7ff;
/** The first code point after the surrogate range. */
const FIRST_AFTER_SURROGATES = 0xe000;

/** A file key, up to 1024 UTF-8 bytes. */
export const BucketKey = { check, prefixEnd };

/** Check the portable UTF-8 file key limit. */
function check(key: string): void {
    const size = new TextEncoder().encode(key).byteLength;
    if (size === 0 || size > MAX_KEY_BYTES || key.includes("\0") || !key.isWellFormed()) {
        throw new StorageError("INVALID_KEY", "file keys require 1–1024 UTF-8 bytes without NUL");
    }
}

/** Find the smallest UTF-8 string above every key with this prefix, or undefined when none exists. */
function prefixEnd(prefix: string): string | undefined {
    const characters = Array.from(prefix);
    for (let index = characters.length - 1; index >= 0; index--) {
        // carry past the highest code point
        const codepoint = characters[index]!.codePointAt(0)!;
        if (codepoint === MAX_CODE_POINT) {
            continue;
        }

        // increment the last code point that can grow, skipping surrogates
        const next = codepoint === LAST_BEFORE_SURROGATES ? FIRST_AFTER_SURROGATES : codepoint + 1;

        return characters.slice(0, index).join("") + String.fromCodePoint(next);
    }

    return undefined;
}
