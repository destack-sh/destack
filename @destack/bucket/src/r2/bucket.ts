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
    async get(
        key: string,
        options: BucketGetOptions = {},
    ): Promise<BucketFileBody | BucketFile | null> {
        // validate the key and range
        BucketKey.check(key);
        if (options.range) {
            BucketRange.check(options.range);
        }

        // pass the structured options through the separately declared Workers types
        const entry = await this.#bucket
            .get(key, options as Cloudflare.R2GetOptions)
            .catch((error: unknown) => {
                throw readError(error);
            });
        if (!entry) {
            return null;
        }
        if (!("body" in entry)) {
            return R2File.describe(entry);
        }

        // return the body with its resolved range
        const body = entry.body as unknown as ReadableStream<Uint8Array>;
        try {
            const range = options.range
                ? BucketRange.resolve(entry.size, options.range)
                : undefined;

            return new BucketFileBody(R2File.describe(entry), body, range);
        } catch (error) {
            try {
                await body.cancel();
            } catch (cleanup) {
                throw new AggregateError([error, cleanup], "file read and cancellation failed");
            }
            throw error;
        }
    }

    /** Publish a file with its metadata and optional precondition. */
    put(
        key: string,
        body: BucketBody,
        options?: BucketPutOptions & { onlyIf?: undefined },
    ): Promise<BucketFile>;
    put(key: string, body: BucketBody, options: BucketPutOptions): Promise<BucketFile | null>;
    async put(
        key: string,
        body: BucketBody,
        options: BucketPutOptions = {},
    ): Promise<BucketFile | null> {
        BucketKey.check(key);

        // pass the structured options through the separately declared Workers types
        const entry = await this.#bucket.put(
            key,
            body as Parameters<Cloudflare.R2Bucket["put"]>[1],
            options as unknown as Cloudflare.R2PutOptions,
        );

        return entry ? R2File.describe(entry) : null;
    }

    /** Delete the current file. */
    async delete(key: string | string[]): Promise<void> {
        // validate the keys before deleting
        const keys = typeof key === "string" ? [key] : key;
        if (keys.length > MAX_BATCH_FILES) {
            throw new StorageError("INVALID_LIMIT", "delete accepts at most 1000 file keys");
        }
        for (const key of keys) {
            BucketKey.check(key);
        }
        await this.#bucket.delete(key);
    }

    /** Create a multipart upload in R2. */
    async createMultipartUpload(
        key: string,
        options: MultipartOptions = {},
    ): Promise<MultipartUpload> {
        BucketKey.check(key);

        return new R2MultipartUpload(
            await this.#bucket.createMultipartUpload(
                key,
                options as unknown as Cloudflare.R2MultipartOptions,
            ),
        );
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
            cursor,
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
    if ((error as Cloudflare.R2Error).code === INVALID_RANGE_CODE) {
        return new StorageError("INVALID_RANGE", "the requested file range is not satisfiable", {
            cause: error,
        });
    }

    return error;
}
