import { type SQL, sql } from "drizzle-orm";
import { v7 } from "uuid";
import type { DatabaseConnection } from "../database/connection.ts";
import { TABLE, type Select, type Table } from "../table/table.ts";
import type { Dialect } from "../dialect/dialect.ts";
import { assertNever, DatabaseError } from "../error/error.ts";
import { LOG_EPOCH, LOG_HOLD, LOG_HORIZON, LOG_COPYING, LOG, LOG_TRANSACTION } from "./schema.ts";
import { latestOf, selectHead, type LogPosition } from "./position.ts";
import { Snapshot, type Rewind } from "./snapshot.ts";
import type { Row } from "../table/row.ts";
import type { Column } from "../table/column.ts";
import { Key } from "../query/key.ts";
import { CHAIN_TERMS } from "../query/predicate.ts";

/**
 * The most changes one read returns by default before completing its last transaction.
 *
 * A change row is about 0.5 to 2 KB of JSON, so a page is about 1 MB and reads in milliseconds.
 */
const PAGE_LIMIT = 1000;

/** A logged row, binary and sensitive columns left out, for each table of a union. */
export type ChangeRow<Definition extends Table> = Definition extends Table
    ? {
          [
              Property in keyof Select<Definition> as Select<Definition>[Property] extends Uint8Array | null
                  ? never
                  : Property
          ]: Select<Definition>[Property];
      }
    : never;

/** One committed change to a logged table. */
export interface Change<Definition extends Table = Table> {
    /** The change's position in commit order. */
    readonly sequence: number;
    /** The committing transaction, absent for SQLite scripts run outside a transaction. */
    readonly transaction: string | null;
    /** The changed table. */
    readonly table: Definition;
    /** The primary key of the changed row. */
    readonly key: Partial<Select<Definition>>;
    /** Whether the row was inserted, updated or deleted. */
    readonly operation: "insert" | "update" | "delete";
    /** The row before an update or deletion. */
    readonly before?: ChangeRow<Definition>;
    /** The row after an insertion or update. */
    readonly after?: ChangeRow<Definition>;
    /** The scope the changed row lives in. */
    readonly scope: string;
    /** The change time in UTC epoch milliseconds. */
    readonly changedAt: number;
}

/** The transaction that wrote changes: the positions around its changes, and when it started and committed. */
export interface TransactionBounds {
    /** The position before its first change. */
    readonly before: number;
    /** The position after its last change. */
    readonly after: number;
    /** The time of its first change, in UTC epoch milliseconds. */
    readonly startedAt: number;
    /** The time of its last change, in UTC epoch milliseconds. */
    readonly committedAt: number;
}

/** Changes read after a sequence, and the sequence the next read continues after. */
export interface ChangePage<Definition extends Table = Table> {
    /** The changes in commit order. */
    readonly changes: readonly Change<Definition>[];
    /** The sequence the next read continues after. */
    readonly sequence: number;
}

/** The tables and position of a change read. */
export interface ChangeSelection<Definition extends Table> {
    /** The logged tables to read. */
    readonly tables: readonly Definition[];
    /** The sequence already consumed, zero for the beginning of the log. */
    readonly after: number;
    /** The most changes a page holds before completing its last transaction. */
    readonly limit?: number;
    /** Read only changes of rows living in these scopes, through the scope index. */
    readonly scopes?: readonly string[];
}

/** A raw log entry. */
interface ChangeEntry extends Record<string, unknown> {
    /** The newest sequence the log holds, null while it holds none. */
    logged: number | string | null;
    /** The highest compacted sequence, null before the first compaction. */
    horizon: number | string | null;
    /** The entry's sequence, absent when no entry matched. */
    sequence: number | string | null;
    /** The committing transaction. */
    transaction: string | null;
    /** The changed table's SQL name. */
    table: string | null;
    /** The encoded primary key. */
    key: string | unknown[] | null;
    /** Insert, update or delete. */
    operation: "insert" | "update" | "delete" | null;
    /** The encoded row. */
    row: string | Record<string, unknown> | null;
    /** The encoded values an update changed, before it. */
    previous: string | Record<string, unknown> | null;
    /** The scope the changed row lives in, absent when no entry matched. */
    scope: string | null;
    /** The change time. */
    changed_at: number | string | null;
}

/** The log of one database: its committed changes in commit order. */
export class Log {
    /** The database holding the log. */
    readonly database: DatabaseConnection;

    /** Read the log of a database. */
    constructor(database: DatabaseConnection) {
        this.database = database;
    }

    /**
     * Read the images a table's rows had before their first change after one sequence, up to another, by key.
     *
     * A row inserted after the sequence has a null image.
     */
    async images(table: Table, after: number, upto: number): Promise<Map<string, Row | null>> {
        return imagesOf(table, await this.#changes(table, after, upto));
    }

    /** Read images through one memory of each table's changes, which snapshots of nearby positions share. */
    rewind(): Rewind {
        const known = new Map<Table, { after: number; upto: number; changes: Change[] }>();

        return async (table, after, upto) => {
            // read the changes before and beyond those read so far
            const read = known.get(table) ?? { after, upto: after, changes: [] };
            const earlier = after < read.after ? await this.#changes(table, after, read.after) : [];
            const later = upto > read.upto ? await this.#changes(table, read.upto, upto) : [];
            const changes = [...earlier, ...read.changes, ...later];
            known.set(table, {
                after: Math.min(after, read.after),
                upto: Math.max(upto, read.upto),
                changes,
            });

            // keep the image of each row before its first change in the range
            const range = changes.filter(
                (change) => change.sequence > after && change.sequence <= upto,
            );

            return imagesOf(table, range);
        };
    }

    /** Read a table's changes after one sequence, up to another, in commit order. */
    async #changes(table: Table, after: number, upto: number): Promise<Change[]> {
        // read page by page until the pages reach the upper sequence
        const changes: Change[] = [];
        for (let reached = after; reached < upto;) {
            const read = await this.read({ tables: [table], after: reached });
            changes.push(...read.changes.filter((change) => change.sequence <= upto));
            if (read.sequence <= reached) {
                break;
            }
            reached = read.sequence;
        }

        return changes;
    }

    /** Show the database as it was at a position: its logged columns, as the log restores them. */
    at(position: LogPosition, rewind?: Rewind): Snapshot {
        return new Snapshot(this.database, position, rewind);
    }

    /** Read the position of the latest commit: the log's epoch and its latest sequence. */
    async position(): Promise<LogPosition> {
        const [position] = await this.database.execute<{
            epoch: string;
            logged: number | string | null;
            horizon: number | string | null;
        }>(selectHead());

        return { epoch: position!.epoch, sequence: latestOf(position!.logged, position!.horizon) };
    }

    /** Read the latest committed sequence; read it before listing rows, then follow after it. */
    async latest(): Promise<number> {
        return (await this.position()).sequence;
    }

    /** Read the identifier the log records on the open transaction's changes, absent without a log. */
    async currentTransaction(): Promise<string | undefined> {
        // require an open transaction, whose identity the log's triggers read
        if (!this.database.driver.transaction) {
            throw new TypeError("read the transaction identity inside a transaction");
        }

        // read the identity SQLite transactions stamp once the database holds a log, or PostgreSQL's transaction identifier
        const dialect = this.database.dialect;
        if (dialect === "sqlite") {
            const [marked] = this.database.state.isLogged
                ? await this.database.execute<{ id: string }>(
                      sql`SELECT id FROM ${sql.identifier(LOG_TRANSACTION)} WHERE slot = 1`,
                  )
                : [];

            return marked?.id;
        }
        // read PostgreSQL's identifier of the transaction
        else if (dialect === "postgresql") {
            const [current] = await this.database.execute<{ id: string }>(
                sql`SELECT pg_current_xact_id()::TEXT AS id`,
            );

            return current!.id;
        }
        // reject other dialects
        else {
            return assertNever(dialect);
        }
    }

    /** Write rows a source already derived, keeping this database's aggregates from deriving them again. */
    async copying<Value>(run: () => Promise<Value>): Promise<Value> {
        // mark the open transaction while the rows are written
        if (!this.database.driver.transaction) {
            throw new TypeError("copy rows inside a transaction");
        }
        const marker = sql.identifier(LOG_COPYING);
        await this.database.execute(sql`INSERT INTO ${marker} (slot) VALUES (1)`);
        const result = await run();
        await this.database.execute(sql`DELETE FROM ${marker} WHERE slot = 1`);

        return result;
    }

    /** Read the log's epoch, which names the history its sequences count within. */
    async epoch(): Promise<string> {
        const [row] = await this.database.execute<{ epoch: string }>(
            sql`SELECT epoch FROM ${sql.identifier(LOG_EPOCH)} WHERE slot = 1`,
        );

        return row!.epoch;
    }

    /** Start a new epoch after restoring the database, so readers of the old history start over. */
    async renew(): Promise<string> {
        const epoch = v7();
        await this.database.execute(
            sql`UPDATE ${sql.identifier(LOG_EPOCH)} SET epoch = ${epoch} WHERE slot = 1`,
        );

        return epoch;
    }

    /** Read committed changes of the given tables after a sequence, ending each page with a whole transaction. */
    async read<Definition extends Table>(
        selection: ChangeSelection<Definition>,
    ): Promise<ChangePage<Definition>> {
        // read the entries, the latest sequence and the horizon in one statement snapshot
        const limit = selection.limit ?? PAGE_LIMIT;
        const tables = new Map(selection.tables.map((table) => [table[TABLE].sqlName, table]));
        const rows = await this.#entries(
            selection,
            tables,
            sql`sequence > ${selection.after}`,
            limit,
        );

        // require the sequence to be within the retained log, for tables whose changes compaction removes
        const bounds = rows[0]!;
        const horizon = bounds.horizon === null ? 0 : Number(bounds.horizon);
        const isCompacted = selection.tables.some((table) => table[TABLE].tier === "window");
        if (isCompacted && selection.after < horizon) {
            throw new DatabaseError(
                "CHANGES_COMPACTED",
                `changes after ${selection.after} were compacted; list the tables again`,
            );
        }

        // complete the last transaction of a full page
        let entries = rows.filter((entry) => entry.sequence !== null);
        const last = entries.at(-1);
        if (entries.length === limit && last !== undefined && last.transaction !== null) {
            const rest = await this.#entries(
                selection,
                tables,
                sql`sequence > ${last.sequence} AND "transaction" = ${last.transaction}`,
            );
            entries = [...entries, ...rest.filter((entry) => entry.sequence !== null)];
        }

        // decode each entry through its table's columns
        const dialect = this.database.dialect;
        const changes = entries.map((entry) =>
            decodeChange(entry, tables.get(entry.table!)!, dialect),
        );

        // skip past other tables' changes unless the page stopped at the limit
        const end = changes.length > 0 ? changes.at(-1)!.sequence : selection.after;
        const sequence =
            changes.length >= limit ? end : Math.max(end, latestOf(bounds.logged, bounds.horizon));

        return { changes, sequence };
    }

    /** Wait until the log holds a sequence, returning false when the signal aborts first. */
    async wait(sequence: number, signal: AbortSignal): Promise<boolean> {
        // wait on the connection, since a transaction sees no later commits
        if (this.database.driver.transaction) {
            throw new TypeError("wait for changes outside a transaction");
        }

        return this.until(async () => (await this.latest()) >= sequence, signal);
    }

    /** Wait until a check of the log holds, checking again after each commit; false once the signal aborts. */
    until(check: () => Promise<boolean>, signal: AbortSignal): Promise<boolean> {
        return this.database.state.commits.until(this, check, signal);
    }

    /** Yield committed changes of the given tables after a sequence until the signal aborts. */
    async *follow<Definition extends Table>(
        selection: ChangeSelection<Definition>,
        signal: AbortSignal,
    ): AsyncGenerator<ChangePage<Definition>> {
        // read until caught up, then wait for the next commit
        let after = selection.after;
        while (!signal.aborted) {
            const page = await this.read({ ...selection, after });
            after = page.sequence;
            if (page.changes.length > 0) {
                yield page;
            } else {
                await this.wait(after + 1, signal);
            }
        }
    }

    /** Keep the changes after a position for a consumer until a time, replacing the consumer's earlier hold. */
    async hold(name: string, sequence: number, expiresAt: number): Promise<void> {
        const hold = sql.identifier(LOG_HOLD);
        await this.database.execute(sql`
            INSERT INTO ${hold} (name, sequence, expires_at)
            VALUES (${name}, ${sequence}, ${expiresAt})
            ON CONFLICT (name) DO UPDATE SET sequence = excluded.sequence, expires_at = excluded.expires_at
        `);
    }

    /** Stop keeping changes for a consumer. */
    async release(name: string): Promise<void> {
        await this.database.execute(
            sql`DELETE FROM ${sql.identifier(LOG_HOLD)} WHERE name = ${name}`,
        );
    }

    /** Delete the windowed changes older than a time that no consumer's hold keeps, and advance the horizon past them. */
    async compact(before: number, now = Date.now()): Promise<void> {
        await this.database.transaction(async (transaction) => {
            // find the newest windowed change to remove
            const log = sql.identifier(LOG);
            const [newest] = await transaction.execute<{ sequence: number | string | null }>(sql`
                SELECT max(sequence) AS sequence
                FROM ${log}
                WHERE tier = 'window'
                    AND changed_at < ${before}
                    AND sequence IS NOT NULL
            `);
            if (newest?.sequence === null || newest?.sequence === undefined) {
                return;
            }

            // keep the changes after the earliest position a consumer holds until its hold expires, and those compaction kept already
            const [held] = await transaction.execute<{
                sequence: number | string | null;
                horizon: number | string | null;
            }>(sql`
                SELECT
                    (SELECT min(sequence) FROM ${sql.identifier(LOG_HOLD)} WHERE expires_at > ${now}) AS sequence,
                    (SELECT sequence FROM ${sql.identifier(LOG_HORIZON)} WHERE slot = 1) AS horizon
            `);
            const cap =
                held?.sequence === null || held?.sequence === undefined
                    ? Number(newest.sequence)
                    : Math.min(Number(newest.sequence), Number(held.sequence));
            if (cap <= Number(held?.horizon ?? 0)) {
                return;
            }

            // stop before the transaction of the change at that position when it wrote later changes too
            const [split] = await transaction.execute<{ first: number | string | null }>(sql`
                SELECT min(earlier.sequence) AS first
                FROM ${log} change
                JOIN ${log} later ON later."transaction" = change."transaction" AND later.sequence > change.sequence
                JOIN ${log} earlier ON earlier."transaction" = change."transaction"
                WHERE change.sequence = ${cap}
            `);
            const sequence =
                split?.first === null || split?.first === undefined ? cap : Number(split.first) - 1;

            // remove the windowed changes of whole transactions and require readers behind them to list again
            await transaction.execute(sql`
                DELETE FROM ${log}
                WHERE tier = 'window'
                    AND sequence <= ${sequence}
            `);
            const horizon = sql.identifier(LOG_HORIZON);
            const dialect = transaction.dialect;
            const highest =
                dialect === "sqlite"
                    ? sql`max(${horizon}.sequence, excluded.sequence)`
                    : dialect === "postgresql"
                      ? sql`GREATEST(${horizon}.sequence, excluded.sequence)`
                      : assertNever(dialect);
            await transaction.execute(sql`
                INSERT INTO ${horizon} (slot, sequence)
                VALUES (1, ${sequence})
                ON CONFLICT (slot) DO UPDATE SET sequence = ${highest}
            `);
        });
    }

    /** Read the transaction that wrote a change: the positions before its first change and after its last, and when they happened. */
    async bounds(sequence: number): Promise<TransactionBounds> {
        // read the first and last change of the change's transaction, the change alone without one
        const log = sql.identifier(LOG);
        const [bounds] = await this.database.execute<{
            first: number | string | null;
            last: number | string;
            startedAt: number | string;
            committedAt: number | string;
        }>(sql`
            SELECT min(sequence) AS first, max(sequence) AS last,
                min(changed_at) AS "startedAt", max(changed_at) AS "committedAt"
            FROM ${log}
            WHERE sequence = ${sequence}
                OR "transaction" = (SELECT "transaction" FROM ${log} WHERE sequence = ${sequence})
        `);
        if (bounds === undefined || bounds.first === null) {
            throw new DatabaseError("CHANGES_COMPACTED", `change ${sequence} is not in the log`);
        }

        return {
            before: Number(bounds.first) - 1,
            after: Number(bounds.last),
            startedAt: Number(bounds.startedAt),
            committedAt: Number(bounds.committedAt),
        };
    }

    /** Read log entries of the selected tables and scopes with the latest sequence and horizon. */
    async #entries(
        selection: ChangeSelection<Table>,
        tables: ReadonlyMap<string, Table>,
        position: SQL,
        limit?: number,
    ): Promise<ChangeEntry[]> {
        // select the tables and scopes, through a flat chain for a few scopes, which Turso plans far faster than a list
        const log = sql.identifier(LOG);
        const names = sql.join(
            [...tables.keys()].map((name) => sql`${name}`),
            sql`, `,
        );
        const scopes = selection.scopes;
        const scoped =
            scopes === undefined
                ? sql``
                : scopes.length === 0
                  ? sql`AND false`
                  : scopes.length <= CHAIN_TERMS
                    ? sql`AND (${sql.join(
                          scopes.map((scope) => sql`scope = ${scope}`),
                          sql` OR `,
                      )})`
                    : sql`AND scope IN (${sql.join(
                          scopes.map((scope) => sql`${scope}`),
                          sql`, `,
                      )})`;

        return await this.database.execute<ChangeEntry>(sql`
            WITH bounds AS (
                SELECT
                    (SELECT max(sequence) FROM ${log}) AS logged,
                    (SELECT sequence FROM ${sql.identifier(LOG_HORIZON)} WHERE slot = 1) AS horizon
            ), entries AS (
                SELECT sequence, "transaction", "table", key, operation, "row", previous, scope, changed_at
                FROM ${log}
                WHERE ${position}
                    AND "table" IN (${names})
                    ${scoped}
                ORDER BY sequence
                ${limit === undefined ? sql`` : sql`LIMIT ${limit}`}
            )
            SELECT bounds.logged, bounds.horizon, entries.*
            FROM bounds
            LEFT JOIN entries ON 1 = 1
            ORDER BY entries.sequence
        `);
    }
}

/** Decode a log entry through its table's columns. */
function decodeChange<Definition extends Table>(
    entry: ChangeEntry,
    table: Definition,
    dialect: Dialect,
): Change<Definition> {
    // read JSON text from SQLite and parsed JSON from PostgreSQL
    const key = (typeof entry.key === "string" ? JSON.parse(entry.key) : entry.key) as unknown[];
    const row = (typeof entry.row === "string" ? JSON.parse(entry.row) : entry.row) as Record<
        string,
        unknown
    >;
    const previous = (
        typeof entry.previous === "string" ? JSON.parse(entry.previous) : entry.previous
    ) as Record<string, unknown> | null;

    // decode each value through its column, keeping missing values as null
    const { columns, entries, key: keyProperties, logged: recorded } = table[TABLE];
    const decode = (column: Column, value: unknown) =>
        value === null || value === undefined ? null : column.definition.decode(value, dialect);

    // restore the row before an update from the values it changed
    const logged: Record<string, unknown> = {};
    const changed: Record<string, unknown> = {};
    for (const [property, column] of entries) {
        if (!Object.hasOwn(recorded, property)) {
            continue;
        }
        const name = column.definition.name;
        logged[property] = decode(column, row[name]);
        if (previous !== null && Object.hasOwn(previous, name)) {
            changed[property] = decode(column, previous[name]);
        }
    }
    const decodedKey: Record<string, unknown> = {};
    for (const [index, property] of keyProperties.entries()) {
        decodedKey[property] = decode(columns[property]!, key[index]);
    }

    return {
        sequence: Number(entry.sequence),
        transaction: entry.transaction,
        table,
        key: decodedKey as Partial<Select<Definition>>,
        operation: entry.operation!,
        ...(entry.operation === "insert"
            ? { after: logged as ChangeRow<Definition> }
            : entry.operation === "delete"
              ? { before: logged as ChangeRow<Definition> }
              : {
                    before: { ...logged, ...changed } as ChangeRow<Definition>,
                    after: logged as ChangeRow<Definition>,
                }),
        scope: entry.scope!,
        changedAt: Number(entry.changed_at),
    };
}

/** Keep the image each row had before its first change among some changes, by key, null for a row they insert. */
function imagesOf(table: Table, changes: readonly Change[]): Map<string, Row | null> {
    const images = new Map<string, Row | null>();
    for (const change of changes) {
        const key = Key.name(table, change.key);
        if (!images.has(key)) {
            images.set(key, (change.before as Row | undefined) ?? null);
        }
    }

    return images;
}
