import { join } from "node:path";
import { inArray } from "@destack/db";
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
import { StorageError } from "../error/index.ts";
import { file, segment } from "./stack/index.ts";
import { ContentFile } from "./content.ts";
import { ContentReader } from "./reader.ts";
import { LocalMultipartUpload } from "./multipart.ts";
import { LocalStorage } from "./storage.ts";
import { LocalFile } from "./file.ts";
import { CustomerKey } from "./encryption.ts";

/** Persistent file storage backed by a SQLite catalogue and immutable content files. */
export class LocalBucket implements S3Bucket, AsyncDisposable {
    /** The host's open storage. */
    readonly #storage: LocalStorage;

    /** Retain the opened catalogue and content directory. */
    private constructor(storage: LocalStorage) {
        this.#storage = storage;
    }

    /** The bucket directory. */
    get directory(): string {
        return this.#storage.directory;
    }

    /** Open or create a bucket without contacting a remote service. */
    static async open(directory: string): Promise<LocalBucket> {
        return new LocalBucket(await LocalStorage.open(directory));
    }

    /** Read the current metadata. */
    async head(key: string): Promise<BucketFile | null> {
        BucketKey.check(key);
        const entry = await this.#storage.exclusive(() => this.#storage.entry(key));

        return entry ? LocalFile.describe(entry) : null;
    }

    /** Open the immutable contents selected by the catalogue. */
    get(
        key: string,
        options?: BucketGetOptions & { onlyIf?: undefined },
    ): Promise<BucketFileBody | null>;
    get(key: string, options: BucketGetOptions): Promise<BucketFileBody | BucketFile | null>;
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
            const current = LocalFile.describe(entry);
            if (!BucketCondition.matches(current, options.onlyIf)) {
                return current;
            }

            // retain the segments while another caller replaces or deletes the key
            const range = options.range
                ? BucketRange.resolve(entry.size, options.range)
                : undefined;
            const segments = await this.#storage.segments(entry.version);
            for (const selected of segments) {
                this.#storage.retain(selected.content);
            }
            const reader = new ContentReader(
                join(this.#storage.directory, "files"),
                segments,
                range?.offset ?? 0,
                range?.length ?? entry.size,
                () => {
                    for (const selected of segments) {
                        this.#storage.release(selected.content);
                    }
                },
                customerKey,
            );
            try {
                return new BucketFileBody(current, reader.stream, range);
            } catch (error) {
                try {
                    await reader.stream.cancel();
                } catch (cleanup) {
                    throw new AggregateError([error, cleanup], "file read and cancellation failed");
                }
                throw error;
            }
        });
    }

    /** Write immutable contents, then atomically publish their catalogue entry. */
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
        return this.#write(key, body, options);
    }

    /** Write a file with the entity tag, version and upload time another bucket gave it. */
    async restore(
        key: string,
        body: BucketBody,
        file: Pick<BucketFile, "etag" | "version" | "uploaded"> & Omit<BucketPutOptions, "onlyIf">,
    ): Promise<BucketFile> {
        const { etag, version, uploaded, ...options } = file;

        return this.#write(key, body, options, { etag, version, uploaded });
    }

    /** Fetch a file from a URL with the identity another bucket gave it, unless the bucket has it at that version. */
    async fetch(
        url: string,
        file: Pick<BucketFile, "key" | "etag" | "version" | "uploaded"> &
            Omit<BucketPutOptions, "onlyIf">,
    ): Promise<void> {
        // skip a file the bucket has at the same version
        const { key, ...identity } = file;
        const present = await this.head(key);
        if (present?.etag === file.etag && present.version === file.version) {
            return;
        }

        // stream the file from its source
        const response = await fetch(url);
        if (!response.ok) {
            throw new StorageError("WRITE_FAILED", `fetching ${key} answered ${response.status}`);
        }
        await this.restore(key, response.body, identity);
    }

    /** List the keys in a range: after one key, up to and including another, by UTF-8 bytes as listings sort them. */
    async keys(range: { readonly after?: string; readonly last?: string }): Promise<string[]> {
        // list pages after the first key until one passes the last
        const isAfter = (key: string, other: string) =>
            Buffer.compare(Buffer.from(key), Buffer.from(other)) > 0;
        const { after, last } = range;
        const keys: string[] = [];
        let cursor: string | undefined;
        do {
            const page = await this.list({
                ...(cursor === undefined ? {} : { cursor }),
                ...(after === undefined ? {} : { startAfter: after }),
            });
            keys.push(...page.files.map((file) => file.key));
            cursor = page.cursor;
        } while (cursor !== undefined && (last === undefined || !isAfter(keys.at(-1)!, last)));

        return last === undefined ? keys : keys.filter((key) => !isAfter(key, last));
    }

    /** Write and publish a file with no precondition. */
    #write(
        key: string,
        body: BucketBody,
        options: BucketPutOptions & { onlyIf?: undefined },
        identity?: Pick<BucketFile, "etag" | "version" | "uploaded">,
    ): Promise<BucketFile>;
    /** Write and publish a file when its precondition matches, else answer null. */
    #write(
        key: string,
        body: BucketBody,
        options: BucketPutOptions,
        identity?: Pick<BucketFile, "etag" | "version" | "uploaded">,
    ): Promise<BucketFile | null>;
    /** Write immutable contents, then publish their catalogue entry under its own or a restored identity. */
    async #write(
        key: string,
        body: BucketBody,
        options: BucketPutOptions,
        identity?: Pick<BucketFile, "etag" | "version" | "uploaded">,
    ): Promise<BucketFile | null> {
        // retain the storage while writing unpublished content
        BucketKey.check(key);
        const customerKey = await CustomerKey.read(options.ssecKey);
        const storageClass = StorageClass.read(options.storageClass ?? "Standard");
        await this.#storage.beginUpload();
        let isPublished = false;
        let content: ContentFile | undefined;
        try {
            content = await ContentFile.write(
                join(this.#storage.directory, "files"),
                body,
                options,
                customerKey,
            );

            // describe the file its immutable content becomes
            const segments = [{ content: content.version, size: content.size }];
            const entry = {
                key,
                version: identity?.version ?? content.version,
                etag: identity?.etag ?? content.etag,
                checksums: content.checksums,
                size: content.size,
                uploaded: identity?.uploaded.getTime() ?? Date.now(),
                httpMetadata: LocalFile.encodeHttpMetadata(options.httpMetadata ?? {}),
                customMetadata: options.customMetadata ?? {},
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
                        previous ? LocalFile.describe(previous) : null,
                        options.onlyIf,
                    )
                ) {
                    return null;
                }

                // atomically publish the new immutable file
                const detached = await this.#storage.database.transaction((transaction) =>
                    this.#storage.publish(entry, segments, transaction),
                );

                // retire the previous contents after the database commit
                isPublished = true;
                for (const name of detached) {
                    this.#storage.retired.add(name);
                }

                return LocalFile.describe(entry);
            });
        } finally {
            // retain failed upload files for collection and crash recovery
            this.#storage.uploads--;
            if (content && !isPublished) {
                this.#storage.retired.add(content.version);
            }
        }
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
            // require the source and its preconditions
            const entry = await this.#storage.entry(source);
            if (!entry) {
                throw new StorageError("NO_SUCH_KEY", "the source file does not exist");
            }
            if (!BucketCondition.matches(LocalFile.describe(entry), options.onlyIf)) {
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
                        : LocalFile.encodeHttpMetadata(options.httpMetadata),
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

            return LocalFile.describe(copied);
        });
    }

    /** Remove the current key while retaining contents already opened by readers. */
    async delete(key: string | string[]): Promise<void> {
        // validate the complete request before deleting any keys
        const keys = typeof key === "string" ? [key] : key;
        if (keys.length > MAX_BATCH_FILES) {
            throw new StorageError("INVALID_LIMIT", "delete accepts at most 1000 file keys");
        }
        for (const key of keys) {
            BucketKey.check(key);
        }

        // delete the keys under the catalogue lock
        await this.#storage.exclusive(async () => {
            // reclaim earlier writes before changing any requested keys
            await this.#storage.collect();

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
                    .returning({ content: segment.content });
            });

            // preserve detached contents until their open readers finish
            for (const entry of detached) {
                this.#storage.retired.add(entry.content);
            }
        });
    }

    /** Reclaim expired uploads and detached files after their last reader closes. */
    async collect(): Promise<void> {
        await this.#storage.exclusive(() => this.#storage.collect());
    }

    /** List keys in SQLite's UTF-8 binary order. */
    async list(options: BucketListOptions = {}): Promise<BucketListing> {
        return await this.#storage.exclusive(() => this.#storage.list(options));
    }

    /** Create a durable multipart upload. */
    createMultipartUpload(key: string, options: MultipartOptions = {}): Promise<S3MultipartUpload> {
        return LocalMultipartUpload.create(this.#storage, this, key, options);
    }

    /** Reference an existing multipart upload. */
    resumeMultipartUpload(key: string, uploadId: string): S3MultipartUpload {
        return new LocalMultipartUpload(this.#storage, this, key, uploadId);
    }

    /** List active uploads in key and upload identifier order. */
    listUploads(options: UploadListOptions = {}): Promise<UploadListing> {
        return LocalMultipartUpload.list(this.#storage, options);
    }

    /** Close the catalogue, rejecting while readers or writers have content files open. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#storage[Symbol.asyncDispose]();
    }
}
