import type * as Cloudflare from "@cloudflare/workers-types";
import type {
    Bucket,
    BucketBody,
    BucketGetOptions,
    BucketListOptions,
    BucketPutOptions,
    MultipartOptions,
    MultipartUpload,
} from "../bucket/index.ts";
import { BucketFile, BucketFileBody } from "../bucket/index.ts";
import { BucketKey } from "../bucket/key.ts";
import { BucketListing, MAX_BATCH_FILES } from "../bucket/list.ts";
import { BucketRange } from "../bucket/range.ts";
import { StorageError } from "../error/index.ts";
import { R2Body } from "./body.ts";
import { R2MultipartUpload } from "./multipart.ts";
import { R2File } from "./file.ts";

/** The code R2 gives an unsatisfiable range. */
const INVALID_RANGE_CODE = 10039;

/** File storage backed by a Cloudflare R2 binding. */
export class R2Bucket implements Bucket {
    /** The host-authorised bucket binding. */
    readonly #bucket: Cloudflare.R2Bucket;

    /** Use an R2 binding supplied by the host. */
    constructor(bucket: Cloudflare.R2Bucket) {
        this.#bucket = bucket;
    }

    /** Read the current file metadata. */
    async head(key: string): Promise<BucketFile | null> {
        BucketKey.check(key);
        const entry = await this.#bucket.head(key);

        return entry ? R2File.describe(entry) : null;
    }

    /** Read a file with R2's atomic precondition and range handling. */
    get(
        key: string,
        options?: BucketGetOptions & { onlyIf?: undefined },
    ): Promise<BucketFileBody | null>;
    get(key: string, options: BucketGetOptions): Promise<BucketFileBody | BucketFile | null>;
    /**
     * Read a file, or its metadata when a precondition fails.
     *
     * @construct only an `onlyIf` precondition answers metadata without a body.
     */
    async get(
        key: string,
        options: BucketGetOptions = {},
    ): Promise<BucketFileBody | BucketFile | null> {
        // validate the key and range
        BucketKey.check(key);
        if (options.range) {
            BucketRange.check(options.range);
        }

        // pass the structured options to R2
        const entry = await this.#bucket.get(key, options).catch((error: unknown) => {
            throw readError(error);
        });
        if (!entry) {
            return null;
        }
        if (!("body" in entry)) {
            return R2File.describe(entry);
        }

        // return the body with its resolved range
        const body = R2Body.read(entry);
        try {
            const range = options.range
                ? BucketRange.resolve(entry.size, options.range)
                : undefined;

            return new BucketFileBody(R2File.describe(entry), body, range);
        } catch (error) {
            try {
                await body.cancel();
            } catch (cleanup) {
                throw new AggregateError([error, cleanup], "file read and cancellation failed", {
                    cause: cleanup,
                });
            }
            throw error;
        }
    }

    /** Publish a file with its metadata and optional precondition. */
    put(
        key: string,
        body: BucketBody | null,
        options?: BucketPutOptions & { onlyIf?: undefined },
    ): Promise<BucketFile>;
    put(
        key: string,
        body: BucketBody | null,
        options: BucketPutOptions,
    ): Promise<BucketFile | null>;
    /**
     * Write a file, or answer null when its precondition fails.
     *
     * @construct only an `onlyIf` precondition refuses a write.
     */
    async put(
        key: string,
        body: BucketBody | null,
        options: BucketPutOptions = {},
    ): Promise<BucketFile | null> {
        // validate the key and pass the body as R2 takes it
        BucketKey.check(key);
        const value = body === null ? null : R2Body.write(body);
        const entry = await this.#bucket.put(key, value, options);

        return entry === null ? null : R2File.describe(entry);
    }

    /** Delete the current file. */
    async delete(key: string | string[]): Promise<void> {
        // validate the keys before deleting
        const keys = typeof key === "string" ? [key] : key;
        if (keys.length > MAX_BATCH_FILES) {
            throw new StorageError("INVALID_LIMIT", "delete accepts at most 1000 file keys");
        }
        for (const each of keys) {
            BucketKey.check(each);
        }
        await this.#bucket.delete(key);
    }

    /** Create a multipart upload in R2. */
    async createMultipartUpload(
        key: string,
        options: MultipartOptions = {},
    ): Promise<MultipartUpload> {
        BucketKey.check(key);

        return new R2MultipartUpload(await this.#bucket.createMultipartUpload(key, options));
    }

    /** Reference an existing R2 upload. */
    resumeMultipartUpload(key: string, uploadId: string): MultipartUpload {
        return new R2MultipartUpload(this.#bucket.resumeMultipartUpload(key, uploadId));
    }

    /** List files with the selected metadata and delimiter. */
    async list(options: BucketListOptions = {}): Promise<BucketListing> {
        // validate the limit and bind cursors to the prefix and delimiter
        BucketListing.checkLimit(options.limit ?? MAX_BATCH_FILES);
        const prefix = options.prefix ?? "";
        const selection = JSON.stringify([prefix, options.delimiter ?? ""]);
        if (prefix) {
            BucketKey.check(prefix);
        }
        const cursor =
            options.cursor === undefined
                ? undefined
                : BucketListing.decodeCursor(options.cursor, "r2", selection);
        const page = await this.#bucket.list({
            ...options,
            ...(cursor === undefined ? {} : { cursor }),
        });

        return {
            files: page.objects.map(R2File.describe),
            delimitedPrefixes: page.delimitedPrefixes,
            ...(page.truncated
                ? {
                      truncated: true,
                      cursor: BucketListing.encodeCursor("r2", selection, page.cursor),
                  }
                : { truncated: false }),
        };
    }
}

/** Report R2's unsatisfiable range as the storage failure of the same meaning, and any other failure as itself. */
function readError(error: unknown): unknown {
    if (
        typeof error === "object" &&
        error !== null &&
        "code" in error &&
        error.code === INVALID_RANGE_CODE
    ) {
        return new StorageError("INVALID_RANGE", "the requested file range is not satisfiable", {
            cause: error,
        });
    }

    return error;
}
