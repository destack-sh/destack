import type { Environment } from "@destack/environment";
import { asc, gt, inArray, lte } from "../sql/operator.ts";
import { sql, type SQL } from "../sql/sql.ts";
import { Column } from "../table/column.ts";
import { TABLE, type Table } from "../table/table.ts";
import { Condition } from "../query/condition.ts";
import { textElements } from "../query/statement.ts";
import type { DatabaseConnection } from "../database/connection.ts";
import { blobGarbage, blobLease } from "./table.ts";
import type { BlobStorage } from "@destack/resource";
import { Digest, schema } from "@destack/schema";

/** How long a store leases what it writes for a row to reference, by default: half an hour. */
const LEASE_MILLISECONDS = 30 * 60_000;

/** The digests one statement binds: 500, far under SQLite's 32,766 parameters. */
const BATCH_DIGESTS = 500;

/** A row of one blob, as a raw statement answers it. */
const DigestRow = schema.object({ digest: Digest });

/** The storage, database and environment of a partition's blob store. */
export interface BlobStoreOptions<Storage extends BlobStorage = BlobStorage> {
    /** The storage keeping the blobs by digest. */
    readonly storage: Storage;
    /** The partition's database, which declares the blob tables beside its own. */
    readonly database: DatabaseConnection;
    /** The clock leases end on, and the crypto digesting written bytes. */
    readonly environment: Pick<Environment, "clock" | "crypto">;
    /** How long a write's lease lasts for a row to reference it, half an hour by default. */
    readonly leaseMilliseconds?: number;
}

/** A partition's blobs, each kept alive by a blob column of any of its rows, or by a lease. */
export class BlobStore<Storage extends BlobStorage = BlobStorage> {
    /** The storage keeping the blobs by digest. */
    readonly storage: Storage;
    /** The partition's database. */
    readonly database: DatabaseConnection;
    /** The clock leases end on, and the crypto digesting written bytes. */
    readonly environment: Pick<Environment, "clock" | "crypto">;
    /** How long a write's lease lasts, in milliseconds. */
    readonly leaseMilliseconds: number;

    /** Keep a partition's blobs in a storage, referenced from its database. */
    constructor(options: BlobStoreOptions<Storage>) {
        // store the storage, database, environment and lease length
        this.storage = options.storage;
        this.database = options.database;
        this.environment = options.environment;
        this.leaseMilliseconds = options.leaseMilliseconds ?? LEASE_MILLISECONDS;
    }

    /** Lease a blob for a row to reference, then write it unless stored. */
    async put(digest: Digest, open: () => Promise<AsyncIterable<Uint8Array>>): Promise<void> {
        await this.lease([digest]);
        if ((await this.storage.missing([digest])).length > 0) {
            await this.storage.write(await open(), digest);
        }
    }

    /** Lease blobs until a moment, rescuing any retired one, waiting out its deletion. */
    async lease(
        digests: readonly Digest[],
        until: number = this.environment.clock.now() + this.leaseMilliseconds,
    ): Promise<void> {
        await this.database.transaction(async (transaction) => {
            for (let start = 0; start < digests.length; start += BATCH_DIGESTS) {
                // lease the batch, keeping a later lease
                const batch = [...new Set(digests.slice(start, start + BATCH_DIGESTS))];
                const excluded = sql`excluded.${sql.identifier("until")}`;
                await transaction
                    .insert(blobLease)
                    .values(batch.map((digest) => ({ digest, until })))
                    .onConflictDoUpdate({
                        target: blobLease.digest,
                        set: {
                            until: sql`CASE WHEN ${excluded} > ${blobLease.until}
                                THEN ${excluded} ELSE ${blobLease.until} END`,
                        },
                    });

                // take them out of the garbage, waiting while a collection deletes one
                await transaction.delete(blobGarbage).where(inArray(blobGarbage.digest, batch));
            }
        });
    }

    /** Mark some blobs, which a collection deletes unless a row or a lease keeps them. */
    async #retire(transaction: DatabaseConnection, digests: readonly Digest[]): Promise<void> {
        for (let start = 0; start < digests.length; start += BATCH_DIGESTS) {
            const batch = [...new Set(digests.slice(start, start + BATCH_DIGESTS))];
            await transaction
                .insert(blobGarbage)
                .values(batch.map((digest) => ({ digest })))
                .onConflictDoNothing();
        }
    }

    /** Delete retired and lease-expired blobs no row references, answering the next lease end. */
    async collect(): Promise<number | undefined> {
        // retire the blobs of the ended leases, writing only when one ended
        const now = this.environment.clock.now();
        const [ending] = await this.database
            .select({ digest: blobLease.digest })
            .from(blobLease)
            .where(lte(blobLease.until, now))
            .limit(1);
        if (ending !== undefined) {
            await this.database.transaction(async (transaction) => {
                const ended = await transaction
                    .delete(blobLease)
                    .where(lte(blobLease.until, now))
                    .returning({ digest: blobLease.digest });
                await this.#retire(
                    transaction,
                    ended.map(({ digest }) => digest),
                );
            });
        }

        // delete each retired blob still unreferenced under its row's lock, so a leasing writer waits
        const retired = await this.database
            .select({ digest: blobGarbage.digest })
            .from(blobGarbage);
        const digests = retired.map(({ digest }) => digest);
        for (let start = 0; start < digests.length; start += BATCH_DIGESTS) {
            const batch = digests.slice(start, start + BATCH_DIGESTS);
            await this.database.transaction(async (transaction) => {
                const taken = await transaction
                    .delete(blobGarbage)
                    .where(inArray(blobGarbage.digest, batch))
                    .returning({ digest: blobGarbage.digest });
                for (const digest of await this.#unreferenced(transaction, taken, now)) {
                    await this.storage.delete(digest);
                }
            });
        }

        // look again once the next lease ends
        return this.nextLeaseEnd(now);
    }

    /** Read when the next lease ending after a moment ends, absent without one. */
    async nextLeaseEnd(after: number): Promise<number | undefined> {
        const [next] = await this.database
            .select({ until: blobLease.until })
            .from(blobLease)
            .where(gt(blobLease.until, after))
            .orderBy(asc(blobLease.until))
            .limit(1);

        return next?.until;
    }

    /** Delete every blob, lease and retirement, as a partition leaving its host does. */
    async clear(): Promise<void> {
        for await (const digest of this.storage.digests()) {
            await this.storage.delete(digest);
        }
        await this.database.delete(blobLease);
        await this.database.delete(blobGarbage);
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
        const rows = await transaction.all(this.referenced(now, { among: digests }));
        const referenced = new Set(rows.map((row) => DigestRow.parse(row).digest));

        return digests.filter((digest) => !referenced.has(digest));
    }

    /** Select the digests the partition's blob columns and live leases reference, as `digest`. */
    referenced(
        now: number,
        options: {
            /** The digests to look for, absent for all. */
            readonly among?: readonly Digest[];
            /** The tables whose blob columns to leave out, such as a store's own edges. */
            readonly except?: readonly Table[];
        } = {},
    ): SQL {
        // select from each blob column and the live leases, among the digests when given
        const { among, except = [] } = options;
        const within = (column: SQL | Column) =>
            among === undefined ? sql.empty() : sql`AND ${column} IN ${among}`;
        const references = this.#blobColumns()
            .filter(({ table }) => !except.includes(table))
            .map(({ table, column }) => {
                // reference the blob only while the column's condition holds for the row
                const condition = column.definition.blob?.while;
                const holds =
                    condition === undefined
                        ? sql.empty()
                        : sql`AND (${Condition.render(condition, table)})`;

                // reference a scalar column's digest, or each digest of a JSON array column
                if (column.definition.kind !== "json") {
                    return sql`SELECT ${column} AS digest FROM ${table}
                        WHERE ${column} IS NOT NULL ${holds} ${within(column)}`;
                }
                const element = sql.raw("element.value");

                return sql`SELECT ${element} AS digest FROM ${table}, ${textElements(column, "element")}
                    WHERE ${column} IS NOT NULL ${holds} ${within(element)}`;
            });
        const leased = sql`SELECT ${blobLease.digest} AS digest FROM ${blobLease}
            WHERE ${blobLease.until} > ${now} ${within(blobLease.digest)}`;

        return sql.join([...references, leased], sql` UNION `);
    }

    /** List the blob columns of the partition's tables, each with its table. */
    #blobColumns(): { readonly table: Table; readonly column: Column }[] {
        return this.database.tables.flatMap((table) =>
            Object.values(table[TABLE].columns)
                .filter((column): column is Column => Column.is(column))
                .filter((column) => column.definition.blob !== undefined)
                .map((column) => ({ table, column })),
        );
    }
}
