import { type SQL, sql } from "drizzle-orm";
import { v7 } from "uuid";
import type { DatabaseConnection } from "../database/connection.ts";
import { TABLE, type Select, type Table } from "../table/table.ts";
import type { Dialect } from "../dialect/dialect.ts";
import { assertNever, DatabaseError } from "../error/error.ts";
import {
    LOG_EPOCH,
    LOG_HORIZON,
    LOG_COPYING,
    LOG,
    LOG_TRANSACTION,
    loggedColumns,
    primaryKey,
} from "./trigger.ts";
import type { LogPosition } from "./position.ts";

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
    /** The committing transaction, absent for SQLite statements outside a transaction. */
    readonly transaction: string | null;
    /** The changed table. */
    readonly table: Definition;
    /** The primary key of the changed row. */
    readonly key: Partial<Select<Definition>>;
    /** Whether the row was inserted, updated or deleted. */
    readonly operation: "insert" | "update" | "delete";
    /** The row after an insertion or update, or before a deletion. */
    readonly row: ChangeRow<Definition>;
    /** The values an update changed, as they were before it. */
    readonly previous?: Partial<ChangeRow<Definition>>;
    /** The value of the table's routing column, absent for tables without a route. */
    readonly route: string | null;
    /** The change time in UTC epoch milliseconds. */
    readonly changedAt: number;
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
    /** Read only changes routed to these values, through the route index. */
    readonly routes?: readonly string[];
}

/** A raw log entry. */
interface ChangeEntry extends Record<string, unknown> {
    /** The latest committed sequence in the log, zero while it is empty. */
    latest: number | string;
    /** The highest compacted sequence, zero before the first compaction. */
    horizon: number | string;
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
    /** The routing column's value. */
    route: string | null;
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

    /** Read the position of the latest commit: the log's epoch and its latest sequence. */
    async position(): Promise<LogPosition> {
        return { epoch: await this.epoch(), sequence: await this.latest() };
    }

    /** Read the latest committed sequence; read it before listing rows, then follow after it. */
    async latest(): Promise<number> {
        const [latest] = await this.database.execute<{ sequence: number | string }>(
            sql`SELECT coalesce(max(sequence), 0) AS sequence FROM ${sql.identifier(LOG)}`,
        );

        return Number(latest!.sequence);
    }

    /** Read the identifier the log records on the open transaction's changes, absent without a log. */
    async currentTransaction(): Promise<string | undefined> {
        // require an open transaction, whose identity the log's triggers read
        if (!this.database.driver.transaction) {
            throw new TypeError("read the transaction identity inside a transaction");
        }

        // read the identity SQLite transactions stamp, or PostgreSQL's transaction identifier
        const dialect = this.database.dialect;
        if (dialect === "sqlite") {
            const [logged] = await this.database.execute<{ name: string }>(
                sql`SELECT name FROM sqlite_schema WHERE type = 'table' AND name = ${LOG_TRANSACTION}`,
            );
            const [marked] = logged
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
        const horizon = Number(bounds.horizon);
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
        if (entries.length === limit && last?.transaction !== null && last !== undefined) {
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
        const sequence = changes.length >= limit ? end : Math.max(end, Number(bounds.latest));

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

    /** Delete windowed changes older than a time and advance the horizon past them. */
    async compact(before: number): Promise<void> {
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

            // stop before the transaction of that change when it wrote later changes too
            const [split] = await transaction.execute<{ first: number | string | null }>(sql`
                SELECT min(later.sequence) AS first
                FROM ${log} newest
                JOIN ${log} later ON later."transaction" = newest."transaction"
                WHERE newest.sequence = ${Number(newest.sequence)}
                    AND later.sequence > newest.sequence
            `);
            const sequence =
                split?.first === null || split?.first === undefined
                    ? Number(newest.sequence)
                    : await this.#before(transaction, Number(newest.sequence));

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

    /** Read the sequence before the first change of the transaction that wrote a change. */
    async #before(database: DatabaseConnection, sequence: number): Promise<number> {
        const [first] = await database.execute<{ sequence: number | string }>(sql`
            SELECT min(earlier.sequence) AS sequence
            FROM ${sql.identifier(LOG)} change
            JOIN ${sql.identifier(LOG)} earlier ON earlier."transaction" = change."transaction"
            WHERE change.sequence = ${sequence}
        `);

        return Number(first!.sequence) - 1;
    }

    /** Read log entries of the selected tables and routes with the latest sequence and horizon. */
    async #entries(
        selection: ChangeSelection<Table>,
        tables: ReadonlyMap<string, Table>,
        position: SQL,
        limit?: number,
    ): Promise<ChangeEntry[]> {
        // select the tables and routes
        const log = sql.identifier(LOG);
        const names = sql.join(
            [...tables.keys()].map((name) => sql`${name}`),
            sql`, `,
        );
        const routes =
            selection.routes === undefined
                ? sql``
                : selection.routes.length === 0
                  ? sql`AND false`
                  : sql`AND route IN (${sql.join(
                        selection.routes.map((route) => sql`${route}`),
                        sql`, `,
                    )})`;

        return await this.database.execute<ChangeEntry>(sql`
            WITH bounds AS (
                SELECT
                    coalesce((SELECT max(sequence) FROM ${log}), 0) AS latest,
                    coalesce((SELECT sequence FROM ${sql.identifier(LOG_HORIZON)} WHERE slot = 1), 0) AS horizon
            ), entries AS (
                SELECT sequence, "transaction", "table", key, operation, "row", previous, route, changed_at
                FROM ${log}
                WHERE ${position}
                    AND "table" IN (${names})
                    ${routes}
                ORDER BY sequence
                ${limit === undefined ? sql`` : sql`LIMIT ${limit}`}
            )
            SELECT bounds.latest, bounds.horizon, entries.*
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
    const columns = Object.entries(table[TABLE].columns);
    const keyColumns = primaryKey(table).map((column) =>
        columns.find(([, candidate]) => candidate === column)!,
    );
    const decode = (column: (typeof columns)[number][1], value: unknown) =>
        value === null || value === undefined ? null : column.definition.decode(value, dialect);

    return {
        sequence: Number(entry.sequence),
        transaction: entry.transaction,
        table,
        key: Object.fromEntries(
            keyColumns.map(([property, column], index) => [property, decode(column, key[index])]),
        ) as Partial<Select<Definition>>,
        operation: entry.operation!,
        row: Object.fromEntries(
            Object.entries(loggedColumns(table)).map(([property, column]) => [
                property,
                decode(column, row[column.definition.name]),
            ]),
        ) as ChangeRow<Definition>,
        ...(previous === null || previous === undefined
            ? {}
            : {
                  previous: Object.fromEntries(
                      columns
                          .filter(([, column]) => Object.hasOwn(previous, column.definition.name))
                          .map(([property, column]) => [
                              property,
                              decode(column, previous[column.definition.name]),
                          ]),
                  ) as Partial<ChangeRow<Definition>>,
              }),
        route: entry.route ?? null,
        changedAt: Number(entry.changed_at),
    };
}
