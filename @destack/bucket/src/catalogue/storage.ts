import { found } from "@destack/schema";
import {
    and,
    asc,
    type DatabaseConnection,
    eq,
    gt,
    gte,
    inArray,
    lt,
    lte,
    type DatabaseHandle,
    type SQL,
    type Table,
} from "@destack/db";
import type { BucketFile, BucketListOptions } from "../bucket/index.ts";
import { BucketKey } from "../bucket/key.ts";
import { BucketListing, MAX_BATCH_FILES } from "../bucket/list.ts";
import { BucketError } from "../error/index.ts";
import { file, part, segment, upload } from "./stack/index.ts";
import { CatalogueFile } from "./file.ts";
import type { Segment } from "./reader.ts";
import type { BlobStore } from "./store.ts";

/** Bound blob reference queries and their SQL parameter counts. */
const REFERENCE_BATCH_SIZE = 500;

/** A catalogue transaction. */
export type CatalogueTransaction = Parameters<Parameters<DatabaseConnection["transaction"]>[0]>[0];

/** A bucket's catalogue database, which its host opened for it alone, and the store of its blobs. */
export interface Catalogue extends DatabaseHandle {
    /** The store of the blobs the catalogue's segments and parts reference. */
    readonly blobs: BlobStore;
}

/**
 * A bucket's catalogue and the blobs its files keep.
 *
 * Blobs are deleted only under the catalogue lock while no writer holds blobs it has not yet published.
 */
export class CatalogueStorage implements AsyncDisposable {
    /** The catalogue, reopened by its host when it gains tables. */
    readonly #catalogue: Catalogue;
    /** Operations that select or replace blobs. */
    #pending: Promise<void> = Promise.resolve();
    /** Open download streams by blob. */
    readonly #readers = new Map<string, number>();
    /** Blobs detached from their keys. */
    readonly retired = new Set<string>();
    /** The number of writers and holds keeping blobs they have not yet published. */
    #holders = 0;
    /** Whether a transfer fenced the bucket, which refuses new writes. */
    #isFenced = false;
    /** Wakes a fence waiting for the last writer to finish. */
    #drained: PromiseWithResolvers<void> | undefined;
    /** Whether the bucket has released its database and lock. */
    #isClosed = false;
    /** Share an in-progress close between concurrent callers. */
    #closing: Promise<void> | undefined;

    /** Retain an opened catalogue and its blobs. */
    constructor(catalogue: Catalogue) {
        this.#catalogue = catalogue;
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

    /** The catalogue connection. */
    get database(): DatabaseConnection {
        return this.#catalogue.database;
    }

    /** The store of the bucket's blobs. */
    get blobs(): BlobStore {
        return this.#catalogue.blobs;
    }

    /** Migrate the catalogue to its tables and some tables beside them, reopening it over all of them. */
    async migrate(beside: readonly Table[]): Promise<void> {
        await this.exclusive(() => this.#catalogue.migrate(beside));
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
            if (this.#holders !== 0 || this.#readers.size !== 0) {
                throw new BucketError(
                    "BUSY",
                    "consume or cancel bucket streams before closing storage",
                );
            }
            await this.collect();

            // release the catalogue
            await this.#catalogue.close();
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

    /** Delete the retired blobs no reader, segment or part keeps, once no writer holds unpublished blobs. */
    async #collectFiles(): Promise<void> {
        // keep every blob while a writer may share one it has not yet published
        if (this.#holders > 0) {
            return;
        }

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

    /** Admit a writer into an unfenced bucket, keeping blobs from closure and deletion while it writes. */
    async beginWrite(): Promise<void> {
        await this.exclusive(async () => {
            this.checkWritable();
            await this.collect();
            this.#holders++;
        });
    }

    /** Release a writer, waking a fence waiting for the last one. */
    endWrite(): void {
        this.#holders--;
        if (this.#holders === 0) {
            this.#drained?.resolve();
            this.#drained = undefined;
        }
    }

    /** Hold every blob from deletion until released, while a copy fetches blobs before its rows. */
    async hold(): Promise<AsyncDisposable> {
        await this.exclusive(async () => {
            await this.collect();
            this.#holders++;
        });

        return {
            [Symbol.asyncDispose]: async () => {
                // release the hold, then reclaim what it kept
                this.endWrite();
                await this.exclusive(() => this.collect());
            },
        };
    }

    /** Refuse new writes while a transfer copies the bucket, returning once the writers and holds in flight finished, and report whether this call set the fence. */
    async fence(): Promise<boolean> {
        // refuse writes after the catalogue changes queued before the fence
        const isSet = await this.exclusive(async () => {
            const wasFenced = this.#isFenced;
            this.#isFenced = true;

            return !wasFenced;
        });

        // wait for the writers admitted before it
        while (this.#holders > 0) {
            this.#drained ??= Promise.withResolvers();
            await this.#drained.promise;
        }

        return isSet;
    }

    /** Accept writes again. */
    async lift(): Promise<void> {
        await this.exclusive(async () => {
            this.#isFenced = false;
        });
    }

    /** Retire the blobs that deleted rows referenced, deleting those nothing references. */
    async retire(digests: readonly string[]): Promise<void> {
        await this.exclusive(async () => {
            for (const digest of digests) {
                this.retired.add(digest);
            }
            await this.collect();
        });
    }

    /**
     * Delete what interrupted writes left: unfinished blob writes and blobs no segment or part references.
     *
     * The sweep stops and answers false while a writer holds unpublished blobs, so its host sweeps again later.
     */
    async sweep(): Promise<boolean> {
        // clean the catalogue and the store while no writer runs
        const isIdle = await this.exclusive(async () => {
            // reclaim expired uploads, and stop while a writer runs
            await this.collect();
            if (this.#holders > 0) {
                return false;
            }

            // delete the unfinished writes
            await this.blobs.clean();

            return true;
        });
        if (!isIdle) {
            return false;
        }

        // read the blobs in bounded batches, deleting each batch's unreferenced ones while no writer runs
        let batch: string[] = [];
        for await (const digest of this.blobs.digests()) {
            batch.push(digest);
            if (batch.length === REFERENCE_BATCH_SIZE) {
                if (!(await this.#sweep(batch))) {
                    return false;
                }
                batch = [];
            }
        }

        return batch.length === 0 || (await this.#sweep(batch));
    }

    /** Delete the blobs of a batch no segment, part or reader keeps, answering false while a writer holds unpublished blobs. */
    async #sweep(digests: string[]): Promise<boolean> {
        return await this.exclusive(async () => {
            // keep every blob while a writer may share one it has not yet published
            if (this.#holders > 0) {
                return false;
            }

            // delete the kept blobs nothing references, which collection may have deleted since listing
            const referenced = await this.#referenced(digests);
            const unreferenced = digests.filter(
                (digest) => !referenced.has(digest) && !this.#readers.has(digest),
            );
            const missing = new Set(await this.blobs.missing(unreferenced));
            for (const digest of unreferenced) {
                if (!missing.has(digest)) {
                    await this.blobs.delete(digest);
                }
                this.retired.delete(digest);
            }

            return true;
        });
    }

    /** Read the catalogue entry of a key while the caller has the catalogue lock. */
    async entry(key: string): Promise<CatalogueFile | undefined> {
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
        entry: CatalogueFile,
        segments: Segment[],
        transaction: CatalogueTransaction,
    ): Promise<string[]> {
        // refuse replacing a locked file, then replace the key's file and detach the segments of its previous version
        const previous = await transaction
            .select({ version: file.version, retainUntil: file.retainUntil })
            .from(file)
            .where(eq(file.key, entry.key))
            .get();
        if (previous?.retainUntil != null && previous.retainUntil > Date.now()) {
            throw new BucketError("LOCKED", `file ${entry.key} is retained until its lock expires`);
        }
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
                : BucketListing.decodeCursor(options.cursor, "catalogue", selection);
        let startAfter = options.cursor === undefined ? options.startAfter : undefined;
        const page: ListingPage = { files: [], delimitedPrefixes: [] };

        // seek past each emitted prefix instead of reading every file inside it
        while (true) {
            // start after the requested key without skipping a group it names, then after the last key
            const lower =
                startAfter === undefined
                    ? lowerBound(prefix, delimiter, after)
                    : gt(file.key, startAfter);
            startAfter = undefined;
            if (lower === undefined) {
                return { ...page, truncated: false };
            }

            // fetch only the remaining page and one lookahead entry
            const remaining = limit - page.files.length - page.delimitedPrefixes.length;
            const entries = await this.#entries(options, prefix, lower, remaining + 1);
            if (entries.length === 0) {
                return { ...page, truncated: false };
            }

            // retain one lookahead entry to determine whether another page exists
            const added = addEntries(page, entries, { prefix, delimiter, limit, after });
            after = added.after;
            if (added.isFull) {
                return truncated(page, selection, after);
            }
        }
    }

    /** Read the files from a lower bound under a prefix in key order, with the metadata the caller requests. */
    async #entries(
        options: BucketListOptions,
        prefix: string,
        lower: SQL,
        count: number,
    ): Promise<ListingEntry[]> {
        // compare keys by their UTF-8 bytes, as every dialect collates text
        const key = file.key;
        const end = BucketKey.prefixEnd(prefix);
        const upper = end === undefined ? undefined : lt(key, end);

        return await this.database
            .select(listingColumns(options))
            .from(file)
            .where(and(gte(key, prefix), lower, upper))
            .orderBy(asc(key))
            .limit(count);
    }

    /** Reject access after disposal. */
    checkOpen(): void {
        if (this.#isClosed) {
            throw new BucketError("CLOSED", "bucket is closed");
        }
    }

    /** Refuse a write into a fenced bucket. */
    checkWritable(): void {
        if (this.#isFenced) {
            throw new BucketError("FENCED", "bucket is fenced while a transfer copies it");
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

/** A listing page as it fills. */
interface ListingPage {
    /** The listed files. */
    readonly files: BucketFile[];
    /** The groups of keys sharing a prefix up to the delimiter. */
    readonly delimitedPrefixes: string[];
}

/** A listed catalogue entry, with its metadata absent unless requested. */
type ListingEntry = Omit<CatalogueFile, "httpMetadata" | "customMetadata"> &
    Partial<Pick<CatalogueFile, "httpMetadata" | "customMetadata">>;

/** Select the listed file columns, reading metadata only when the caller requests it. */
function listingColumns(options: BucketListOptions) {
    return {
        key: file.key,
        version: file.version,
        etag: file.etag,
        size: file.size,
        uploaded: file.uploaded,
        checksums: file.checksums,
        storageClass: file.storageClass,
        ssecKeyMd5: file.ssecKeyMd5,
        retainUntil: file.retainUntil,
        ...(options.include?.includes("httpMetadata") === true
            ? { httpMetadata: file.httpMetadata }
            : {}),
        ...(options.include?.includes("customMetadata") === true
            ? { customMetadata: file.customMetadata }
            : {}),
    };
}

/** Bound the next listing read after the last key or group, absent once no key can follow the last group. */
function lowerBound(prefix: string, delimiter: string, after: string | undefined): SQL | undefined {
    // start at the prefix
    if (after === undefined) {
        return gte(file.key, prefix);
    }

    // seek past a group, or end after the last possible group
    const isGroup =
        delimiter !== "" &&
        after.indexOf(delimiter, prefix.length) === after.length - delimiter.length;
    if (isGroup) {
        const next = BucketKey.prefixEnd(after);

        return next === undefined ? undefined : gte(file.key, next);
    }

    // seek past the key
    return gt(file.key, after);
}

/** Finish a full listing page with the cursor continuing after its last key or group. */
function truncated(page: ListingPage, selection: string, after: string | undefined): BucketListing {
    // require the last key of a full page
    if (after === undefined) {
        throw new TypeError("a full listing page has no last key");
    }

    return {
        ...page,
        truncated: true,
        cursor: BucketListing.encodeCursor("catalogue", selection, after),
    };
}

/** Add listed entries to a page until it fills or a group starts, returning the last key or group added. */
function addEntries(
    page: ListingPage,
    entries: readonly ListingEntry[],
    listing: {
        /** The listed prefix. */
        readonly prefix: string;
        /** The delimiter grouping keys, empty for none. */
        readonly delimiter: string;
        /** The most files and groups a page holds. */
        readonly limit: number;
        /** The last key or group listed before. */
        readonly after: string | undefined;
    },
): { readonly after: string | undefined; readonly isFull: boolean } {
    // add each entry after the last one listed
    const { prefix, delimiter, limit } = listing;
    let after = listing.after;
    for (const entry of entries) {
        // stop at an entry beyond a full page
        if (page.files.length + page.delimitedPrefixes.length === limit) {
            return { after, isFull: true };
        }
        const index = delimiter === "" ? -1 : entry.key.indexOf(delimiter, prefix.length);
        // emit the group the key falls into, then seek past it
        if (index !== -1) {
            after = entry.key.slice(0, index + delimiter.length);
            page.delimitedPrefixes.push(after);
            break;
        }
        // emit the file itself
        else {
            after = entry.key;
            page.files.push(
                CatalogueFile.describe({
                    ...entry,
                    httpMetadata: entry.httpMetadata ?? {},
                    customMetadata: entry.customMetadata ?? {},
                }),
            );
        }
    }

    return { after, isFull: false };
}
