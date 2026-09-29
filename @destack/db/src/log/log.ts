import { type SQL, type SQLWrapper, sql } from "drizzle-orm";
import { dialectSQL } from "../dialect/expression.ts";
import { Statement } from "../query/statement.ts";
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
import { toJsonSchema } from "@destack/schema";
import { digest } from "@destack/schema/json";

/**
 * The default page size of a log read, in changes.
 *
 * At 0.5 to 2 KB a change, a page is about 1 MB.
 */
const PAGE_LIMIT = 1000;

/** A logged row, without binary and sensitive columns. */
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
    /** The committing transaction, absent for SQLite scripts outside a transaction. */
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
    /** The change time, in UTC epoch milliseconds. */
    readonly changedAt: number;
}

/** The positions and times of a transaction's changes. */
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

/** Changes read after a sequence. */
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
    /** The consumed sequence, zero for the start. */
    readonly after: number;
    /** The most changes a page holds before completing its last transaction. */
    readonly limit?: number;
    /** The scopes to read. */
    readonly scopes?: readonly string[];
}

/** A raw log entry. */
interface ChangeEntry extends Record<string, unknown> {
    /** The newest logged sequence. */
    logged: number | string | null;
    /** The highest compacted sequence. */
    horizon: number | string | null;
    /** The entry's sequence. */
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
    /** The values an update changed, before it. */
    previous: string | Record<string, unknown> | null;
    /** The changed row's scope. */
    scope: string | null;
    /** The change time. */
    changed_at: number | string | null;
}

/** The hexadecimal digits of a shape digest: 128 bits, whose collisions are negligible among any database's tables. */
const SHAPE_LENGTH = 32;

/** The logged columns of each table, described once. */
const COLUMNS = new WeakMap<Table, unknown[]>();

/** The committed changes of one database, in commit order. */
export class Log {
    /** The database holding the log. */
    readonly database: DatabaseConnection;

    /** Create the log of a database. */
    constructor(database: DatabaseConnection) {
        this.database = database;
    }

    /** Digest the shape tables' changes carry: each one's logged columns with their kinds and values. */
    static async shape(tables: readonly Table[]): Promise<string> {
        // describe each table's logged columns once
        const described = tables.map((table) => {
            let columns = COLUMNS.get(table);
            if (columns === undefined) {
                columns = Object.values(table[TABLE].logged).map(({ definition }) => [
                    definition.name,
                    definition.kind,
                    definition.nullable,
                    toJsonSchema(definition.json ?? definition.schema),
                ]);
                COLUMNS.set(table, columns);
            }

            return [table[TABLE].sqlName, columns];
        });

        return (await digest(described)).slice(0, SHAPE_LENGTH);
    }

    /** Read the images a table's rows had before their first change between two sequences, by key. */
    async images(table: Table, after: number, upto: number): Promise<Map<string, Row | null>> {
        return imagesOf(table, await this.#changes(table, after, upto));
    }

    /** Read images through one shared memory of each table's changes. */
    rewind(): Rewind {
        const known = new Map<Table, { after: number; upto: number; changes: Change[] }>();

        return async (table, after, upto) => {
            // read the missing changes
            const read = known.get(table) ?? { after, upto: after, changes: [] };
            const earlier = after < read.after ? await this.#changes(table, after, read.after) : [];
            const later = upto > read.upto ? await this.#changes(table, read.upto, upto) : [];
            const changes = [...earlier, ...read.changes, ...later];
            known.set(table, {
                after: Math.min(after, read.after),
                upto: Math.max(upto, read.upto),
                changes,
            });

            // keep each row's image before its first change
            const range = changes.filter(
                (change) => change.sequence > after && change.sequence <= upto,
            );

            return imagesOf(table, range);
        };
    }

    /** Read a table's changes between two sequences. */
    async #changes(table: Table, after: number, upto: number): Promise<Change[]> {
        // read page by page
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

    /** Show the database's logged columns as they were at a position. */
    at(position: LogPosition, rewind?: Rewind): Snapshot {
        return new Snapshot(this.database, position, rewind);
    }

    /** Read the position of the latest commit. */
    async position(): Promise<LogPosition> {
        const [position] = await this.database.execute<{
            epoch: string;
            logged: number | string | null;
            horizon: number | string | null;
        }>(selectHead(this.database.dialect));

        return { epoch: position!.epoch, sequence: latestOf(position!.logged, position!.horizon) };
    }

    /**
     * Read the position the open transaction's reads reach.
     *
     * SQLite numbers a transaction's entries on write, and PostgreSQL at commit.
     */
    async reached(): Promise<LogPosition> {
        // reach the latest commit
        const committed = await this.position();
        if (this.database.dialect !== "sqlite" || !this.database.driver.transaction) {
            return committed;
        }

        // reach past the open SQLite transaction's entries
        const [own] = await this.database.execute<{ sequence: number | string | null }>(sql`
            SELECT max(sequence) AS sequence FROM ${sql.identifier(LOG)}
            WHERE "transaction" = (SELECT id FROM ${sql.identifier(LOG_TRANSACTION)} WHERE slot = 1)
        `);
        const sequence = own?.sequence === null || own === undefined ? 0 : Number(own.sequence);

        return { epoch: committed.epoch, sequence: Math.max(committed.sequence, sequence) };
    }

    /** Name the open transaction's log identifier as an SQL expression. */
    stamp(): SQL {
        // require an open transaction
        if (!this.database.driver.transaction) {
            throw new TypeError("read the transaction identity inside a transaction");
        }

        // name the SQLite transaction stamp
        const dialect = this.database.dialect;
        if (dialect === "sqlite") {
            return this.database.state.isLogged
                ? sql`(SELECT id FROM ${sql.identifier(LOG_TRANSACTION)} WHERE slot = 1)`
                : sql`NULL`;
        }
        // name PostgreSQL's transaction identifier
        else if (dialect === "postgresql") {
            return sql`pg_current_xact_id_if_assigned()::TEXT`;
        }
        // reject other dialects
        else {
            return assertNever(dialect);
        }
    }

    /** Write rows a source already derived, without deriving aggregates again. */
    async copying<Value>(run: () => Promise<Value>): Promise<Value> {
        // mark the open transaction
        if (!this.database.driver.transaction) {
            throw new TypeError("copy rows inside a transaction");
        }
        const marker = sql.identifier(LOG_COPYING);
        await this.database.execute(sql`INSERT INTO ${marker} (slot) VALUES (1)`);
        const result = await run();
        await this.database.execute(sql`DELETE FROM ${marker} WHERE slot = 1`);

        return result;
    }

    /** Read the log's epoch. */
    async epoch(): Promise<string> {
        const [row] = await this.database.execute<{ epoch: string }>(
            sql`SELECT epoch FROM ${sql.identifier(LOG_EPOCH)} WHERE slot = 1`,
        );

        return row!.epoch;
    }

    /** Start a new epoch after a restore. */
    async renew(): Promise<string> {
        const epoch = v7();
        await this.database.execute(
            sql`UPDATE ${sql.identifier(LOG_EPOCH)} SET epoch = ${epoch} WHERE slot = 1`,
        );

        return epoch;
    }

    /** Read committed changes after a sequence, ending each page with a whole transaction. */
    async read<Definition extends Table>(
        selection: ChangeSelection<Definition>,
    ): Promise<ChangePage<Definition>> {
        // read the entries and bounds in one statement
        const limit = selection.limit ?? PAGE_LIMIT;
        const tables = new Map(selection.tables.map((table) => [table[TABLE].sqlName, table]));
        const rows = await this.#entries(selection, tables, selection.after, { limit });

        // require a retained sequence
        const bounds = rows[0]!;
        const horizon = bounds.horizon === null ? 0 : Number(bounds.horizon);
        const isCompacted = selection.tables.some((table) => table[TABLE].retention === "window");
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
            const rest = await this.#entries(selection, tables, Number(last.sequence), {
                transaction: last.transaction,
            });
            entries = [...entries, ...rest.filter((entry) => entry.sequence !== null)];
        }

        // decode each entry
        const dialect = this.database.dialect;
        const changes = entries.map((entry) => ({
            sequence: Number(entry.sequence),
            ...decodeChange(entry, tables.get(entry.table!)!, dialect),
        }));

        // skip other tables' changes unless the page is full
        const end = changes.length > 0 ? changes.at(-1)!.sequence : selection.after;
        const sequence =
            changes.length >= limit ? end : Math.max(end, latestOf(bounds.logged, bounds.horizon));

        return { changes, sequence };
    }

    /** Read the open transaction's changes before commit. */
    async written<Definition extends Table>(
        tables: readonly Definition[],
    ): Promise<Omit<Change<Definition>, "sequence">[]> {
        // require a log and a writing transaction
        if (this.database.dialect === "sqlite" && !this.database.state.isLogged) {
            throw new TypeError("read written changes of a logged database");
        }
        const [current] = await this.database.execute<{ id: string | null }>(
            sql`SELECT ${this.stamp()} AS id`,
        );
        const transaction = current?.id ?? null;
        if (transaction === null) {
            return [];
        }

        // read its entries in write order
        const byName = new Map(tables.map((table) => [table[TABLE].sqlName, table]));
        const names = sql.join(
            [...byName.keys()].map((name) => sql`${name}`),
            sql`, `,
        );
        const written =
            this.database.dialect === "postgresql"
                ? sql`sequence IS NULL ORDER BY id`
                : sql`true ORDER BY sequence`;
        const entries = await this.database.execute<ChangeEntry>(sql`
            SELECT sequence, "transaction", "table", key, operation, "row", previous, scope, changed_at
            FROM ${sql.identifier(LOG)}
            WHERE "transaction" = ${transaction} AND "table" IN (${names}) AND ${written}
        `);

        return entries.map((entry) =>
            decodeChange(entry, byName.get(entry.table!)!, this.database.dialect),
        );
    }

    /** Wait until the log holds a sequence, returning false once the signal aborts. */
    async wait(sequence: number, signal: AbortSignal): Promise<boolean> {
        // wait on the connection
        if (this.database.driver.transaction) {
            throw new TypeError("wait for changes outside a transaction");
        }

        return this.until(async () => (await this.position()).sequence >= sequence, signal);
    }

    /** Wait until a check holds after a commit, returning false once the signal aborts. */
    until(check: () => Promise<boolean>, signal: AbortSignal): Promise<boolean> {
        return this.database.state.commits.until(check, signal);
    }

    /** Yield each page past a sequence, with its selected changes, until the signal aborts. */
    async *follow<Definition extends Table>(
        selection: ChangeSelection<Definition>,
        signal: AbortSignal,
    ): AsyncGenerator<ChangePage<Definition>> {
        // yield each advance, also past commits of other tables, then wait
        let after = selection.after;
        while (!signal.aborted) {
            const page = await this.read({ ...selection, after });
            if (page.sequence > after) {
                after = page.sequence;
                yield page;
            } else {
                await this.wait(after + 1, signal);
            }
        }
    }

    /** Keep the changes after a position for a consumer until a time. */
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

    /** Delete old windowed changes no hold keeps, and advance the horizon. */
    async compact(before: number, now = Date.now()): Promise<void> {
        await this.database.transaction(async (transaction) => {
            // find the newest removable change
            const log = sql.identifier(LOG);
            const [newest] = await transaction.execute<{ sequence: number | string | null }>(sql`
                SELECT max(sequence) AS sequence
                FROM ${log}
                WHERE retention = 'window'
                    AND changed_at < ${before}
                    AND sequence IS NOT NULL
            `);
            if (newest?.sequence === null || newest?.sequence === undefined) {
                return;
            }

            // keep the held changes
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

            // stop before a split transaction
            const [split] = await transaction.execute<{ first: number | string | null }>(sql`
                SELECT min(earlier.sequence) AS first
                FROM ${log} change
                JOIN ${log} later ON later."transaction" = change."transaction" AND later.sequence > change.sequence
                JOIN ${log} earlier ON earlier."transaction" = change."transaction"
                WHERE change.sequence = ${cap}
            `);
            const sequence =
                split?.first === null || split?.first === undefined ? cap : Number(split.first) - 1;

            // remove the windowed changes of whole transactions
            await transaction.execute(sql`
                DELETE FROM ${log}
                WHERE retention = 'window'
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

    /** Read the positions and times of a change's transaction. */
    async bounds(sequence: number): Promise<TransactionBounds> {
        // read the transaction's first and last change
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

    /** Read log entries after a sequence, or one transaction's rest, with the bounds. */
    async #entries(
        selection: ChangeSelection<Table>,
        tables: ReadonlyMap<string, Table>,
        after: number,
        options: { readonly limit?: number; readonly transaction?: string },
    ): Promise<ChangeEntry[]> {
        // read through the statement of the selection's shape
        const statement = entryRead(
            selection.scopes !== undefined,
            options.transaction !== undefined,
        );

        return statement.all(this.database, {
            after,
            tables: JSON.stringify([...tables.keys()]),
            scopes: JSON.stringify(selection.scopes ?? []),
            limit: options.limit ?? null,
            transaction: options.transaction ?? null,
        }) as Promise<ChangeEntry[]>;
    }
}

/** The entry statements by shape. */
const ENTRY_READS = new Map<string, Statement>();

/** Build the statement reading log entries after a sequence. */
function entryRead(isScoped: boolean, isRest: boolean): Statement {
    // reuse the statement of the same shape
    const key = `${isScoped}:${isRest}`;
    const known = ENTRY_READS.get(key);
    if (known !== undefined) {
        return known;
    }

    // match a column against a JSON array
    const listed = (column: SQL, array: SQLWrapper) =>
        dialectSQL({
            sqlite: sql`${column} IN (SELECT value FROM json_each(${array}))`,
            postgresql: sql`${column} IN (SELECT jsonb_array_elements_text(${array}::jsonb))`,
        });

    // select the bounds and entries
    const log = sql.identifier(LOG);
    const statement = new Statement(
        (value) => sql`
            WITH bounds AS (
                SELECT
                    (SELECT max(sequence) FROM ${log}) AS logged,
                    (SELECT sequence FROM ${sql.identifier(LOG_HORIZON)} WHERE slot = 1) AS horizon
            ), entries AS (
                SELECT sequence, "transaction", "table", key, operation, "row", previous, scope, changed_at
                FROM ${log}
                WHERE sequence > ${value("after")}
                    ${isRest ? sql`AND "transaction" = ${value("transaction")}` : sql``}
                    AND ${listed(sql`"table"`, value("tables"))}
                    ${isScoped ? sql`AND ${listed(sql`scope`, value("scopes"))}` : sql``}
                ORDER BY sequence
                ${isRest ? sql`` : sql`LIMIT ${value("limit")}`}
            )
            SELECT bounds.logged, bounds.horizon, entries.*
            FROM bounds
            LEFT JOIN entries ON 1 = 1
            ORDER BY entries.sequence
        `,
    );
    ENTRY_READS.set(key, statement);

    return statement;
}

/** Decode a log entry. */
function decodeChange<Definition extends Table>(
    entry: ChangeEntry,
    table: Definition,
    dialect: Dialect,
): Omit<Change<Definition>, "sequence"> {
    // parse SQLite JSON text
    const key = (typeof entry.key === "string" ? JSON.parse(entry.key) : entry.key) as unknown[];
    const row = (typeof entry.row === "string" ? JSON.parse(entry.row) : entry.row) as Record<
        string,
        unknown
    >;
    const previous = (
        typeof entry.previous === "string" ? JSON.parse(entry.previous) : entry.previous
    ) as Record<string, unknown> | null;

    // decode each value through its column
    const { columns, entries, key: keyProperties, logged: recorded } = table[TABLE];
    const decode = (column: Column, value: unknown) =>
        value === null || value === undefined ? null : column.definition.decode(value, dialect);

    // restore the row before an update
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

/** Keep each row's image before its first change, null for an insert. */
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
