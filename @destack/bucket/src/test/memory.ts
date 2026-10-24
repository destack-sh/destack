import { createHash } from "node:crypto";
import {
    type Bucket,
    type BucketBody,
    BucketCondition,
    BucketEntry,
    BucketFile,
    BucketFileBody,
    type BucketGetOptions,
    BucketKey,
    BucketListing,
    type BucketListOptions,
    type BucketPutOptions,
    BucketRange,
    ContentDigest,
    type DigestAlgorithm,
    DIGEST_ALGORITHMS,
    MAX_BATCH_FILES,
    type MultipartUpload,
} from "../bucket/index.ts";
import { BucketError, preconditionFailed } from "../error/index.ts";
import type { Clock } from "@destack/environment";

/** A file a memory bucket keeps: its metadata and bytes. */
interface MemoryFile {
    /** The metadata. */
    readonly file: BucketFile;
    /** The bytes. */
    readonly bytes: Uint8Array<ArrayBuffer>;
}

/** File storage in memory, as tests bind it. */
export class MemoryBucket implements Bucket {
    /** The files by key. */
    readonly #files = new Map<string, MemoryFile>();
    /** The clock object locks lapse by. */
    readonly #clock: Clock;
    /** The version the next write takes, counted per bucket. */
    #version = 1;

    /** Keep files on a clock. */
    constructor(clock: Clock) {
        this.#clock = clock;
    }

    /** Read a file's metadata, null when absent. */
    async head(key: string): Promise<BucketFile | null> {
        BucketKey.check(key);

        return this.#files.get(key)?.file ?? null;
    }

    /** Read a file, null when absent, refusing a failed precondition. */
    async get(key: string, options: BucketGetOptions = {}): Promise<BucketFileBody | null> {
        // find the file, refusing a failed precondition
        BucketKey.check(key);
        const kept = this.#files.get(key);
        if (kept === undefined) {
            return null;
        } else if (!BucketCondition.matches(kept.file, options.onlyIf)) {
            throw preconditionFailed(kept.file);
        }

        // answer the requested bytes
        const range =
            options.range === undefined
                ? undefined
                : BucketRange.resolve(kept.file.size, options.range);
        const bytes =
            range === undefined
                ? kept.bytes
                : kept.bytes.slice(range.offset, range.offset + range.length);

        return new BucketFileBody(
            kept.file,
            new Response(bytes).body ?? new ReadableStream(),
            range,
        );
    }

    /** Store a file, refusing a failed precondition or a lock keeping the current one. */
    async put(
        key: string,
        body: BucketBody | null,
        options: BucketPutOptions = {},
    ): Promise<BucketFile> {
        // refuse a failed precondition and a locked file
        BucketKey.check(key);
        const current = this.#files.get(key)?.file ?? null;
        if (!BucketCondition.matches(current, options.onlyIf)) {
            throw preconditionFailed(current);
        }
        this.#requireUnlocked(current);

        // keep the bytes the body spans with their tag and metadata
        const content = ArrayBuffer.isView(body)
            ? new Uint8Array(body.buffer, body.byteOffset, body.byteLength)
            : body;
        const bytes = new Uint8Array(await new Response(content).arrayBuffer());
        const etag = createHash("md5").update(bytes).digest("hex");

        // verify and record each checksum the caller supplies, as stored objects keep them
        const digests: Partial<Record<DigestAlgorithm, string>> = { md5: etag };
        for (const algorithm of DIGEST_ALGORITHMS) {
            const supplied = options[algorithm];
            if (supplied !== undefined) {
                const actual = createHash(algorithm).update(bytes).digest("hex");
                if (ContentDigest.hex(supplied) !== actual) {
                    throw BucketError.of(
                        "BAD_DIGEST",
                        "object checksum does not match its contents",
                    );
                }
                digests[algorithm] = actual;
            }
        }
        const file = new BucketFile(
            key,
            String(this.#version++),
            bytes.byteLength,
            etag,
            new Date(this.#clock.now()),
            options.httpMetadata,
            options.customMetadata,
            new ContentDigest(digests),
            options.storageClass,
            undefined,
            options.retainUntil,
        );
        this.#files.set(key, { file, bytes });

        return file;
    }

    /** Delete files, refusing a locked one, leaving absent keys absent. */
    async delete(key: string | string[]): Promise<void> {
        // check every key and lock before deleting any
        const keys = typeof key === "string" ? [key] : key;
        if (keys.length > MAX_BATCH_FILES) {
            throw BucketError.of("INVALID_ARGUMENT", "a delete accepts at most 1000 files");
        }
        for (const each of keys) {
            BucketKey.check(each);
            this.#requireUnlocked(this.#files.get(each)?.file ?? null);
        }

        // delete them
        for (const each of keys) {
            this.#files.delete(each);
        }
    }

    /** List a page of files in key order under a prefix, grouping keys at a delimiter. */
    async list(options: BucketListOptions = {}): Promise<BucketListing> {
        // select the keys after the cursor or start under the prefix
        const prefix = options.prefix ?? "";
        const limit = options.limit ?? MAX_BATCH_FILES;
        BucketListing.checkLimit(limit);
        const selection = JSON.stringify([prefix, options.delimiter ?? null]);
        const after =
            options.cursor === undefined
                ? options.startAfter
                : BucketListing.decodeCursor(options.cursor, "memory", selection);
        const entries = [...this.#files.entries()]
            .filter(([key]) => key.startsWith(prefix) && (after === undefined || key > after))
            .toSorted(([left], [right]) => (left < right ? -1 : 1));

        // fill the page with files and the prefixes the delimiter groups, each group's keys at once
        const files: BucketEntry[] = [];
        const prefixes = new Set<string>();
        let last: string | undefined;
        for (const [key, kept] of entries) {
            const rest = key.slice(prefix.length);
            const at = options.delimiter === undefined ? -1 : rest.indexOf(options.delimiter);
            const group =
                at < 0 ? undefined : prefix + rest.slice(0, at + (options.delimiter?.length ?? 0));
            if (group === undefined || !prefixes.has(group)) {
                if (files.length + prefixes.size === limit) {
                    break;
                } else if (group === undefined) {
                    files.push(BucketEntry.of(kept.file));
                } else {
                    prefixes.add(group);
                }
            }
            last = key;
        }

        // continue after the last key while more remain
        const isTruncated = last !== undefined && entries.at(-1)?.[0] !== last;

        return isTruncated
            ? {
                  files,
                  delimitedPrefixes: [...prefixes],
                  truncated: true,
                  cursor: BucketListing.encodeCursor("memory", selection, last ?? ""),
              }
            : { files, delimitedPrefixes: [...prefixes], truncated: false };
    }

    /** Refuse a multipart upload, which a memory bucket does not keep. */
    createMultipartUpload(): Promise<MultipartUpload> {
        return Promise.reject(
            BucketError.of("UNSUPPORTED", "a memory bucket does not support multipart uploads"),
        );
    }

    /** Refuse a multipart upload, which a memory bucket does not keep. */
    resumeMultipartUpload(): MultipartUpload {
        throw BucketError.of("UNSUPPORTED", "a memory bucket does not support multipart uploads");
    }

    /** Refuse to replace or delete a file its lock keeps. */
    #requireUnlocked(file: BucketFile | null): void {
        if (file?.isLocked(this.#clock.now()) === true) {
            throw BucketError.of("ACCESS_DENIED", `file ${file.key} is locked`);
        }
    }
}
