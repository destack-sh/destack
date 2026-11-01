import type { Environment } from "@destack/environment";
import {
    and,
    asc,
    type DatabaseConnection,
    eq,
    gt,
    gte,
    lt,
    lte,
    type DatabaseHandle,
    defineDatabase,
    type SQL,
    type Table,
} from "@destack/db";
import {
    BucketEntry,
    type BucketListOptions,
    BucketKey,
    BucketListing,
    MAX_BATCH_FILES,
} from "../bucket/index.ts";
import { BucketError } from "../error/index.ts";
import { file, part, segment, upload } from "./stack/index.ts";
import { CatalogueFile } from "./file.ts";
import type { Segment } from "./reader.ts";
import { BLOB_TABLES, type BlobStore } from "@destack/db";

/** The file columns a listing reads. */
const LISTING_COLUMNS = {
    key: file.key,
    etag: file.etag,
    size: file.size,
    uploaded: file.uploaded,
    storageClass: file.storageClass,
};

/** Bound the catalogue's row inserts and their SQL parameter counts. */
const REFERENCE_BATCH_SIZE = 500;

/** A catalogue transaction. */
export type CatalogueTransaction = Parameters<Parameters<DatabaseConnection["transaction"]>[0]>[0];

/** The clock, randomness and network a catalogue's writes run on. */
export type CatalogueEnvironment = Pick<Environment, "clock" | "random" | "network">;

/** A bucket's catalogue database, opened by its host only for it, and its blob store. */
export interface Catalogue extends DatabaseHandle {
    /** The blobs the catalogue's segments and parts reference, indexed in its connection. */
    readonly blobs: BlobStore;
    /** The clock, randomness and network the catalogue's writes run on. */
    readonly environment: CatalogueEnvironment;
    /** Sweep blobs while no other process has the catalogue open, answering whether it ran. */
    readonly reclaim: (deletion: () => Promise<void>) => Promise<boolean>;
}

/** A bucket's catalogue and the blobs its files keep. */
export class CatalogueStorage implements AsyncDisposable {
    /** The private catalogue database of a local bucket. */
    static readonly database = defineDatabase({
        name: "catalogue",
        tables: [file, segment, upload, part, ...BLOB_TABLES],
    });

    /** The catalogue, reopened by its host when it gains tables. */
    readonly #catalogue: Catalogue;
    /** Operations that select or replace blobs. */
    #pending: Promise<void> = Promise.resolve();
    /** The writers admitted and not yet finished, which a fence waits for. */
    #writers = 0;
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

    /** The clock, randomness and network the catalogue's writes run on. */
    get environment(): CatalogueEnvironment {
        return this.#catalogue.environment;
    }

    /** The catalogue connection. */
    get database(): DatabaseConnection {
        return this.#catalogue.database;
    }

    /** The bucket's blobs, indexed in the catalogue's connection. */
    get blobs(): BlobStore {
        return this.#catalogue.blobs;
    }

    /** Migrate the catalogue to its tables and some tables beside them. */
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
            if (this.#writers !== 0) {
                throw BucketError.of(
                    "BUSY",
                    "consume or cancel bucket streams before closing storage",
                );
            }
            await this.#expire();

            // release the catalogue
            await this.#catalogue.close();
            this.#isClosed = true;
        });
    }

    /** Expire the uploads past their time and reclaim their parts, as S3 lifecycle rules do. */
    async #expire(): Promise<void> {
        // expire one upload at a time, each bounded by the 10000-part limit
        while (true) {
            const expired = await this.database
                .select({ id: upload.id })
                .from(upload)
                .where(lte(upload.expires, this.environment.clock.now()))
                .limit(1)
                .get();
            if (!expired) {
                break;
            }
            // delete its parts and the upload, which retires their blobs, then collect them
            await this.database.transaction(async (transaction) => {
                await transaction.delete(part).where(eq(part.uploadId, expired.id));
                await transaction.delete(upload).where(eq(upload.id, expired.id));
            });
            await this.blobs.collect();
        }
    }

    /** Admit a writer into an unfenced bucket. */
    async beginWrite(): Promise<void> {
        await this.exclusive(async () => {
            this.checkWritable();
            this.#writers++;
        });
    }

    /** Release a writer, waking a fence waiting for the last one. */
    endWrite(): void {
        this.#writers--;
        if (this.#writers === 0) {
            this.#drained?.resolve();
            this.#drained = undefined;
        }
    }

    /** Refuse new writes while a transfer copies the bucket. */
    async fence(): Promise<boolean> {
        // refuse writes after the catalogue changes queued before the fence
        const isSet = await this.exclusive(async () => {
            const wasFenced = this.#isFenced;
            this.#isFenced = true;

            return !wasFenced;
        });

        // wait for the writers admitted before it
        while (this.#writers > 0) {
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

    /** Delete what interrupted writes left, while no other process has the catalogue open. */
    async sweep(): Promise<boolean> {
        // sweep only while no other process may write a blob, and no writer here runs
        let isSwept = false;
        const isAlone = await this.#catalogue.reclaim(async () => {
            isSwept = await this.exclusive(async () => {
                // expire the uploads past their time, and stop while a writer runs
                await this.#expire();
                if (this.#writers > 0) {
                    return false;
                }

                // delete unfinished writes and the packs no row points into
                await this.blobs.sweep();

                return true;
            });
        });

        return isAlone && isSwept;
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

    /** Publish a file version and its segments. */
    async publish(
        entry: CatalogueFile,
        segments: Segment[],
        transaction: CatalogueTransaction,
    ): Promise<void> {
        // refuse replacing a locked file, then replace the key's file and detach the segments of its previous version
        const previous = await transaction
            .select({ version: file.version, retainUntil: file.retainUntil })
            .from(file)
            .where(eq(file.key, entry.key))
            .get();
        if (previous?.retainUntil != null && previous.retainUntil > this.environment.clock.now()) {
            throw BucketError.of(
                "ACCESS_DENIED",
                `file ${entry.key} is retained until its lock expires`,
            );
        }
        await transaction.insert(file).values(entry).onConflictDoUpdate({
            target: file.key,
            set: entry,
        });
        if (previous !== undefined) {
            await transaction.delete(segment).where(eq(segment.version, previous.version));
        }

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
            // start after the requested key without skipping a group it starts, then after the last key
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
            const entries = await this.#entries(prefix, lower, remaining + 1);
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

    /** Read the files from a lower bound under a prefix in key order. */
    async #entries(prefix: string, lower: SQL, count: number): Promise<ListingEntry[]> {
        // compare keys by their UTF-8 bytes, as every dialect collates text
        const key = file.key;
        const end = BucketKey.prefixEnd(prefix);
        const upper = end === undefined ? undefined : lt(key, end);

        return await this.database
            .select(LISTING_COLUMNS)
            .from(file)
            .where(and(gte(key, prefix), lower, upper))
            .orderBy(asc(key))
            .limit(count);
    }

    /** Reject access after disposal. */
    checkOpen(): void {
        if (this.#isClosed) {
            throw BucketError.of("CLOSED", "bucket is closed");
        }
    }

    /** Refuse a write into a fenced bucket. */
    checkWritable(): void {
        if (this.#isFenced) {
            throw BucketError.of("UNAVAILABLE", "bucket is fenced while a transfer copies it");
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
    readonly files: BucketEntry[];
    /** The groups of keys sharing a prefix up to the delimiter. */
    readonly delimitedPrefixes: string[];
}

/** A listed catalogue row. */
type ListingEntry = Pick<CatalogueFile, keyof typeof LISTING_COLUMNS>;

/** Bound the next listing read after the last key or group. */
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

/** Add listed entries to a page until it fills or a group starts. */
function addEntries(
    page: ListingPage,
    entries: readonly ListingEntry[],
    listing: {
        /** The listed prefix. */
        readonly prefix: string;
        /** The delimiter grouping keys, empty for none. */
        readonly delimiter: string;
        /** The most files and groups a page has. */
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
            page.files.push(BucketEntry.of({ ...entry, uploaded: new Date(entry.uploaded) }));
        }
    }

    return { after, isFull: false };
}
