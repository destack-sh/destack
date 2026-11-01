import type { Environment } from "@destack/environment";
import { asc, eq, gt, inArray } from "../sql/operator.ts";
import { sql, type SQL } from "../sql/sql.ts";
import { Column } from "../table/column.ts";
import { TABLE, type Table } from "../table/table.ts";
import { Condition } from "../query/condition.ts";
import { textValues } from "../query/statement.ts";
import type { DatabaseConnection } from "../database/connection.ts";
import { defineTable } from "../table/table.ts";
import { index } from "../table/constraint.ts";
import { boolean, integer, text } from "../table/column.ts";
import type { BlobEntry, BlobPlacement, BlobStorage, ContentStore } from "@destack/resource";
import { canonicalize, Digest, type JsonValue, present, schema } from "@destack/schema";

/** How long a store leases what it writes for a row to reference, by default: half an hour. */
const LEASE_MILLISECONDS = 30 * 60_000;

/** How long an unreferenced blob stays before deletion: an hour, so reads in flight finish. */
const GRACE_MILLISECONDS = 60 * 60_000;

/** The bytes a fetch gathers into one pack: 64 MiB, about 13 of R2's 5 MiB parts. */
const PACK_BYTES = 64 * 1024 * 1024;

/** The digests one statement binds: 500, far under SQLite's 32,766 parameters. */
const BATCH_DIGESTS = 500;

/** A row of one blob, as a raw statement answers it. */
const DigestRow = schema.object({ digest: Digest });

/** The storage, database and environment of a partition's blob store. */
export interface BlobStoreOptions<Storage extends BlobStorage = BlobStorage> {
    /** The storage of the blobs' packs. */
    readonly storage: Storage;
    /** The partition's database, which declares the blob tables beside its own. */
    readonly database: DatabaseConnection;
    /** The clock leases end on. */
    readonly environment: Pick<Environment, "clock">;
    /** How long a write's lease lasts for a row to reference it, half an hour by default. */
    readonly leaseMilliseconds?: number;
}

/** A blob to store: its digest, its size, and how to read its bytes once the store lacks it. */
export interface BlobUpload {
    /** The blob's digest. */
    readonly digest: Digest;
    /** Its size in bytes. */
    readonly size: number;
    /** Read its bytes. */
    readonly open: () => Promise<AsyncIterable<Uint8Array>>;
}

/** A partition's blobs in packs, each alive while a row's blob column or a lease references it. */
export class BlobStore<Storage extends BlobStorage = BlobStorage> implements ContentStore {
    /** The blobs a partition stores: where each one's bytes lie, and what decides its lifetime. */
    static readonly table = defineTable(
        "blob",
        {
            /** The blob's digest. */
            digest: text("digest").validate(Digest).primaryKey(),
            /** The key of its bytes' pack, its own digest for a blob stored alone. */
            pack: text("pack").notNull(),
            /** Where its bytes start in the pack. */
            offset: integer("offset").notNull(),
            /** Its size in bytes. */
            size: integer("size").notNull(),
            /** When its lease ends, in UTC epoch milliseconds, zero once it ended. */
            leasedUntil: integer("leased_until").notNull(),
            /** Whether its last reference may have gone, which a collection checks. */
            isRetired: boolean("is_retired").notNull(),
            /** When a collection found it free of references, absent while referenced or unchecked. */
            freedAt: integer("freed_at"),
        },
        {
            constraints: (table) => [
                index("blob_pack").on(table.pack),
                index("blob_leased_until").on(table.leasedUntil),
                index("blob_retired").on(table.isRetired),
            ],
        },
    );

    /** The storage of the blobs' packs. */
    readonly storage: Storage;
    /** The partition's database, with the row of each blob. */
    readonly database: DatabaseConnection;
    /** The clock leases end on. */
    readonly environment: Pick<Environment, "clock">;
    /** How long a write's lease lasts, in milliseconds. */
    readonly leaseMilliseconds: number;

    /** Store a partition's blobs in a storage, indexed and referenced from its database. */
    constructor(options: BlobStoreOptions<Storage>) {
        // store the storage, database, environment and lease length
        this.storage = options.storage;
        this.database = options.database;
        this.environment = options.environment;
        this.leaseMilliseconds = options.leaseMilliseconds ?? LEASE_MILLISECONDS;
    }

    /** Lease the stored blobs for a row to reference, and store the missing ones in one pack. */
    async put(
        uploads: readonly BlobUpload[],
        until: number = this.environment.clock.now() + this.leaseMilliseconds,
    ): Promise<void> {
        // lease each blob once, the stored ones by their rows
        const unique = [...new Map(uploads.map((upload) => [upload.digest, upload])).values()];
        await this.lease(
            unique.map((upload) => upload.digest),
            until,
        );

        // store the missing ones in one pack, each body opened as the pack reaches it
        const missing = new Set(await this.missing(unique.map((upload) => upload.digest)));
        const entries: BlobEntry[] = unique
            .filter((upload) => missing.has(upload.digest))
            .map(({ digest, size, open }) => ({
                digest,
                size,
                body: (async function* () {
                    yield* await open();
                })(),
            }));
        if (entries.length > 0) {
            await this.record(await this.storage.write(entries), until);
        }
    }

    /** Stream bytes into a blob before a row refers to it, refusing an unexpected digest. */
    async write(body: AsyncIterable<Uint8Array>, expected?: Digest): Promise<Digest> {
        // store them, then record where they lie under a lease
        const until = this.environment.clock.now() + this.leaseMilliseconds;
        const placements = await this.storage.write([
            expected === undefined ? { body } : { body, digest: expected },
        ]);
        await this.record(placements, until);

        return present(placements[0], "the written blob's placement").digest;
    }

    /** Read a blob's bytes. */
    read(digest: Digest): AsyncIterable<Uint8Array> {
        return this.slice(digest, 0, Number.POSITIVE_INFINITY);
    }

    /** Read a range of a blob's bytes from its pack. */
    async *slice(digest: Digest, offset: number, length: number): AsyncIterable<Uint8Array> {
        const placement = present((await this.#placements([digest]))[0], `blob ${digest}`);
        const end = Math.min(placement.size, offset + length);
        yield* this.storage.slice(
            placement.pack,
            placement.offset + offset,
            Math.max(0, end - offset),
        );
    }

    /** Read a blob's whole bytes. */
    bytes(digest: Digest): Promise<Uint8Array<ArrayBuffer>> {
        return gathered(this.read(digest));
    }

    /** Read many blobs in order, each body read before the next blob's. */
    async *batch(digests: readonly Digest[]): AsyncIterable<BlobEntry> {
        const placements = new Map(
            (await this.#placements(digests)).map((placement) => [placement.digest, placement]),
        );
        for (const digest of digests) {
            const { pack, offset, size } = present(placements.get(digest), `blob ${digest}`);
            yield { digest, size, body: this.storage.slice(pack, offset, size) };
        }
    }

    /** Store the blobs this store lacks among some digests, read from another store. */
    async fetch(digests: readonly Digest[], source: Pick<ContentStore, "batch">): Promise<void> {
        // gather the missing blobs in order, writing a pack each time they reach a pack's bytes
        const missing = await this.missing([...new Set(digests)]);
        let gathering: BlobEntry[] = [];
        let bytes = 0;
        for await (const entry of source.batch(missing)) {
            gathering.push({ ...entry, body: chunks(await gathered(entry.body)) });
            bytes += entry.size;
            if (bytes >= PACK_BYTES) {
                await this.#writePack(gathering);
                gathering = [];
                bytes = 0;
            }
        }

        // write the rest
        if (gathering.length > 0) {
            await this.#writePack(gathering);
        }
    }

    /** Write blobs as one pack, recorded under a write's lease. */
    async #writePack(entries: readonly BlobEntry[]): Promise<void> {
        const until = this.environment.clock.now() + this.leaseMilliseconds;
        await this.record(await this.storage.write(entries), until);
    }

    /** Count the bytes the store's blobs take, or some of them. */
    async size(digests?: readonly Digest[]): Promise<number> {
        // sum every blob's size, or the given blobs', a batch at a time
        const batches =
            digests === undefined
                ? [undefined]
                : Array.from({ length: Math.ceil(digests.length / BATCH_DIGESTS) }, (_, index) =>
                      digests.slice(index * BATCH_DIGESTS, (index + 1) * BATCH_DIGESTS),
                  );
        let bytes = 0;
        for (const batch of batches) {
            const [total] = await this.database
                .select({ bytes: sql<number | string>`COALESCE(SUM(${BlobStore.table.size}), 0)` })
                .from(BlobStore.table)
                .where(
                    batch === undefined ? undefined : inArray(BlobStore.table.digest, [...batch]),
                );
            bytes += Number(present(total, "the blobs' total").bytes);
        }

        return bytes;
    }

    /** List the digests the store lacks, in the given order. */
    async missing(digests: readonly Digest[]): Promise<Digest[]> {
        const stored = new Set((await this.#placements(digests)).map(({ digest }) => digest));

        return digests.filter((digest) => !stored.has(digest));
    }

    /** Write a JSON value's canonical form as a blob before a row refers to it. */
    writeJson(value: JsonValue): Promise<Digest> {
        return this.write(chunks(new TextEncoder().encode(canonicalize(value))));
    }

    /** Read a JSON blob's value, parsed once by the schema its writer wrote it with. */
    async readJson<Value>(digest: Digest, written: schema.Schema<Value>): Promise<Value> {
        const text = new TextDecoder().decode(await this.bytes(digest));

        return written.parse(JSON.parse(text));
    }

    /** Lease stored blobs until a moment, rescuing any retired one. */
    async lease(
        digests: readonly Digest[],
        until: number = this.environment.clock.now() + this.leaseMilliseconds,
    ): Promise<void> {
        await this.database.transaction(async (transaction) => {
            for (let start = 0; start < digests.length; start += BATCH_DIGESTS) {
                // lease the batch, leaving a later lease, and take it out of retirement
                const batch = [...new Set(digests.slice(start, start + BATCH_DIGESTS))];
                await transaction
                    .update(BlobStore.table)
                    .set({
                        leasedUntil: sql`CASE WHEN ${BlobStore.table.leasedUntil} > ${until}
                            THEN ${BlobStore.table.leasedUntil} ELSE ${until} END`,
                        isRetired: false,
                        freedAt: null,
                    })
                    .where(inArray(BlobStore.table.digest, batch));
            }
        });
    }

    /** Retire blobs whose rows went without a mutation retiring them, for a collection to check. */
    async retire(digests: readonly Digest[]): Promise<void> {
        for (let start = 0; start < digests.length; start += BATCH_DIGESTS) {
            const batch = [...new Set(digests.slice(start, start + BATCH_DIGESTS))];
            await this.database
                .update(BlobStore.table)
                .set({ isRetired: true })
                .where(inArray(BlobStore.table.digest, batch));
        }
    }

    /** End the leases of some blobs, for a collection to delete those no row references. */
    async release(digests: readonly Digest[]): Promise<void> {
        for (let start = 0; start < digests.length; start += BATCH_DIGESTS) {
            const batch = [...new Set(digests.slice(start, start + BATCH_DIGESTS))];
            await this.database
                .update(BlobStore.table)
                .set({ leasedUntil: 0, isRetired: true })
                .where(inArray(BlobStore.table.digest, batch));
        }
    }

    /** List the blobs whose lease ends first, as one write or use leased them together. */
    async oldestLeased(): Promise<Digest[]> {
        // read the earliest live lease end, then every blob it leases
        const now = this.environment.clock.now();
        const [first] = await this.database
            .select({ until: BlobStore.table.leasedUntil })
            .from(BlobStore.table)
            .where(gt(BlobStore.table.leasedUntil, now))
            .orderBy(asc(BlobStore.table.leasedUntil))
            .limit(1);
        if (first === undefined) {
            return [];
        }
        const rows = await this.database
            .select({ digest: BlobStore.table.digest })
            .from(BlobStore.table)
            .where(eq(BlobStore.table.leasedUntil, first.until));

        return rows.map(({ digest }) => digest);
    }

    /** Delete retired blobs a grace after no row or lease referenced them. */
    async collect(grace: number = GRACE_MILLISECONDS): Promise<number | undefined> {
        // retire the blobs of the ended leases, writing only when one ended
        const now = this.environment.clock.now();
        const [ending] = await this.database
            .select({ digest: BlobStore.table.digest })
            .from(BlobStore.table)
            .where(
                sql`${BlobStore.table.leasedUntil} > 0 AND ${BlobStore.table.leasedUntil} <= ${now}`,
            )
            .limit(1);
        if (ending !== undefined) {
            await this.database
                .update(BlobStore.table)
                .set({ leasedUntil: 0, isRetired: true })
                .where(
                    sql`${BlobStore.table.leasedUntil} > 0 AND ${BlobStore.table.leasedUntil} <= ${now}`,
                );
        }

        // delete retired blobs until none is left, as deleting one's cascading rows retires what they kept
        const cascading = this.#blobColumns("cascade");
        for (
            let retired = await this.#retired(now, grace);
            retired.length > 0;
            retired = await this.#retired(now, grace)
        ) {
            for (let start = 0; start < retired.length; start += BATCH_DIGESTS) {
                const batch = retired.slice(start, start + BATCH_DIGESTS);
                await this.#delete(batch, cascading, now, grace);
            }
        }

        // look again once the next lease or grace ends
        return this.#nextCollection(now, grace);
    }

    /** Read when a collection next has work: the next lease or grace to end. */
    async #nextCollection(after: number, grace: number): Promise<number | undefined> {
        // read the first lease to end, and the first unreferenced blob's grace
        const [lease] = await this.database
            .select({ until: BlobStore.table.leasedUntil })
            .from(BlobStore.table)
            .where(gt(BlobStore.table.leasedUntil, after))
            .orderBy(asc(BlobStore.table.leasedUntil))
            .limit(1);
        const [freed] = await this.database
            .select({ at: BlobStore.table.freedAt })
            .from(BlobStore.table)
            .where(sql`${BlobStore.table.isRetired} AND ${BlobStore.table.freedAt} IS NOT NULL`)
            .orderBy(asc(BlobStore.table.freedAt))
            .limit(1);

        // wake at the earlier of the two
        const ends = [
            ...(lease === undefined ? [] : [lease.until]),
            ...(freed?.at === undefined || freed.at === null ? [] : [freed.at + grace]),
        ];

        return ends.length === 0 ? undefined : Math.min(...ends);
    }

    /** Delete unfinished writes and the packs no row points into, while no writer runs. */
    async sweep(): Promise<void> {
        await this.storage.clean();
        for await (const pack of this.storage.packs()) {
            await this.#deletePackUnlessUsed(this.database, pack);
        }
    }

    /** Delete every blob and pack, as a partition leaving its host does. */
    async clear(): Promise<void> {
        for await (const pack of this.storage.packs()) {
            await this.storage.delete(pack);
        }
        await this.database.delete(BlobStore.table);
    }

    /** Record where blobs a storage stored lie, leased, deleting a pack no row points into. */
    async record(placements: readonly BlobPlacement[], until: number): Promise<void> {
        // record each blob unless another writer recorded it first, then lease them all
        await this.database.transaction(async (transaction) => {
            for (let start = 0; start < placements.length; start += BATCH_DIGESTS) {
                const batch = placements.slice(start, start + BATCH_DIGESTS);
                await transaction
                    .insert(BlobStore.table)
                    .values(
                        batch.map((placement) => ({
                            ...placement,
                            leasedUntil: until,
                            isRetired: false,
                        })),
                    )
                    .onConflictDoNothing();
            }
        });
        await this.lease(
            placements.map(({ digest }) => digest),
            until,
        );

        // delete each pack no row points into
        for (const pack of new Set(placements.map((placement) => placement.pack))) {
            await this.#deletePackUnlessUsed(this.database, pack);
        }
    }

    /** Read the rows of some blobs. */
    async #placements(digests: readonly Digest[]): Promise<BlobPlacement[]> {
        const found: BlobPlacement[] = [];
        for (let start = 0; start < digests.length; start += BATCH_DIGESTS) {
            const batch = [...new Set(digests.slice(start, start + BATCH_DIGESTS))];
            found.push(
                ...(await this.database
                    .select({
                        digest: BlobStore.table.digest,
                        pack: BlobStore.table.pack,
                        offset: BlobStore.table.offset,
                        size: BlobStore.table.size,
                    })
                    .from(BlobStore.table)
                    .where(inArray(BlobStore.table.digest, batch))),
            );
        }

        return found;
    }

    /** List the retired blobs a collection acts on: unchecked ones, and those past their grace. */
    async #retired(now: number, grace: number): Promise<Digest[]> {
        const rows = await this.database
            .select({ digest: BlobStore.table.digest })
            .from(BlobStore.table)
            .where(
                sql`${BlobStore.table.isRetired} AND (${BlobStore.table.freedAt} IS NULL
                    OR ${BlobStore.table.freedAt} <= ${now - grace})`,
            );

        return rows.map(({ digest }) => digest);
    }

    /** Delete each of some retired blobs still unreferenced, with the rows cascading from it. */
    async #delete(
        batch: readonly Digest[],
        cascading: readonly { readonly table: Table; readonly column: Column }[],
        now: number,
        grace: number,
    ): Promise<void> {
        await this.database.transaction(async (transaction) => {
            // read the batch's retired rows, and which of them no blob column or live lease references
            const taken = await transaction
                .select({ digest: BlobStore.table.digest, freedAt: BlobStore.table.freedAt })
                .from(BlobStore.table)
                .where(
                    sql`${BlobStore.table.isRetired} AND ${inArray(BlobStore.table.digest, batch)}`,
                );
            const unreferenced = new Set(await this.#unreferenced(transaction, taken, now));

            // split them: referenced, newly free with a grace to wait, or due once free past the grace
            const referenced: Digest[] = [];
            const freed: Digest[] = [];
            const due: Digest[] = [];
            for (const { digest, freedAt } of taken) {
                // take a referenced blob out of retirement
                if (!unreferenced.has(digest)) {
                    referenced.push(digest);
                }
                // free a blob found unreferenced now, which waits a grace
                else if (freedAt === null && grace > 0) {
                    freed.push(digest);
                }
                // delete a blob free past its grace, or at once without a grace
                else if ((freedAt ?? now) <= now - grace) {
                    due.push(digest);
                }
            }
            await transaction
                .update(BlobStore.table)
                .set({ isRetired: false, freedAt: null })
                .where(inArray(BlobStore.table.digest, referenced));
            await transaction
                .update(BlobStore.table)
                .set({ freedAt: now })
                .where(inArray(BlobStore.table.digest, freed));

            // delete the due ones with the rows cascading from them, which retires the blobs those rows referenced
            if (due.length === 0) {
                return;
            }
            for (const { table, column } of cascading) {
                await transaction.delete(table).where(inArray(column, due));
            }
            const deleted = await transaction
                .delete(BlobStore.table)
                .where(inArray(BlobStore.table.digest, due))
                .returning({ pack: BlobStore.table.pack });

            // delete each pack no row points into any more
            for (const pack of new Set(deleted.map((row) => row.pack))) {
                await this.#deletePackUnlessUsed(transaction, pack);
            }
        });
    }

    /** Delete a pack unless a row points into it. */
    async #deletePackUnlessUsed(database: DatabaseConnection, pack: string): Promise<void> {
        const [used] = await database
            .select({ digest: BlobStore.table.digest })
            .from(BlobStore.table)
            .where(sql`${BlobStore.table.pack} = ${pack}`)
            .limit(1);
        if (used === undefined) {
            await this.storage.delete(pack);
        }
    }

    /** Filter some blobs down to those no blob column and no live lease references. */
    async #unreferenced(
        transaction: DatabaseConnection,
        taken: readonly { readonly digest: Digest }[],
        now: number,
    ): Promise<Digest[]> {
        // read which of them a blob column of any table, or a live lease, references
        const digests = taken.map(({ digest }) => digest);
        if (digests.length === 0) {
            return [];
        }
        const rows = await transaction.all(this.referenced(now, digests));
        const referenced = new Set(rows.map((row) => DigestRow.parse(row).digest));

        return digests.filter((digest) => !referenced.has(digest));
    }

    /** Select the digests the partition's blob columns and live leases reference, as `digest`. */
    referenced(now: number, among?: readonly Digest[]): SQL {
        // select from each restricting blob column and the live leases, among the digests when given
        const within = (column: SQL | Column) =>
            among === undefined ? sql.empty() : sql`AND ${column} IN ${among}`;
        const cascading = this.#blobColumns("cascade");
        const references = this.#blobColumns("restrict").map(({ table, column }) => {
            // reference the blob only while the column's condition holds for the row
            const options = column.definition.blob;
            const condition = options?.onDelete === "cascade" ? undefined : options?.while;
            const holds =
                condition === undefined
                    ? sql.empty()
                    : sql`AND (${Condition.render(condition, table)})`;

            // never keep alive the blob the row cascades from, as a build attached to itself
            const owners = cascading.filter((each) => each.table === table);
            const others = (digest: SQL | Column) =>
                sql.join(
                    owners.map((owner) => sql`AND ${digest} <> ${owner.column}`),
                    sql` `,
                );

            // reference a scalar column's digest, or each digest of a list or map column
            if (column.definition.kind !== "json") {
                return sql`SELECT ${column} AS digest FROM ${table}
                        WHERE ${column} IS NOT NULL ${holds} ${others(column)} ${within(column)}`;
            }
            const element = sql.raw("element.value");

            return sql`SELECT ${element} AS digest FROM ${table}, ${textValues(column, "element")}
                    WHERE ${column} IS NOT NULL ${holds} ${others(element)} ${within(element)}`;
        });
        const leased = sql`SELECT ${BlobStore.table.digest} AS digest FROM ${BlobStore.table}
            WHERE ${BlobStore.table.leasedUntil} > ${now} ${within(BlobStore.table.digest)}`;

        return sql.join([...references, leased], sql` UNION `);
    }

    /** List the blobs the partition's rows or leases reference, each once. */
    async list(): Promise<Digest[]> {
        const rows = await this.database.all(this.referenced(this.environment.clock.now()));

        return [...new Set(rows.map((row) => DigestRow.parse(row).digest))];
    }

    /** List the blob columns doing one thing on delete, each with its table. */
    #blobColumns(
        onDelete: "restrict" | "cascade",
    ): { readonly table: Table; readonly column: Column }[] {
        return this.database.tables.flatMap((table) =>
            Object.values(table[TABLE].columns)
                .filter((column): column is Column => Column.is(column))
                .filter((column) => {
                    const options = column.definition.blob;

                    return options !== undefined && (options.onDelete ?? "restrict") === onDelete;
                })
                .map((column) => ({ table, column })),
        );
    }
}

/** Gather a body's chunks into one array of bytes. */
export async function gathered(body: AsyncIterable<Uint8Array>): Promise<Uint8Array<ArrayBuffer>> {
    const parts = await Array.fromAsync(body);

    return new Blob(parts.map((part) => new Uint8Array(part))).bytes();
}

/** Read some bytes as one chunk. */
async function* chunks(bytes: Uint8Array): AsyncIterable<Uint8Array> {
    yield bytes;
}

/** The tables of a partition's blob store, which its database declares beside its own. */
export const BLOB_TABLES = [BlobStore.table];
