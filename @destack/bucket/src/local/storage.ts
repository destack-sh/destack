import { found } from "@destack/schema";
import { mkdir, opendir, stat, unlink } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { and, asc, eq, gt, gte, inArray, lt, lte, type Table } from "@destack/db";
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
import { Digest } from "@destack/schema";
import { LocalBlobStore } from "@destack/db/blob/local";

/** Bound blob reference queries and their SQL parameter counts. */
const REFERENCE_BATCH_SIZE = 500;

/** A catalogue transaction. */
export type CatalogueTransaction = Parameters<Parameters<SqliteDatabase["transaction"]>[0]>[0];

/** A bucket's catalogue, the blobs its files keep, and the exclusive host lock. */
export class LocalStorage implements AsyncDisposable {
    /** The bucket directory. */
    readonly directory: string;
    /** The private catalogue connection, reopened when the catalogue gains tables. */
    #database: SqliteDatabase;
    /** The blobs of the bucket's files and parts. */
    readonly blobs: LocalBlobStore;
    /** The operating-system lock this host has taken. */
    readonly #lock: FileLock;
    /** Operations that select or replace blobs. */
    #pending: Promise<void> = Promise.resolve();
    /** Open download streams by blob. */
    readonly #readers = new Map<string, number>();
    /** Blobs detached from their keys. */
    readonly retired = new Set<string>();
    /** The number of uploads writing blobs they have not yet published. */
    uploads = 0;
    /** Whether the bucket has released its database and lock. */
    #isClosed = false;
    /** Share an in-progress close between concurrent callers. */
    #closing: Promise<void> | undefined;

    /** Retain an opened catalogue and its blobs. */
    private constructor(
        directory: string,
        database: SqliteDatabase,
        blobs: LocalBlobStore,
        lock: FileLock,
    ) {
        // keep the opened catalogue, blobs and lock
        this.directory = directory;
        this.#database = database;
        this.blobs = blobs;
        this.#lock = lock;
    }

    /** Open or create a bucket without contacting a remote service. */
    static async open(directory: string, scope?: string): Promise<LocalStorage> {
        // refuse creating a bucket without the scope it belongs to
        directory = resolve(directory);
        const catalogue = join(directory, "bucket.db");
        const isNew = await stat(catalogue).then(
            () => false,
            (error: NodeJS.ErrnoException) => {
                if (error.code !== "ENOENT") {
                    throw error;
                }

                return true;
            },
        );
        if (isNew && scope === undefined) {
            throw new StorageError("NO_SUCH_BUCKET", `no bucket at ${directory}`);
        }

        // create the blob directory before publishing any catalogue entries
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
            database = await connect(catalogue, catalogueDatabase);
            if (isNew) {
                await database.log.create(scope);
            }
            await database.migrate(catalogueDatabase.tables);

            // persist the initial catalogue and lock entries before accepting writes
            await syncDirectory(directory);

            // reclaim files left by interrupted uploads or a terminated host
            const blobs = await LocalBlobStore.open(contents);
            const bucket = new LocalStorage(directory, database, blobs, lock);
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
                throw new AggregateError(failures, "storage initialization and closure failed", {
                    cause: error,
                });
            }
            throw error;
        }
    }

    /** Recover unreferenced blobs and interrupted writes with bounded directory and catalogue reads. */
    async #recover(): Promise<void> {
        // collect blob digests in bounded batches
        const directory = await opendir(this.blobs.directory);
        let digests: string[] = [];
        for await (const entry of directory) {
            if (!entry.isFile()) {
                continue;
            }
            // delete a write a terminated host left behind
            if (entry.name.startsWith(".")) {
                await unlink(join(this.blobs.directory, entry.name));
            }
            // check a blob's references
            else if (Digest.safeParse(entry.name).success) {
                digests.push(entry.name);
                if (digests.length === REFERENCE_BATCH_SIZE) {
                    await this.#deleteUnreferenced(digests);
                    digests = [];
                }
            }
        }
        if (digests.length) {
            await this.#deleteUnreferenced(digests);
        }
    }

    /** Delete the blobs no segment or part references, at most one batch of them. */
    async #deleteUnreferenced(digests: string[]): Promise<void> {
        const referenced = await this.#referenced(digests);
        for (const digest of digests) {
            if (!referenced.has(digest)) {
                await this.blobs.delete(digest);
            }
        }
    }

    /** Select the blobs a segment or a part references, at most one batch of them. */
    async #referenced(digests: string[]): Promise<Set<string>> {
        const segments = await this.database
            .select({ blob: segment.blob })
            .from(segment)
            .where(inArray(segment.blob, digests));
        const parts = await this.database
            .select({ blob: part.blob })
            .from(part)
            .where(inArray(part.blob, digests));

        return new Set([...segments, ...parts].map((entry) => entry.blob));
    }

    /** The private catalogue connection. */
    get database(): SqliteDatabase {
        return this.#database;
    }

    /** Migrate the catalogue to its tables and some tables beside them, then reopen it over all of them. */
    async migrate(beside: readonly Table[]): Promise<void> {
        await this.exclusive(async () => {
            // migrate, then reopen the connection declaring every table
            const tables = [...catalogueDatabase.tables, ...beside];
            await this.#database.migrate(tables);
            await this.#database.close();
            this.#database = await connect(join(this.directory, "bucket.db"), tables);
        });
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

    /** Reclaim detached blobs after their last reader closes. */
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
                    .returning({ blob: part.blob });
                await transaction.delete(upload).where(eq(upload.id, expired.id));

                return entries;
            });
            for (const entry of removed) {
                this.retired.add(entry.blob);
            }
            await this.#collectFiles();
        }

        await this.#collectFiles();
    }

    /** Delete retired blobs no reader has open and no segment or part still references. */
    async #collectFiles(): Promise<void> {
        // keep every blob an active reader retains
        const released = [...this.retired].filter((digest) => !this.#readers.has(digest));
        for (let start = 0; start < released.length; start += REFERENCE_BATCH_SIZE) {
            // keep blobs a copy, another file or a completed upload still shares, which retire again with their last reference
            const batch = released.slice(start, start + REFERENCE_BATCH_SIZE);
            const referenced = await this.#referenced(batch);
            for (const digest of batch) {
                if (!referenced.has(digest)) {
                    await this.blobs.delete(digest);
                }
                this.retired.delete(digest);
            }
        }
    }

    /** Retain a blob until its reader finishes. */
    retain(digest: string): void {
        const count = this.#readers.get(digest);
        this.#readers.set(digest, count === undefined ? 1 : count + 1);
    }

    /** Release a retained blob. */
    release(digest: string): void {
        const remaining = found(this.#readers, digest) - 1;
        if (remaining === 0) {
            this.#readers.delete(digest);
        } else {
            this.#readers.set(digest, remaining);
        }
    }

    /** Reclaim earlier writes and prevent closure while an upload writes its blobs. */
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

    /** Read the segments of a file version in order while the caller has the catalogue lock. */
    async segments(version: string): Promise<Segment[]> {
        return await this.database
            .select({ blob: segment.blob, nonce: segment.nonce, size: segment.size })
            .from(segment)
            .where(eq(segment.version, version))
            .orderBy(asc(segment.position));
    }

    /** Publish a file version and its segments, returning the blobs the replaced version detaches. */
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
                      .returning({ blob: segment.blob });

        // write the new version's segments in bounded batches
        const rows = segments.map((selected, position) => ({
            version: entry.version,
            position,
            blob: selected.blob,
            nonce: selected.nonce,
            size: selected.size,
        }));
        for (let start = 0; start < rows.length; start += REFERENCE_BATCH_SIZE) {
            await transaction
                .insert(segment)
                .values(rows.slice(start, start + REFERENCE_BATCH_SIZE));
        }

        return detached.map((selected) => selected.blob);
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
            ...(options.include?.includes("httpMetadata") === true
                ? { httpMetadata: file.httpMetadata }
                : {}),
            ...(options.include?.includes("customMetadata") === true
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
                    if (after === undefined) {
                        throw new TypeError("a full listing page has no last key");
                    }

                    return {
                        files,
                        delimitedPrefixes,
                        truncated: true,
                        cursor: BucketListing.encodeCursor("local", selection, after),
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

    /** Serialize catalogue changes and opening their selected blobs. */
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
