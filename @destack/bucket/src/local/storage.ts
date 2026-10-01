import { mkdir, opendir, unlink } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { and, asc, eq, gt, gte, inArray, lt, lte } from "@destack/db";
import type { BucketFile, BucketListOptions } from "../bucket/index.ts";
import { BucketKey } from "../bucket/key.ts";
import { BucketListing, MAX_BATCH_FILES } from "../bucket/list.ts";
import { StorageError } from "../error/index.ts";
import { catalogueDatabase, file, part, segment, upload } from "./stack/index.ts";
import { LocalFile } from "./file.ts";
import type { Segment } from "./reader.ts";
import { connect } from "@destack/db/bun";
import type { SqliteDatabase } from "@destack/db/sqlite";
import { syncDirectory } from "./directory.ts";
import { FileLock } from "@destack/fs";

/** Bound content reference queries and their SQL parameter counts. */
const REFERENCE_BATCH_SIZE = 500;
/** The name of a content file, the random UUID of the upload that wrote it. */
const CONTENT_NAME = /^[0-9a-f]{8}-(?:[0-9a-f]{4}-){3}[0-9a-f]{12}$/;

/** A catalogue transaction. */
export type CatalogueTransaction = Parameters<Parameters<SqliteDatabase["transaction"]>[0]>[0];

/** A bucket's catalogue, content references, and exclusive host lock. */
export class LocalStorage implements AsyncDisposable {
    /** The bucket directory. */
    readonly directory: string;
    /** The private catalogue connection. */
    readonly database: SqliteDatabase;
    /** The operating-system lock this host has taken. */
    readonly #lock: FileLock;
    /** Operations that select or replace content files. */
    #pending: Promise<void> = Promise.resolve();
    /** Open download streams by content filename. */
    readonly #readers = new Map<string, number>();
    /** Content files detached from their keys. */
    readonly retired = new Set<string>();
    /** The number of uploads writing contents they have not yet published. */
    uploads = 0;
    /** Whether the bucket has released its database and lock. */
    #isClosed = false;
    /** Share an in-progress close between concurrent callers. */
    #closing?: Promise<void>;

    /** Retain an opened catalogue. */
    private constructor(directory: string, database: SqliteDatabase, lock: FileLock) {
        this.directory = directory;
        this.database = database;
        this.#lock = lock;
    }

    /** Open or create a bucket without contacting a remote service. */
    static async open(directory: string): Promise<LocalStorage> {
        // create the content directory before publishing any catalogue entries
        directory = resolve(directory);
        const contents = join(directory, "files");
        const created = await mkdir(contents, { recursive: true, mode: 0o700 });

        // persist each newly created directory entry through its existing parent
        if (created !== undefined) {
            const parent = dirname(created);
            let current = contents;
            while (true) {
                await syncDirectory(current);
                if (current === parent) {
                    break;
                }
                current = dirname(current);
            }
        }
        const lock = await FileLock.acquire(join(directory, "bucket.lock"));
        let database: SqliteDatabase | undefined;

        // prepare the private catalogue through the shared database lifecycle
        try {
            database = await connect(join(directory, "bucket.db"), catalogueDatabase);
            await database.migrate(catalogueDatabase.tables);

            // persist the initial catalogue and lock entries before accepting writes
            await syncDirectory(directory);

            // reclaim files left by interrupted uploads or a terminated host
            const bucket = new LocalStorage(directory, database, lock);
            await bucket.#recover();
            await bucket.collect();

            return bucket;
        } catch (error) {
            const failures = [error];
            try {
                await database?.close();
            } catch (cleanup) {
                failures.push(cleanup);
            }
            try {
                await lock.close();
            } catch (cleanup) {
                failures.push(cleanup);
            }
            if (failures.length > 1) {
                throw new AggregateError(failures, "storage initialization and closure failed");
            }
            throw error;
        }
    }

    /** Recover unreferenced files with bounded directory and catalogue reads. */
    async #recover(): Promise<void> {
        // collect content filenames in bounded batches
        const directory = await opendir(join(this.directory, "files"));
        let contents: string[] = [];
        for await (const entry of directory) {
            if (!entry.isFile() || !CONTENT_NAME.test(entry.name)) {
                continue;
            }
            contents.push(entry.name);
            if (contents.length === REFERENCE_BATCH_SIZE) {
                await this.#unlinkUnreferenced(contents);
                contents = [];
            }
        }
        if (contents.length) {
            await this.#unlinkUnreferenced(contents);
        }
    }

    /** Delete the content files no file or part references, at most one batch of them. */
    async #unlinkUnreferenced(contents: string[]): Promise<void> {
        const referenced = await this.#referenced(contents);
        for (const content of contents) {
            if (!referenced.has(content)) {
                await unlink(join(this.directory, "files", content));
            }
        }
    }

    /** Select the contents a file segment or a part references, at most one batch of them. */
    async #referenced(contents: string[]): Promise<Set<string>> {
        const segments = await this.database
            .select({ content: segment.content })
            .from(segment)
            .where(inArray(segment.content, contents));
        const parts = await this.database
            .select({ content: part.content })
            .from(part)
            .where(inArray(part.content, contents));

        return new Set([...segments, ...parts].map((entry) => entry.content));
    }

    /** Close the private database connection. */
    async [Symbol.asyncDispose](): Promise<void> {
        if (this.#isClosed) {
            return;
        }
        this.#closing ??= this.#close();
        try {
            await this.#closing;
        } finally {
            this.#closing = undefined;
        }
    }

    /** Finish maintenance and release the catalogue and host lock. */
    async #close(): Promise<void> {
        await this.exclusive(async () => {
            if (this.uploads !== 0 || this.#readers.size !== 0) {
                throw new StorageError(
                    "BUSY",
                    "consume or cancel bucket streams before closing storage",
                );
            }
            await this.collect();

            // retain the host lock if the catalogue cannot close safely
            await this.database.close();
            await this.#lock.close();
            this.#isClosed = true;
        });
    }

    /** Reclaim detached contents after their last reader closes. */
    async collect(): Promise<void> {
        // expire one upload at a time, each bounded by the 10000-part limit
        while (true) {
            const expired = await this.database
                .select({ id: upload.id })
                .from(upload)
                .where(lte(upload.expires, Date.now()))
                .limit(1)
                .get();
            if (!expired) {
                break;
            }
            const removed = await this.database.transaction(async (transaction) => {
                const entries = await transaction
                    .delete(part)
                    .where(eq(part.uploadId, expired.id))
                    .returning({ content: part.content });
                await transaction.delete(upload).where(eq(upload.id, expired.id));

                return entries;
            });
            for (const entry of removed) {
                this.retired.add(entry.content);
            }
            await this.#collectFiles();
        }

        await this.#collectFiles();
    }

    /** Delete retired files no reader has open and no file segment or part still references. */
    async #collectFiles(): Promise<void> {
        // keep every file an active reader retains
        const released = [...this.retired].filter((content) => !this.#readers.has(content));
        for (let start = 0; start < released.length; start += REFERENCE_BATCH_SIZE) {
            // keep files a copy or a completed upload still shares, which retire again with their last reference
            const batch = released.slice(start, start + REFERENCE_BATCH_SIZE);
            const referenced = await this.#referenced(batch);
            for (const content of batch) {
                if (!referenced.has(content)) {
                    await unlink(join(this.directory, "files", content));
                }
                this.retired.delete(content);
            }
        }
    }

    /** Retain a content file until its reader finishes. */
    retain(content: string): void {
        const count = this.#readers.get(content);
        this.#readers.set(content, count === undefined ? 1 : count + 1);
    }

    /** Release a retained content file. */
    release(content: string): void {
        const remaining = this.#readers.get(content)! - 1;
        if (remaining === 0) {
            this.#readers.delete(content);
        } else {
            this.#readers.set(content, remaining);
        }
    }

    /** Reclaim earlier writes and prevent closure while an upload writes its contents. */
    async beginUpload(): Promise<void> {
        await this.exclusive(async () => {
            await this.collect();
            this.uploads++;
        });
    }

    /** Read the catalogue entry of a key while the caller has the catalogue lock. */
    async entry(key: string): Promise<LocalFile | undefined> {
        return await this.database.select().from(file).where(eq(file.key, key)).get();
    }

    /** Read the content segments of a file version in order while the caller has the catalogue lock. */
    async segments(version: string): Promise<Segment[]> {
        return await this.database
            .select({ content: segment.content, size: segment.size })
            .from(segment)
            .where(eq(segment.version, version))
            .orderBy(asc(segment.position));
    }

    /** Publish a file version and its segments, returning the contents the replaced version detaches. */
    async publish(
        entry: LocalFile,
        segments: Segment[],
        transaction: CatalogueTransaction,
    ): Promise<string[]> {
        // replace the key's file and detach the segments of its previous version
        const previous = await transaction
            .select({ version: file.version })
            .from(file)
            .where(eq(file.key, entry.key))
            .get();
        await transaction.insert(file).values(entry).onConflictDoUpdate({
            target: file.key,
            set: entry,
        });
        const detached =
            previous === undefined
                ? []
                : await transaction
                      .delete(segment)
                      .where(eq(segment.version, previous.version))
                      .returning({ content: segment.content });

        // write the new version's segments in bounded batches
        const rows = segments.map((selected, position) => ({
            version: entry.version,
            position,
            content: selected.content,
            size: selected.size,
        }));
        for (let start = 0; start < rows.length; start += REFERENCE_BATCH_SIZE) {
            await transaction
                .insert(segment)
                .values(rows.slice(start, start + REFERENCE_BATCH_SIZE));
        }

        return detached.map((selected) => selected.content);
    }

    /** Select a page of files while the caller has the catalogue lock. */
    async list(options: BucketListOptions): Promise<BucketListing> {
        // validate the optional prefix as a key
        const prefix = options.prefix ?? "";
        if (prefix) {
            BucketKey.check(prefix);
        }
        const limit = options.limit ?? MAX_BATCH_FILES;
        BucketListing.checkLimit(limit);

        // seek the key index between the prefix and its exclusive upper bound
        const delimiter = options.delimiter ?? "";
        const selection = JSON.stringify([prefix, delimiter]);
        let after =
            options.cursor === undefined
                ? undefined
                : BucketListing.decodeCursor(options.cursor, "local", selection);
        let startAfter = options.cursor === undefined ? options.startAfter : undefined;
        const end = BucketKey.prefixEnd(prefix);
        const files: BucketFile[] = [];
        const delimitedPrefixes: string[] = [];

        // read metadata only when the caller requests it
        const fields = {
            key: file.key,
            version: file.version,
            etag: file.etag,
            size: file.size,
            uploaded: file.uploaded,
            checksums: file.checksums,
            storageClass: file.storageClass,
            ssecKeyMd5: file.ssecKeyMd5,
            ...(options.include?.includes("httpMetadata")
                ? { httpMetadata: file.httpMetadata }
                : {}),
            ...(options.include?.includes("customMetadata")
                ? { customMetadata: file.customMetadata }
                : {}),
        };

        // seek past each emitted prefix instead of reading every file inside it
        while (true) {
            let lower = gte(file.key, prefix);
            // start after the requested key without skipping a group it names
            if (startAfter !== undefined) {
                lower = gt(file.key, startAfter);
                startAfter = undefined;
            }
            // continue after the cursor's key, or skip the complete group it names
            else if (after !== undefined) {
                const isGroup =
                    delimiter !== "" &&
                    after.indexOf(delimiter, prefix.length) === after.length - delimiter.length;
                // seek past the group, or end after the last possible group
                if (isGroup) {
                    const next = BucketKey.prefixEnd(after);
                    if (next === undefined) {
                        return { files, delimitedPrefixes, truncated: false };
                    }
                    lower = gte(file.key, next);
                }
                // seek past the key
                else {
                    lower = gt(file.key, after);
                }
            }

            // fetch only the remaining page and one lookahead entry
            const upper = end === undefined ? undefined : lt(file.key, end);
            const remaining = limit - files.length - delimitedPrefixes.length;
            const entries = await this.database
                .select(fields)
                .from(file)
                .where(and(gte(file.key, prefix), lower, upper))
                .orderBy(asc(file.key))
                .limit(remaining + 1);
            if (entries.length === 0) {
                return { files, delimitedPrefixes, truncated: false };
            }

            // retain one lookahead entry to determine whether another page exists
            for (const entry of entries) {
                if (files.length + delimitedPrefixes.length === limit) {
                    return {
                        files,
                        delimitedPrefixes,
                        truncated: true,
                        cursor: BucketListing.encodeCursor("local", selection, after!),
                    };
                }
                const index = delimiter === "" ? -1 : entry.key.indexOf(delimiter, prefix.length);
                // emit the group the key falls into, then seek past it
                if (index !== -1) {
                    after = entry.key.slice(0, index + delimiter.length);
                    delimitedPrefixes.push(after);
                    break;
                }
                // emit the file itself
                else {
                    after = entry.key;
                    files.push(
                        LocalFile.describe({
                            ...entry,
                            httpMetadata: entry.httpMetadata ?? {},
                            customMetadata: entry.customMetadata ?? {},
                        }),
                    );
                }
            }
        }
    }

    /** Reject access after disposal. */
    checkOpen(): void {
        if (this.#isClosed) {
            throw new StorageError("CLOSED", "bucket is closed");
        }
    }

    /** Serialize catalogue changes and opening their selected content files. */
    async exclusive<Value>(operation: () => Promise<Value>): Promise<Value> {
        // wait for the previous operation and keep the lock while this one runs
        const previous = this.#pending;
        const next = Promise.withResolvers<void>();
        this.#pending = next.promise;
        await previous;
        try {
            this.checkOpen();

            return await operation();
        } finally {
            next.resolve();
        }
    }
}
