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
import { canonicalize, Digest, type JsonValue, schema } from "@destack/schema";

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

    /** Write bytes as a blob before a row refers to it, answering their digest. */
    async write(bytes: Uint8Array<ArrayBuffer>): Promise<Digest> {
        const digest = await Digest.of(bytes, this.environment.crypto);
        await this.put(digest, async () => chunks(bytes));

        return digest;
    }

    /** Read a blob's bytes. */
    read(digest: Digest): Promise<Uint8Array<ArrayBuffer>> {
        return gathered(this.storage.read(digest));
    }

    /** Write a JSON value's canonical form as a blob before a row refers to it. */
    writeJson(value: JsonValue): Promise<Digest> {
        return this.write(new TextEncoder().encode(canonicalize(value)));
    }

    /** Read a JSON blob's value. */
    async readJson(digest: Digest): Promise<JsonValue> {
        const text = new TextDecoder().decode(await this.read(digest));

        return schema.json().parse(JSON.parse(text));
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

        // delete retired blobs until none is left, as deleting one's cascading rows retires what they kept
        const cascading = this.#blobColumns("cascade");
        for (
            let retired = await this.#retired();
            retired.length > 0;
            retired = await this.#retired()
        ) {
            for (let start = 0; start < retired.length; start += BATCH_DIGESTS) {
                const batch = retired.slice(start, start + BATCH_DIGESTS);
                await this.#delete(batch, cascading, now);
            }
        }

        // look again once the next lease ends
        return this.nextLeaseEnd(now);
    }

    /** List the retired blobs. */
    async #retired(): Promise<Digest[]> {
        const rows = await this.database.select({ digest: blobGarbage.digest }).from(blobGarbage);

        return rows.map(({ digest }) => digest);
    }

    /** Delete each of some retired blobs still unreferenced, with the rows cascading from it. */
    async #delete(
        batch: readonly Digest[],
        cascading: readonly { readonly table: Table; readonly column: Column }[],
        now: number,
    ): Promise<void> {
        await this.database.transaction(async (transaction) => {
            // take the batch under its rows' lock, so a leasing writer waits, and keep the unreferenced blobs
            const taken = await transaction
                .delete(blobGarbage)
                .where(inArray(blobGarbage.digest, batch))
                .returning({ digest: blobGarbage.digest });
            const unreferenced = await this.#unreferenced(transaction, taken, now);
            if (unreferenced.length === 0) {
                return;
            }

            // delete the rows cascading from them, which retires what those rows kept, and not the blobs again
            for (const { table, column } of cascading) {
                await transaction.delete(table).where(inArray(column, unreferenced));
            }
            await transaction.delete(blobGarbage).where(inArray(blobGarbage.digest, unreferenced));

            // delete the blobs
            for (const digest of unreferenced) {
                await this.storage.delete(digest);
            }
        });
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

            // reference a scalar column's digest, or each digest of a JSON array column
            if (column.definition.kind !== "json") {
                return sql`SELECT ${column} AS digest FROM ${table}
                        WHERE ${column} IS NOT NULL ${holds} ${others(column)} ${within(column)}`;
            }
            const element = sql.raw("element.value");

            return sql`SELECT ${element} AS digest FROM ${table}, ${textElements(column, "element")}
                    WHERE ${column} IS NOT NULL ${holds} ${others(element)} ${within(element)}`;
        });
        const leased = sql`SELECT ${blobLease.digest} AS digest FROM ${blobLease}
            WHERE ${blobLease.until} > ${now} ${within(blobLease.digest)}`;

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
