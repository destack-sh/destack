import { dialectSQL, sql, type SQL, type SQLWrapper } from "../sql/index.ts";
import { Statement } from "../query/statement.ts";
import { v7 } from "uuid";
import type { DatabaseConnection } from "../database/connection.ts";
import { TABLE, type RowImage, Table } from "../table/table.ts";
import type { Dialect } from "../dialect/dialect.ts";
import { assertNever, DatabaseError } from "../error/error.ts";
import { LOG_EPOCH, LOG_SLOT, LOG_HORIZON, LOG_REPLICA, LOG, LOG_TRANSACTION } from "./schema.ts";
import { createLog } from "./trigger.ts";
import { latestOf, LogInteger, selectHead, type LogPosition } from "./position.ts";
import { Snapshot, type Rewind } from "./snapshot.ts";
import type { Row } from "../table/row.ts";
import type { ColumnValue } from "../table/column.ts";
import { Key } from "../query/key.ts";
import { Digest, schema, toJsonSchema, type JsonValue } from "@destack/schema";

/**
 * The default page size of a log read, in changes.
 *
 * At 0.5 to 2 KB a change, a page is about 1 MB.
 */
const PAGE_LIMIT = 1000;

/** One write to a logged table: an insert with the row after, an update with both rows, or a delete with the row before. */
export type Write<Definition extends Table = Table> = {
    /** The writing transaction, absent for SQLite scripts outside a transaction. */
    readonly transaction: string | null;
    /** The changed table. */
    readonly table: Definition;
    /** The primary key of the changed row. */
    readonly key: Key<Definition>;
    /** The scope the changed row lives in. */
    readonly scope: string;
    /** The change time, in UTC epoch milliseconds. */
    readonly changedAt: number;
} & (
    | {
          /** Insert the row. */
          readonly operation: "insert";
          /** The row after the insert. */
          readonly after: RowImage<Definition>;
      }
    | {
          /** Update the row. */
          readonly operation: "update";
          /** The row before the update. */
          readonly before: RowImage<Definition>;
          /** The row after the update. */
          readonly after: RowImage<Definition>;
      }
    | {
          /** Delete the row. */
          readonly operation: "delete";
          /** The row before the delete. */
          readonly before: RowImage<Definition>;
      }
);

/** One committed write, at its position in commit order. */
export type Change<Definition extends Table = Table> = Write<Definition> & {
    /** The change's position in commit order. */
    readonly sequence: number;
    /** The origin whose writes a replica's write replicated, absent for this database's own writes. */
    readonly origin?: string;
};

/** Read changes. */
export const Change = {
    /** Read a write's latest image of its row: the row after an insert or update, the row before a delete. */
    image<Definition extends Table>(change: Write<Definition>): RowImage<Definition> {
        return change.operation === "delete" ? change.before : change.after;
    },

    /** Read the row before a write, null for an insert. */
    before<Definition extends Table>(change: Write<Definition>): RowImage<Definition> | null {
        return change.operation === "insert" ? null : change.before;
    },

    /** Read the row after a write, null for a delete. */
    after<Definition extends Table>(change: Write<Definition>): RowImage<Definition> | null {
        return change.operation === "delete" ? null : change.after;
    },

    /** Report whether a change is to one table, typing its rows by it. */
    of<Definition extends Table>(change: Change, table: Definition): change is Change<Definition> {
        return change.table === table;
    },
};

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
    /** The most changes a page carries before completing its last transaction. */
    readonly limit?: number;
    /** The scopes to read. */
    readonly scopes?: readonly string[];
}

/** A JSON column of the log, as SQLite's text or PostgreSQL's parsed value. */
function logJson<Value>(inner: schema.Schema<Value>): schema.Schema<Value> {
    return schema.union([
        schema
            .string()
            .transform((text): unknown => JSON.parse(text))
            .pipe(inner),
        inner,
    ]);
}

/** One committed change as the log keeps it. */
const LogEntry = schema.looseObject({
    /** The entry's sequence. */
    sequence: LogInteger,
    /** The committing transaction. */
    transaction: schema.string().nullable(),
    /** The changed table's SQL name. */
    table: schema.string(),
    /** The encoded primary key, in key order. */
    key: logJson(schema.array(schema.json())),
    /** Insert, update or delete. */
    operation: schema.enum(["insert", "update", "delete"]),
    /** The encoded row by column name. */
    row: logJson(schema.record(schema.string(), schema.json())),
    /** The values an update changed, before it, by column name. */
    previous: logJson(schema.record(schema.string(), schema.json())).nullable(),
    /** The changed row's scope. */
    scope: schema.string(),
    /** The change time. */
    changed_at: LogInteger,
    /** The origin whose writes a replica's write replicated, null for the database's own. */
    origin: schema.string().nullable(),
});
/** One committed change as the log keeps it. */
type LogEntry = schema.Output<typeof LogEntry>;

/** The newest and the highest compacted sequence beside a page of entries. */
const LogBounds = schema.looseObject({
    /** The newest logged sequence. */
    logged: LogInteger.nullable(),
    /** The highest compacted sequence, zero before any compaction. */
    horizon: LogInteger,
});

/** The latest sequence of some entries, null over none. */
const SequenceRow = schema.looseObject({ sequence: LogInteger.nullable() });

/** The hexadecimal digits of a layout digest: 128 bits, whose collisions are negligible among any database's tables. */
const LAYOUT_LENGTH = 32;

/** The logged columns of each table, described once. */
const COLUMNS = new WeakMap<Table, unknown[]>();

/** The committed changes of one database, in commit order. */
export class Log {
    /** The database keeping the log. */
    readonly database: DatabaseConnection;

    /** Create the log of a database. */
    constructor(database: DatabaseConnection) {
        this.database = database;
    }

    /** Create the log once, with the scope of the rows in tables without a scope column, such as a resource's space. */
    async create(scope?: string): Promise<void> {
        await this.database.executeScript(
            createLog(this.database.dialect, v7(), scope).join(";\n"),
        );
    }

    /** Digest the layout tables' changes hold: each one's logged columns with their kinds and values. */
    static async layout(tables: readonly Table[]): Promise<string> {
        // describe each table's logged columns once
        const described = tables.map((table) => {
            let columns = COLUMNS.get(table);
            if (columns === undefined) {
                columns = Object.values(table[TABLE].logged).map(({ definition }) => [
                    definition.name,
                    definition.kind,
                    definition.nullable,
                    toJsonSchema(definition.json),
                ]);
                COLUMNS.set(table, columns);
            }

            return [table[TABLE].sqlName, columns];
        });

        return (await Digest.json(described)).slice(0, LAYOUT_LENGTH);
    }

    /** Read the images a table's rows had before their first change between two sequences, by key. */
    async images(table: Table, after: number, upto: number): Promise<Map<string, Row | null>> {
        return imagesOf(table, await this.range({ tables: [table], after }, upto));
    }

    /** Read images through one shared memory of each table's changes. */
    rewind(): Rewind {
        const known = new Map<Table, { after: number; upto: number; changes: Change[] }>();

        return async (table, after, upto) => {
            // read the missing changes
            const read = known.get(table) ?? { after, upto: after, changes: [] };
            const earlier =
                after < read.after ? await this.range({ tables: [table], after }, read.after) : [];
            const later =
                upto > read.upto
                    ? await this.range({ tables: [table], after: read.upto }, upto)
                    : [];
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

    /** Forget the reads each change of some tables affects until the signal aborts, clearing them all at the start and after compaction. */
    async invalidate<Definition extends Table>(
        tables: readonly Definition[],
        reads: {
            /** Forget every read. */
            readonly clear: () => void;
            /** Forget the reads one change affects. */
            readonly forget: (change: Change<Definition>) => void;
        },
        signal: AbortSignal,
    ): Promise<void> {
        while (!signal.aborted) {
            // forget every read, and follow the changes committed since
            const after = (await this.position()).sequence;
            reads.clear();
            try {
                for await (const page of this.follow({ tables, after }, signal)) {
                    for (const change of page.changes) {
                        reads.forget(change);
                    }
                }
            } catch (error) {
                // start over once the changes were compacted away
                if (!(error instanceof DatabaseError && error.code === "CHANGES_COMPACTED")) {
                    throw error;
                }
            }
        }
    }

    /** Read the changes after a sequence through another, page by page, refusing a range past the log's head. */
    async range<Definition extends Table>(
        selection: Omit<ChangeSelection<Definition>, "limit">,
        through: number,
    ): Promise<Change<Definition>[]> {
        const changes: Change<Definition>[] = [];
        for (let reached = selection.after; reached < through;) {
            // read the next page, keeping its changes through the end
            const read = await this.read({ ...selection, after: reached });
            changes.push(...read.changes.filter((change) => change.sequence <= through));
            if (read.sequence <= reached) {
                throw new DatabaseError(
                    "INVALID_QUERY",
                    `the log ends at ${read.sequence}, before ${through}`,
                );
            }
            reached = read.sequence;
        }

        return changes;
    }

    /** Show the database's logged columns as they were at a position. */
    at(position: LogPosition, rewind?: Rewind): Snapshot {
        return new Snapshot(this.database, position, rewind);
    }

    /** Read the position of the latest commit, or of the latest change not replicating an origin's writes when given, as a follower of that origin reaches it. */
    async position(origin?: string): Promise<LogPosition> {
        // read the head
        const [row] = await this.database.execute(selectHead(this.database.dialect));
        const head = LogBounds.extend({ epoch: schema.string() }).parse(row);
        if (origin === undefined) {
            return { epoch: head.epoch, sequence: latestOf(head.logged, head.horizon) };
        }

        // read the latest change of another origin, or the horizon once compacted
        const [other] = await this.database.execute(sql`
            SELECT max(sequence) AS sequence FROM ${sql.identifier(LOG)}
            WHERE origin IS NULL OR origin <> ${origin}
        `);
        const { sequence } = SequenceRow.parse(other);

        return { epoch: head.epoch, sequence: latestOf(sequence, head.horizon) };
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
        const [row] = await this.database.execute(sql`
            SELECT max(sequence) AS sequence FROM ${sql.identifier(LOG)}
            WHERE "transaction" = (SELECT id FROM ${sql.identifier(LOG_TRANSACTION)} WHERE slot = 1)
        `);
        const { sequence } = SequenceRow.parse(row);

        return sequence === null
            ? committed
            : { epoch: committed.epoch, sequence: Math.max(committed.sequence, sequence) };
    }

    /** Read the open transaction's log identifier as an SQL expression. */
    stamp(): SQL {
        // require an open transaction
        if (!this.database.driver.transaction) {
            throw new TypeError("read the transaction identity inside a transaction");
        }

        // read the SQLite transaction stamp
        const dialect = this.database.dialect;
        if (dialect === "sqlite") {
            return this.database.state.isLogged
                ? sql`(SELECT id FROM ${sql.identifier(LOG_TRANSACTION)} WHERE slot = 1)`
                : sql`NULL`;
        }
        // read PostgreSQL's transaction identifier
        else if (dialect === "postgresql") {
            return sql`pg_current_xact_id_if_assigned()::TEXT`;
        }
        // reject other dialects
        else {
            return assertNever(dialect);
        }
    }

    /** Write as a replica: log the rows a source already derived, deriving nothing again, each change under the origin whose writes it replicates when given. */
    async asReplica<Value>(run: () => Promise<Value>, origin?: string): Promise<Value> {
        // mark the open transaction with the origin
        if (!this.database.driver.transaction) {
            throw new TypeError("write as a replica inside a transaction");
        }
        const marker = sql.identifier(LOG_REPLICA);
        await this.database.execute(
            sql`INSERT INTO ${marker} (slot, origin) VALUES (1, ${origin ?? null})`,
        );
        const result = await run();
        await this.database.execute(sql`DELETE FROM ${marker} WHERE slot = 1`);

        return result;
    }

    /** Read the log's epoch. */
    async epoch(): Promise<string> {
        const [row] = await this.database.execute(
            sql`SELECT epoch FROM ${sql.identifier(LOG_EPOCH)} WHERE slot = 1`,
        );

        return schema.looseObject({ epoch: schema.string() }).parse(row).epoch;
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
        const bounds = LogBounds.parse(rows[0]);
        const horizon = bounds.horizon;
        const isCompacted = selection.tables.some((table) => table[TABLE].retention === "window");
        if (isCompacted && selection.after < horizon) {
            throw new DatabaseError(
                "CHANGES_COMPACTED",
                `changes after ${selection.after} were compacted; list the tables again`,
            );
        }

        // complete the last transaction of a full page
        let entries = presentEntries(rows);
        const last = entries.at(-1);
        if (entries.length === limit && last !== undefined && last.transaction !== null) {
            const rest = await this.#entries(selection, tables, last.sequence, {
                transaction: last.transaction,
            });
            entries = [...entries, ...presentEntries(rest)];
        }

        // decode each entry
        const dialect = this.database.dialect;
        const changes = entries.map((entry) => ({
            sequence: entry.sequence,
            ...(entry.origin === null ? {} : { origin: entry.origin }),
            ...decodeChange(entry, tableOf(tables, entry.table), dialect),
        }));

        // skip other tables' changes unless the page is full
        const end = changes.at(-1)?.sequence ?? selection.after;
        const sequence =
            changes.length >= limit ? end : Math.max(end, latestOf(bounds.logged, bounds.horizon));

        return { changes, sequence };
    }

    /** Read the open transaction's identity, absent outside a writing transaction. */
    async transaction(): Promise<string | undefined> {
        const [row] = await this.database.execute(sql`SELECT ${this.stamp()} AS id`);

        return schema.looseObject({ id: schema.string().nullable() }).parse(row).id ?? undefined;
    }

    /** Read the open transaction's changes before commit. */
    async written<Definition extends Table>(
        tables: readonly Definition[],
    ): Promise<Write<Definition>[]> {
        // require a log and a writing transaction
        if (this.database.dialect === "sqlite" && !this.database.state.isLogged) {
            throw new TypeError("read written changes of a logged database");
        }
        const transaction = await this.transaction();
        if (transaction === undefined) {
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
        const rows = await this.database.execute(sql`
            SELECT sequence, "transaction", "table", key, operation, "row", previous, scope, changed_at, origin
            FROM ${sql.identifier(LOG)}
            WHERE "transaction" = ${transaction} AND "table" IN (${names}) AND ${written}
        `);

        return rows.map((row) => {
            const entry = LogEntry.extend({ sequence: LogInteger.nullable() }).parse(row);

            return decodeChange(entry, tableOf(byName, entry.table), this.database.dialect);
        });
    }

    /** Wait until the log reaches a sequence, returning false once the signal aborts. */
    async wait(sequence: number, signal: AbortSignal): Promise<boolean> {
        // wait on the connection
        if (this.database.driver.transaction) {
            throw new TypeError("wait for changes outside a transaction");
        }

        return this.until(async () => (await this.position()).sequence >= sequence, signal);
    }

    /** Wait until a check passes after a commit, returning false once the signal aborts. */
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

    /** Move a consumer's slot to a position, keeping the changes after it until a time, as a PostgreSQL replication slot keeps its WAL. */
    async advance(slot: string, sequence: number, expiresAt: number): Promise<void> {
        const table = sql.identifier(LOG_SLOT);
        await this.database.execute(sql`
            INSERT INTO ${table} (name, sequence, expires_at)
            VALUES (${slot}, ${sequence}, ${expiresAt})
            ON CONFLICT (name) DO UPDATE SET sequence = excluded.sequence, expires_at = excluded.expires_at
        `);
    }

    /** Read the position of a consumer's slot, absent without one. */
    async slot(slot: string): Promise<number | undefined> {
        const [row] = await this.database.execute(
            sql`SELECT sequence FROM ${sql.identifier(LOG_SLOT)} WHERE name = ${slot}`,
        );

        return row === undefined
            ? undefined
            : schema.looseObject({ sequence: LogInteger }).parse(row).sequence;
    }

    /** Drop a consumer's slot, keeping no more changes for it. */
    async drop(slot: string): Promise<void> {
        await this.database.execute(
            sql`DELETE FROM ${sql.identifier(LOG_SLOT)} WHERE name = ${slot}`,
        );
    }

    /** Delete old windowed changes no slot keeps, and advance the horizon. */
    async compact(before: number, now = Date.now()): Promise<void> {
        await this.database.transaction(async (transaction) => {
            // find the newest removable change
            const log = sql.identifier(LOG);
            const [newestRow] = await transaction.execute(sql`
                SELECT max(sequence) AS sequence
                FROM ${log}
                WHERE retention = 'window'
                    AND changed_at < ${before}
                    AND sequence IS NOT NULL
            `);
            const newest = SequenceRow.parse(newestRow).sequence;
            if (newest === null) {
                return;
            }

            // keep the changes slots keep
            const [keptRow] = await transaction.execute(sql`
                SELECT
                    (SELECT min(sequence) FROM ${sql.identifier(LOG_SLOT)} WHERE expires_at > ${now}) AS sequence,
                    (SELECT sequence FROM ${sql.identifier(LOG_HORIZON)} WHERE slot = 1) AS horizon
            `);
            const kept = SequenceRow.extend({ horizon: LogInteger }).parse(keptRow);
            const cap = kept.sequence === null ? newest : Math.min(newest, kept.sequence);
            if (cap <= kept.horizon) {
                return;
            }

            // stop before a split transaction
            const [splitRow] = await transaction.execute(sql`
                SELECT min(earlier.sequence) AS first
                FROM ${log} change
                JOIN ${log} later ON later."transaction" = change."transaction" AND later.sequence > change.sequence
                JOIN ${log} earlier ON earlier."transaction" = change."transaction"
                WHERE change.sequence = ${cap}
            `);
            const first = schema
                .looseObject({ first: LogInteger.nullable() })
                .parse(splitRow).first;
            const sequence = first === null ? cap : first - 1;

            // remove the windowed changes of whole transactions
            await transaction.execute(sql`
                DELETE FROM ${log}
                WHERE retention = 'window'
                    AND sequence <= ${sequence}
            `);
            await transaction.execute(sql`
                UPDATE ${sql.identifier(LOG_HORIZON)} SET sequence = ${sequence}
                WHERE slot = 1 AND sequence < ${sequence}
            `);
        });
    }

    /** Read the positions and times of a change's transaction. */
    async bounds(sequence: number): Promise<TransactionBounds> {
        // read the transaction's first and last change
        const log = sql.identifier(LOG);
        const [row] = await this.database.execute(sql`
            SELECT min(sequence) AS first, max(sequence) AS last,
                min(changed_at) AS "startedAt", max(changed_at) AS "committedAt"
            FROM ${log}
            WHERE sequence = ${sequence}
                OR "transaction" = (SELECT "transaction" FROM ${log} WHERE sequence = ${sequence})
        `);
        const bounds = schema
            .looseObject({
                first: LogInteger.nullable(),
                last: LogInteger.nullable(),
                startedAt: LogInteger.nullable(),
                committedAt: LogInteger.nullable(),
            })
            .parse(row);
        if (
            bounds.first === null ||
            bounds.last === null ||
            bounds.startedAt === null ||
            bounds.committedAt === null
        ) {
            throw new DatabaseError("CHANGES_COMPACTED", `change ${sequence} is not in the log`);
        }

        return {
            before: bounds.first - 1,
            after: bounds.last,
            startedAt: bounds.startedAt,
            committedAt: bounds.committedAt,
        };
    }

    /** Read log entries after a sequence, or one transaction's rest, with the bounds. */
    async #entries(
        selection: ChangeSelection<Table>,
        tables: ReadonlyMap<string, Table>,
        after: number,
        options: { readonly limit?: number; readonly transaction?: string },
    ): Promise<Record<string, unknown>[]> {
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
        });
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

    // select the bounds and entries
    const log = sql.identifier(LOG);
    const statement = new Statement(
        (value) => sql`
            WITH bounds AS (
                SELECT
                    (SELECT max(sequence) FROM ${log}) AS logged,
                    (SELECT sequence FROM ${sql.identifier(LOG_HORIZON)} WHERE slot = 1) AS horizon
            ), entries AS (
                SELECT sequence, "transaction", "table", key, operation, "row", previous, scope, changed_at, origin
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

/** Match a column against the values of a JSON array. */
function listed(column: SQL, array: SQLWrapper): SQL {
    return dialectSQL({
        sqlite: sql`${column} IN (SELECT value FROM json_each(${array}))`,
        postgresql: sql`${column} IN (SELECT jsonb_array_elements_text(${array}::jsonb))`,
    });
}

/** Decode a log entry through its table's columns. */
function decodeChange<Definition extends Table>(
    entry: Pick<
        LogEntry,
        "transaction" | "table" | "key" | "operation" | "row" | "previous" | "scope" | "changed_at"
    >,
    table: Definition,
    dialect: Dialect,
): Write<Definition> {
    // decode the key, and the row after the change and before an update
    const key = decodeKey(table, entry.key, dialect);
    const logged = decodeLogged(table, entry.row, dialect);
    const common = {
        transaction: entry.transaction,
        table,
        key,
        scope: entry.scope,
        changedAt: entry.changed_at,
    };

    // keep the row after an insert, before a deletion, and both around an update
    if (entry.operation === "insert") {
        return { ...common, operation: "insert", after: logged };
    } else if (entry.operation === "delete") {
        return { ...common, operation: "delete", before: logged };
    }
    const before = decodeLogged(table, { ...entry.row, ...entry.previous }, dialect);

    return { ...common, operation: "update", before, after: logged };
}

/** Decode a logged row's values by column name, whose signature types it by its table. */
function decodeLogged<Definition extends Table>(
    table: Definition,
    row: Readonly<Record<string, JsonValue>>,
    dialect: Dialect,
): RowImage<Definition>;
/**
 * Decode a logged row's values by column name through its logged columns.
 *
 * @construct each logged column decodes its value into the column's type, which is how RowImage maps the table.
 */
function decodeLogged(
    table: Table,
    row: Readonly<Record<string, JsonValue>>,
    dialect: Dialect,
): Record<string, ColumnValue> {
    return Object.fromEntries(
        Object.entries(table[TABLE].logged).map(([property, column]) => {
            // read a column added after the change as null
            const value = row[column.definition.name];
            if (value === null || value === undefined) {
                return [property, null];
            }

            // decode bytes from hexadecimal text and other values by their column
            return [
                property,
                column.definition.kind === "binary"
                    ? Uint8Array.fromHex(schema.string().parse(value))
                    : column.definition.decode(value, dialect),
            ];
        }),
    );
}

/** Decode a logged key's values in key order, whose signature types it by its table. */
function decodeKey<Definition extends Table>(
    table: Definition,
    key: readonly JsonValue[],
    dialect: Dialect,
): Key<Definition>;
/**
 * Decode a logged key's values in key order through the key columns.
 *
 * @construct the key columns decode the values in key order into their types, which is how Key maps the table.
 */
function decodeKey(
    table: Table,
    key: readonly JsonValue[],
    dialect: Dialect,
): Record<string, ColumnValue> {
    return Object.fromEntries(
        table[TABLE].key.map((property, index) => {
            const value = key[index];
            if (value === undefined) {
                throw new DatabaseError(
                    "INVALID_QUERY",
                    `logged key of ${table[TABLE].name} lacks ${property}`,
                );
            }

            return [property, table[TABLE].column(property).definition.decode(value, dialect)];
        }),
    );
}

/** Read the entries of a page, leaving out the bounds row of an empty page. */
function presentEntries(rows: readonly Record<string, unknown>[]): LogEntry[] {
    return rows.filter((row) => row["sequence"] !== null).map((row) => LogEntry.parse(row));
}

/** Find the table an entry changed, failing for a table the selection does not read. */
function tableOf<Definition extends Table>(
    tables: ReadonlyMap<string, Definition>,
    name: string,
): Definition {
    const table = tables.get(name);
    if (table === undefined) {
        throw new DatabaseError(
            "INVALID_QUERY",
            `the log changed a table the read does not select: ${name}`,
        );
    }

    return table;
}

/** Keep each row's image before its first change, null for an insert. */
function imagesOf(table: Table, changes: readonly Change[]): Map<string, Row | null> {
    const images = new Map<string, Row | null>();
    for (const change of changes) {
        const key = Key.name(table, change.key);
        if (!images.has(key)) {
            images.set(key, Change.before(change));
        }
    }

    return images;
}
