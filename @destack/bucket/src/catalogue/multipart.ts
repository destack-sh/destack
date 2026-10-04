import { aligned } from "@destack/schema";
import { createHash } from "node:crypto";
import { and, asc, eq, gt, gte, lt, or } from "@destack/db";
import type {
    Bucket,
    BucketBody,
    BucketFile,
    MultipartOptions,
    UploadPartOptions,
} from "../bucket/index.ts";
import { BucketFileBody, StorageClass } from "../bucket/index.ts";
import { BucketKey } from "../bucket/key.ts";
import { BucketListing, MAX_BATCH_FILES } from "../bucket/list.ts";
import { MAX_PART_NUMBER, UploadedPart } from "../bucket/multipart.ts";
import type {
    Part,
    PartCopyOptions,
    PartListing,
    PartListOptions,
    S3MultipartUpload,
    UploadListing,
    UploadListOptions,
} from "../s3/bucket.ts";
import { StorageError } from "../error/index.ts";
import { part, upload } from "./stack/index.ts";
import { Content } from "./content.ts";
import type { CatalogueStorage } from "./storage.ts";
import { CatalogueFile } from "./file.ts";
import { CustomerKey } from "./encryption.ts";

/** Retain incomplete uploads for seven days. */
const UPLOAD_RETENTION = 7 * 24 * 60 * 60 * 1000;
/** Require five MiB for every multipart part except the last. */
const MINIMUM_PART_SIZE = 5 * 1024 * 1024;

/** A multipart upload in an embedded bucket. */
export class CatalogueMultipartUpload implements S3MultipartUpload {
    /** The destination key. */
    readonly key: string;
    /** The upload identifier. */
    readonly uploadId: string;

    /** The shared catalogue and content references. */
    readonly #storage: CatalogueStorage;
    /** The bucket part copies read from. */
    readonly #bucket: Bucket;

    /** Retain the upload identifiers, its open storage and its bucket. */
    constructor(storage: CatalogueStorage, bucket: Bucket, key: string, uploadId: string) {
        // retain the identifiers, the storage and the bucket
        this.key = key;
        this.uploadId = uploadId;
        this.#storage = storage;
        this.#bucket = bucket;
    }

    /** Create a durable multipart upload. */
    static async create(
        storage: CatalogueStorage,
        bucket: Bucket,
        key: string,
        options: MultipartOptions = {},
    ): Promise<CatalogueMultipartUpload> {
        // validate the upload before registering it
        BucketKey.check(key);
        const customerKey = await CustomerKey.read(options.ssecKey);
        const storageClass = StorageClass.read(options.storageClass ?? "Standard");
        const uploadId = crypto.randomUUID();

        // register the upload in an unfenced bucket after reclaiming earlier writes
        await storage.exclusive(async () => {
            storage.checkWritable();
            await storage.collect();
            await storage.database.insert(upload).values({
                id: uploadId,
                key,
                state: "active",
                expires: Date.now() + UPLOAD_RETENTION,
                storageClass,
                ssecKeyMd5: customerKey?.md5 ?? null,
                options: {
                    httpMetadata: CatalogueFile.encodeHttpMetadata(options.httpMetadata ?? {}),
                    customMetadata: options.customMetadata ?? {},
                },
            });
        });

        return new CatalogueMultipartUpload(storage, bucket, key, uploadId);
    }

    /** List active uploads in key and upload identifier order while having the catalogue lock. */
    static async list(
        storage: CatalogueStorage,
        options: UploadListOptions,
    ): Promise<UploadListing> {
        // validate the prefix and page size
        const prefix = options.prefix ?? "";
        if (prefix) {
            BucketKey.check(prefix);
        }
        const limit = options.limit ?? MAX_BATCH_FILES;
        BucketListing.checkLimit(limit);

        // seek past the markers within the prefix, comparing keys and identifiers by their UTF-8 bytes
        const end = BucketKey.prefixEnd(prefix);
        const key = upload.key;
        const id = upload.id;
        const { keyMarker, uploadIdMarker } = options;
        const after =
            keyMarker === undefined
                ? undefined
                : uploadIdMarker === undefined
                  ? gt(key, keyMarker)
                  : or(gt(key, keyMarker), and(eq(upload.key, keyMarker), gt(id, uploadIdMarker)));

        // read one lookahead entry to tell whether another page exists
        const entries = await storage.exclusive(
            async () =>
                await storage.database
                    .select()
                    .from(upload)
                    .where(
                        and(
                            eq(upload.state, "active"),
                            gt(upload.expires, Date.now()),
                            gte(key, prefix),
                            end === undefined ? undefined : lt(key, end),
                            after,
                        ),
                    )
                    .orderBy(asc(key), asc(id))
                    .limit(limit + 1),
        );

        return {
            uploads: entries.slice(0, limit).map((entry) => ({
                key: entry.key,
                uploadId: entry.id,
                initiated: new Date(entry.expires - UPLOAD_RETENTION),
                storageClass: entry.storageClass,
            })),
            truncated: entries.length > limit,
        };
    }

    /** Write and atomically replace one upload part. */
    async uploadPart(
        partNumber: number,
        body: BucketBody,
        options: UploadPartOptions = {},
    ): Promise<Part> {
        // validate the part and require the upload's customer key before reading its body
        UploadedPart.checkNumber(partNumber);
        const customerKey = await CustomerKey.read(options.ssecKey);
        const active = await this.#storage.exclusive(() => this.#upload());
        CustomerKey.require(customerKey, active.ssecKeyMd5);

        // retain storage throughout the streamed upload
        await this.#storage.beginWrite();
        let content: Content | undefined;
        let isPublished = false;
        try {
            // write immutable contents before publishing the part
            content = await Content.write(this.#storage.blobs, body, {}, customerKey);
            const entry = {
                uploadId: this.uploadId,
                partNumber,
                blob: content.blob,
                nonce: content.nonce,
                size: content.size,
                etag: content.version,
                md5: content.etag,
                uploaded: Date.now(),
            };

            // publish only if the upload remains active
            await this.#storage.exclusive(async () => {
                await this.#replacePart(entry);
                isPublished = true;
            });

            return {
                partNumber,
                etag: entry.etag,
                size: entry.size,
                uploaded: new Date(entry.uploaded),
            };
        } finally {
            // release the active writer and retain abandoned files for collection
            if (content && !isPublished) {
                this.#storage.retired.add(content.blob);
            }
            this.#storage.endWrite();
        }
    }

    /** Replace one part of the active upload, retiring the contents it replaces, while having the catalogue lock. */
    async #replacePart(entry: typeof part.$inferSelect): Promise<void> {
        // read the part being replaced
        await this.#upload();
        const previous = await this.#storage.database
            .select()
            .from(part)
            .where(and(eq(part.uploadId, this.uploadId), eq(part.partNumber, entry.partNumber)))
            .get();

        // replace one part without invalidating existing readers
        await this.#storage.database
            .insert(part)
            .values(entry)
            .onConflictDoUpdate({
                target: [part.uploadId, part.partNumber],
                set: entry,
            });

        // retire replaced contents after committing their replacement
        if (previous) {
            this.#storage.retired.add(previous.blob);
        }
    }

    /** Stream a file or a range of it into one upload part. */
    async uploadPartCopy(
        partNumber: number,
        source: string,
        options: PartCopyOptions = {},
    ): Promise<Part | null> {
        // read the source under its preconditions
        UploadedPart.checkNumber(partNumber);
        const selected = await this.#bucket.get(source, options);
        if (selected === null) {
            throw new StorageError("NO_SUCH_KEY", "the source file does not exist");
        }
        if (!(selected instanceof BucketFileBody)) {
            return null;
        }

        return await this.uploadPart(partNumber, selected.body);
    }

    /** List the stored parts in part number order. */
    async listParts(options: PartListOptions = {}): Promise<PartListing> {
        // validate the page size
        const limit = options.limit ?? MAX_BATCH_FILES;
        BucketListing.checkLimit(limit);

        // read one lookahead part of the active upload
        const entries = await this.#storage.exclusive(async () => {
            await this.#upload();

            return await this.#storage.database
                .select()
                .from(part)
                .where(
                    and(
                        eq(part.uploadId, this.uploadId),
                        gt(part.partNumber, options.partNumberMarker ?? 0),
                    ),
                )
                .orderBy(asc(part.partNumber))
                .limit(limit + 1);
        });

        return {
            parts: entries.slice(0, limit).map((entry) => ({
                partNumber: entry.partNumber,
                etag: entry.etag,
                size: entry.size,
                uploaded: new Date(entry.uploaded),
            })),
            truncated: entries.length > limit,
        };
    }

    /** Publish the selected parts as the segments of one file in one catalogue transaction. */
    async complete(selected: UploadedPart[]): Promise<BucketFile> {
        requireDistinct(selected);

        return await this.#storage.exclusive(async () => {
            // read the upload of an unfenced bucket and match the selected parts
            this.#storage.checkWritable();
            const metadata = await this.#upload();
            const entries = await this.#storage.database
                .select()
                .from(part)
                .where(eq(part.uploadId, this.uploadId));
            const ordered = matchParts(selected, entries);
            requireSizes(ordered);

            // publish the file its parts' blobs become
            const published = this.#describe(metadata, ordered);
            const detached = await this.#publish(published, ordered);

            // retire the replaced blobs and every part's, which collection keeps while a segment references them
            for (const digest of detached) {
                this.#storage.retired.add(digest);
            }
            for (const { blob } of entries) {
                this.#storage.retired.add(blob);
            }

            return CatalogueFile.describe(published);
        });
    }

    /** Describe the file an upload's ordered parts become, versioned as its first part's write. */
    #describe(
        metadata: typeof upload.$inferSelect,
        ordered: readonly (typeof part.$inferSelect)[],
    ): CatalogueFile {
        const first = aligned(ordered, 0);

        return {
            key: this.key,
            version: first.etag,
            size: ordered.reduce((total, entry) => total + entry.size, 0),
            etag: multipartEtag(ordered),
            checksums: {},
            uploaded: Date.now(),
            httpMetadata: metadata.options.httpMetadata ?? {},
            customMetadata: metadata.options.customMetadata ?? {},
            storageClass: metadata.storageClass,
            ssecKeyMd5: metadata.ssecKeyMd5,
        };
    }

    /** Publish a completed file and close its upload in one transaction, returning the blobs it detaches. */
    async #publish(
        published: CatalogueFile,
        ordered: readonly (typeof part.$inferSelect)[],
    ): Promise<string[]> {
        return await this.#storage.database.transaction(async (transaction) => {
            // publish the segments, then drop the parts and close the upload
            const released = await this.#storage.publish(published, [...ordered], transaction);
            await transaction.delete(part).where(eq(part.uploadId, this.uploadId));
            await transaction
                .update(upload)
                .set({ state: "completed" })
                .where(eq(upload.id, this.uploadId));

            return released;
        });
    }

    /** Discard an incomplete upload and retire each uploaded part. */
    async abort(): Promise<void> {
        await this.#storage.exclusive(async () => {
            // read the upload of an unfenced bucket after reclaiming earlier writes
            this.#storage.checkWritable();
            await this.#storage.collect();
            const entry = await this.#storage.database
                .select()
                .from(upload)
                .where(
                    and(
                        eq(upload.id, this.uploadId),
                        eq(upload.key, this.key),
                        gt(upload.expires, Date.now()),
                    ),
                )
                .get();
            if (!entry) {
                throw new StorageError("NO_SUCH_UPLOAD", "multipart upload does not exist");
            }
            if (entry.state !== "active") {
                return;
            }

            // delete the parts and mark the upload aborted in one transaction
            const removed = await this.#storage.database.transaction(async (transaction) => {
                const deleted = await transaction
                    .delete(part)
                    .where(eq(part.uploadId, this.uploadId))
                    .returning({ blob: part.blob });
                await transaction
                    .update(upload)
                    .set({ state: "aborted" })
                    .where(eq(upload.id, this.uploadId));

                return deleted;
            });
            for (const { blob } of removed) {
                this.#storage.retired.add(blob);
            }
        });
    }

    /** Read a live upload for the requested key while having the catalogue lock. */
    async #upload(): Promise<typeof upload.$inferSelect> {
        const entry = await this.#storage.database
            .select()
            .from(upload)
            .where(
                and(
                    eq(upload.id, this.uploadId),
                    eq(upload.key, this.key),
                    eq(upload.state, "active"),
                    gt(upload.expires, Date.now()),
                ),
            )
            .get();
        if (!entry) {
            throw new StorageError("NO_SUCH_UPLOAD", "multipart upload does not exist");
        }

        return entry;
    }
}

/** Require distinct parts within the part limit. */
function requireDistinct(selected: readonly UploadedPart[]): void {
    if (
        selected.length === 0 ||
        selected.length > MAX_PART_NUMBER ||
        new Set(selected.map((entry) => entry.partNumber)).size !== selected.length
    ) {
        throw new StorageError(
            "INVALID_PART",
            `completion requires 1–${MAX_PART_NUMBER} distinct parts`,
        );
    }
}

/** Match selected entity tags to the exact uploaded parts, in part number order. */
function matchParts(
    selected: readonly UploadedPart[],
    entries: readonly (typeof part.$inferSelect)[],
): (typeof part.$inferSelect)[] {
    const indexed = new Map(entries.map((entry) => [entry.partNumber, entry]));

    return selected
        .map((choice) => {
            // refuse a missing or changed part
            const entry = indexed.get(choice.partNumber);
            if (!entry || entry.etag !== choice.etag) {
                throw new StorageError("INVALID_PART", "a selected part is missing or has changed");
            }

            return entry;
        })
        .toSorted((left, right) => left.partNumber - right.partNumber);
}

/** Require equal parts of at least five MiB, except a final part no larger than the first. */
function requireSizes(ordered: readonly (typeof part.$inferSelect)[]): void {
    const first = aligned(ordered, 0);
    const last = aligned(ordered, ordered.length - 1);
    if (
        ordered.length > 1 &&
        (ordered
            .slice(0, -1)
            .some((entry) => entry.size < MINIMUM_PART_SIZE || entry.size !== first.size) ||
            last.size > first.size)
    ) {
        throw new StorageError(
            "INVALID_PART",
            "multipart parts require equal sizes of at least five MiB, except the final part",
        );
    }
}

/** Derive the multipart entity tag from the parts' checksums. */
function multipartEtag(ordered: readonly (typeof part.$inferSelect)[]): string {
    const hash = createHash("md5");
    for (const entry of ordered) {
        hash.update(Uint8Array.fromHex(entry.md5));
    }

    return `${hash.digest("hex")}-${ordered.length}`;
}
