import { and, gt, inArray, Order, sum } from "@destack/db";
import type {
    BucketBody,
    BucketGetOptions,
    BucketListOptions,
    BucketListing,
    BucketPutOptions,
    MultipartOptions,
} from "../bucket/index.ts";
import { BucketFile, BucketFileBody, StorageClass } from "../bucket/index.ts";
import { BucketKey } from "../bucket/key.ts";
import { BucketRange } from "../bucket/range.ts";
import { BucketCondition } from "../bucket/condition.ts";
import { MAX_BATCH_FILES } from "../bucket/list.ts";
import type {
    BucketCopyOptions,
    S3Bucket,
    S3MultipartUpload,
    UploadListing,
    UploadListOptions,
} from "../s3/bucket.ts";
import { BucketError } from "../error/index.ts";
import { file, segment } from "./stack/index.ts";
import { Content } from "./content.ts";
import { ContentReader } from "./reader.ts";
import { CatalogueMultipartUpload } from "./multipart.ts";
import { type Catalogue, CatalogueStorage } from "./storage.ts";
import { CatalogueFile } from "./file.ts";
import { CustomerKey } from "./encryption.ts";
import type { CatalogueHandle } from "./snapshot.ts";

/** Persistent file storage: a catalogue of files in its host's database over a store of their blobs. */
export class CatalogueBucket implements S3Bucket, AsyncDisposable {
    /** The host's open storage. */
    readonly #storage: CatalogueStorage;

    /** Retain the opened catalogue and blobs. */
    protected constructor(storage: CatalogueStorage) {
        this.#storage = storage;
    }

    /** Keep a bucket over a catalogue its host opened, closing the catalogue with the bucket. */
    static create(catalogue: Catalogue): CatalogueBucket {
        return new CatalogueBucket(new CatalogueStorage(catalogue));
    }

    /** Share the catalogue and the bucket's blobs for copying the bucket between hosts, collecting the blobs its copies retire. */
    database(): CatalogueHandle {
        const storage = this.#storage;

        return {
            get database() {
                return storage.database;
            },
            blobs: storage.blobs,
            hold: () => storage.hold(),
            retire: (digests) => storage.retire(digests),
            migrate: (beside) => storage.migrate(beside),
            close: async () => {},
        };
    }

    /** Count the bytes the published files keep. */
    async bytes(): Promise<number> {
        const [row] = await this.#storage.database.select({ bytes: sum(file.size) }).from(file);

        return Number(row?.bytes ?? 0);
    }

    /** Read the current metadata. */
    async head(key: string): Promise<BucketFile | null> {
        BucketKey.check(key);
        const entry = await this.#storage.exclusive(() => this.#storage.entry(key));

        return entry ? CatalogueFile.describe(entry) : null;
    }

    /** Open the immutable contents selected by the catalogue. */
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
        // validate the requested key, customer key and byte selection before entering the catalogue lock
        BucketKey.check(key);
        const customerKey = await CustomerKey.read(options.ssecKey);
        if (options.range) {
            BucketRange.check(options.range);
        }

        return await this.#storage.exclusive(async () => {
            // require the file's customer key, then evaluate preconditions against the current file
            const entry = await this.#storage.entry(key);
            if (!entry) {
                return null;
            }
            CustomerKey.require(customerKey, entry.ssecKeyMd5);
            const current = CatalogueFile.describe(entry);
            if (!BucketCondition.matches(current, options.onlyIf)) {
                return current;
            }

            // read the selected bytes, cancelling the read when the body cannot take it
            const range = options.range
                ? BucketRange.resolve(entry.size, options.range)
                : undefined;
            const reader = await this.#reader(entry, range, customerKey);
            try {
                return new BucketFileBody(current, reader.stream, range);
            } catch (error) {
                try {
                    await reader.stream.cancel();
                } catch (cleanup) {
                    throw new AggregateError(
                        [error, cleanup],
                        "file read and cancellation failed",
                        {
                            cause: cleanup,
                        },
                    );
                }
                throw error;
            }
        });
    }

    /** Read a range of an entry's bytes, retaining its segments while another caller replaces or deletes the key. */
    async #reader(
        entry: CatalogueFile,
        range: { readonly offset: number; readonly length: number } | undefined,
        customerKey: CustomerKey | undefined,
    ): Promise<ContentReader> {
        const segments = await this.#storage.segments(entry.version);
        for (const selected of segments) {
            this.#storage.retain(selected.blob);
        }

        return new ContentReader(
            this.#storage.blobs,
            segments,
            range?.offset ?? 0,
            range?.length ?? entry.size,
            () => {
                for (const selected of segments) {
                    this.#storage.release(selected.blob);
                }
            },
            customerKey,
        );
    }

    /** Write immutable contents, then atomically publish their catalogue entry. */
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
        return this.#write(key, body, options);
    }

    /** Write a file with the entity tag, version and upload time another bucket gave it. */
    async restore(
        key: string,
        body: BucketBody | null,
        stored: Pick<BucketFile, "etag" | "version" | "uploaded"> &
            Omit<BucketPutOptions, "onlyIf">,
    ): Promise<BucketFile> {
        const { etag, version, uploaded, ...options } = stored;

        return this.#write(key, body, options, { etag, version, uploaded });
    }

    /** Fetch a file from a URL with the identity another bucket gave it, unless the bucket has it at that version. */
    async fetch(
        url: string,
        source: Pick<BucketFile, "key" | "etag" | "version" | "uploaded"> &
            Omit<BucketPutOptions, "onlyIf">,
    ): Promise<void> {
        // skip a file the bucket has at the same version
        const { key, ...identity } = source;
        const present = await this.head(key);
        if (present?.etag === source.etag && present.version === source.version) {
            return;
        }

        // stream the file from its source
        const response = await fetch(url);
        if (!response.ok) {
            throw new BucketError("WRITE_FAILED", `fetching ${key} answered ${response.status}`);
        }
        await this.restore(key, response.body, identity);
    }

    /** List the keys in a range: after one key, up to and including another, by UTF-8 bytes as listings sort them. */
    async keys(range: { readonly after?: string; readonly last?: string }): Promise<string[]> {
        // list pages after the first key until one passes the last
        const { after, last } = range;
        const keys: string[] = [];
        let cursor: string | undefined;
        let isPast = false;
        do {
            const page = await this.list({
                ...(cursor === undefined ? {} : { cursor }),
                ...(after === undefined ? {} : { startAfter: after }),
            });
            keys.push(...page.files.map((listed) => listed.key));
            cursor = page.cursor;
            const final = keys.at(-1);
            isPast = last !== undefined && final !== undefined && isAfter(final, last);
        } while (cursor !== undefined && !isPast);

        return last === undefined ? keys : keys.filter((key) => !isAfter(key, last));
    }

    /** Write and publish a file with no precondition. */
    #write(
        key: string,
        body: BucketBody | null,
        options: BucketPutOptions & { onlyIf?: undefined },
        identity?: Pick<BucketFile, "etag" | "version" | "uploaded">,
    ): Promise<BucketFile>;
    /** Write and publish a file when its precondition matches, else answer null. */
    #write(
        key: string,
        body: BucketBody | null,
        options: BucketPutOptions,
        identity?: Pick<BucketFile, "etag" | "version" | "uploaded">,
    ): Promise<BucketFile | null>;
    /**
     * Write immutable contents, then publish their catalogue entry under its own or a restored identity.
     *
     * @construct only an `onlyIf` precondition refuses a write.
     */
    async #write(
        key: string,
        body: BucketBody | null,
        options: BucketPutOptions,
        identity?: Pick<BucketFile, "etag" | "version" | "uploaded">,
    ): Promise<BucketFile | null> {
        // retain the storage while writing unpublished content
        BucketKey.check(key);
        const customerKey = await CustomerKey.read(options.ssecKey);
        const storageClass = StorageClass.read(options.storageClass ?? "Standard");
        await this.#storage.beginWrite();
        let isPublished = false;
        let content: Content | undefined;
        try {
            content = await Content.write(this.#storage.blobs, body, options, customerKey);

            // describe the file its blob becomes
            const segments = [{ blob: content.blob, nonce: content.nonce, size: content.size }];
            const entry = {
                ...CatalogueBucket.#entryOf(key, content, options, identity),
                storageClass,
                ssecKeyMd5: customerKey?.md5 ?? null,
            };

            // replace the catalogue entry only when its precondition still matches
            return await this.#storage.exclusive(async () => {
                // reclaim earlier writes, then check the precondition against the current entry
                await this.#storage.collect();
                const previous = await this.#storage.entry(key);
                if (
                    !BucketCondition.matches(
                        previous ? CatalogueFile.describe(previous) : null,
                        options.onlyIf,
                    )
                ) {
                    return null;
                }

                // atomically publish the new immutable file
                const detached = await this.#storage.database.transaction((transaction) =>
                    this.#storage.publish(entry, segments, transaction),
                );

                // retire the previous blobs after the database commit
                isPublished = true;
                for (const digest of detached) {
                    this.#storage.retired.add(digest);
                }

                return CatalogueFile.describe(entry);
            });
        } finally {
            // retire the blob of a failed upload for collection
            this.#storage.endWrite();
            if (content && !isPublished) {
                this.#storage.retired.add(content.blob);
            }
        }
    }

    /** Describe the catalogue entry of written content, under its own or a restored identity. */
    static #entryOf(
        key: string,
        content: Content,
        options: BucketPutOptions,
        identity: Pick<BucketFile, "etag" | "version" | "uploaded"> | undefined,
    ) {
        return {
            key,
            version: identity?.version ?? content.version,
            etag: identity?.etag ?? content.etag,
            checksums: content.checksums,
            size: content.size,
            uploaded: identity?.uploaded.getTime() ?? Date.now(),
            httpMetadata: CatalogueFile.encodeHttpMetadata(options.httpMetadata ?? {}),
            customMetadata: options.customMetadata ?? {},
            retainUntil: options.retainUntil?.getTime() ?? null,
        };
    }

    /** Publish a new version of the destination that shares the source's immutable content. */
    async copy(
        source: string,
        destination: string,
        options: BucketCopyOptions = {},
    ): Promise<BucketFile | null> {
        // validate both keys and the storage class before entering the catalogue lock
        BucketKey.check(source);
        BucketKey.check(destination);
        const storageClass = StorageClass.read(options.storageClass ?? "Standard");

        return await this.#storage.exclusive(async () => {
            // require an unfenced bucket, the source and its preconditions
            this.#storage.checkWritable();
            const entry = await this.#storage.entry(source);
            if (!entry) {
                throw new BucketError("NO_SUCH_KEY", "the source file does not exist");
            }
            if (!BucketCondition.matches(CatalogueFile.describe(entry), options.onlyIf)) {
                return null;
            }

            // describe the copy with the source's metadata unless the caller replaces it
            const copied = {
                ...entry,
                key: destination,
                version: crypto.randomUUID(),
                uploaded: Date.now(),
                httpMetadata:
                    options.httpMetadata === undefined
                        ? entry.httpMetadata
                        : CatalogueFile.encodeHttpMetadata(options.httpMetadata),
                customMetadata: options.customMetadata ?? entry.customMetadata,
                storageClass,
            };

            // publish the copy sharing the source's segments, and retire the contents it replaces
            const segments = await this.#storage.segments(entry.version);
            const detached = await this.#storage.database.transaction((transaction) =>
                this.#storage.publish(copied, segments, transaction),
            );
            for (const name of detached) {
                this.#storage.retired.add(name);
            }

            return CatalogueFile.describe(copied);
        });
    }

    /** Remove the current key while retaining contents already opened by readers. */
    async delete(key: string | string[]): Promise<void> {
        // validate the complete request before deleting any keys
        const keys = typeof key === "string" ? [key] : key;
        if (keys.length > MAX_BATCH_FILES) {
            throw new BucketError("INVALID_LIMIT", "delete accepts at most 1000 file keys");
        }
        for (const each of keys) {
            BucketKey.check(each);
        }

        // delete the keys under the catalogue lock
        await this.#storage.exclusive(async () => {
            // refuse a fenced bucket, and reclaim earlier writes before changing any requested keys
            this.#storage.checkWritable();
            await this.#storage.collect();

            // refuse the whole batch while one of its files is locked
            const locked = await this.#storage.database
                .select({ key: file.key })
                .from(file)
                .where(and(inArray(file.key, keys), gt(file.retainUntil, Date.now())));
            if (locked.length > 0) {
                throw new BucketError(
                    "LOCKED",
                    `file ${locked.map((entry) => entry.key).join(", ")} is retained until its lock expires`,
                );
            }

            // delete the files and detach their segments in one transaction
            const detached = await this.#storage.database.transaction(async (transaction) => {
                const deleted = await transaction
                    .delete(file)
                    .where(inArray(file.key, keys))
                    .returning({ version: file.version });

                return await transaction
                    .delete(segment)
                    .where(
                        inArray(
                            segment.version,
                            deleted.map((entry) => entry.version),
                        ),
                    )
                    .returning({ blob: segment.blob });
            });

            // preserve detached blobs until their open readers finish
            for (const entry of detached) {
                this.#storage.retired.add(entry.blob);
            }
        });
    }

    /** Reclaim expired uploads and detached files after their last reader closes. */
    async collect(): Promise<void> {
        await this.#storage.exclusive(() => this.#storage.collect());
    }

    /** Delete what interrupted writes left, answering false when writers kept it from finishing. */
    sweep(): Promise<boolean> {
        return this.#storage.sweep();
    }

    /** Refuse new writes while a transfer copies the bucket, returning once the writes in flight finished, and report whether this call set the fence. */
    fence(): Promise<boolean> {
        return this.#storage.fence();
    }

    /** Accept writes again after a fence. */
    lift(): Promise<void> {
        return this.#storage.lift();
    }

    /** Refuse a write into a fenced bucket, such as before presigning one. */
    checkWritable(): void {
        this.#storage.checkWritable();
    }

    /** List keys in UTF-8 byte order. */
    async list(options: BucketListOptions = {}): Promise<BucketListing> {
        return await this.#storage.exclusive(() => this.#storage.list(options));
    }

    /** Create a durable multipart upload. */
    createMultipartUpload(key: string, options: MultipartOptions = {}): Promise<S3MultipartUpload> {
        return CatalogueMultipartUpload.create(this.#storage, this, key, options);
    }

    /** Reference an existing multipart upload. */
    resumeMultipartUpload(key: string, uploadId: string): S3MultipartUpload {
        return new CatalogueMultipartUpload(this.#storage, this, key, uploadId);
    }

    /** List active uploads in key and upload identifier order. */
    listUploads(options: UploadListOptions = {}): Promise<UploadListing> {
        return CatalogueMultipartUpload.list(this.#storage, options);
    }

    /** Close the catalogue, rejecting while readers or writers have blobs open. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#storage[Symbol.asyncDispose]();
    }
}

/** Report whether a key sorts after another by UTF-8 bytes, as listings sort them. */
function isAfter(key: string, other: string): boolean {
    return Order.codePoints(key, other) > 0;
}
