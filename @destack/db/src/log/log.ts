import { sql, type SQL } from "../sql/index.ts";
import { listed, Statement } from "../query/statement.ts";
import type { DatabaseConnection } from "../database/connection.ts";
import { TABLE, type RowImage, Table } from "../table/table.ts";
import type { Dialect } from "../dialect/dialect.ts";
import { assertNever, DatabaseError } from "../error/error.ts";
import {
    COMMIT_LOCK,
    LOG_EPOCH,
    LOG_SLOT,
    LOG_HORIZON,
    LOG_REPLICA,
    LOG,
    LOG_TRANSACTION,
    insertEpoch,
} from "./schema.ts";
import { createLog } from "./trigger.ts";
import { committedEntry, latestOf, LogColumn, selectHead, type LogPosition } from "./position.ts";
import { Snapshot, type Rewind } from "./snapshot.ts";
import type { Row } from "../table/row.ts";
import { Key } from "../query/key.ts";
import { Digest, type DigestCrypto, mapValues, toJsonSchema, Uuid } from "@destack/schema";

/** A consumer's claimed page: where it ends, and when it was claimed. */
export interface LogClaim {
    /** The log sequence the page ends at. */
    readonly through: number;
    /** When the page was claimed, in Unix milliseconds. */
    readonly claimedAt: number;
}

/** Claims as the slot table stores them. */
export const LogClaim = {
    /** Read a claim from its slot row. */
    read(row: Readonly<Record<string, unknown>>): LogClaim {
        return {
            through: LogColumn.integer(row["claim"]),
            claimedAt: LogColumn.integer(row["claimed_at"]),
        };
    },
};

/** The default page size of a log read, in changes. */
const PAGE_LIMIT = 1000;

/** The column of each dialect's log holding an entry's write-order id. */
const LOG_ID: Readonly<Record<Dialect, string>> = { sqlite: "rowid", postgresql: "id" };

/** One logged write: an insert with the row after, an update with both, a delete with before. */
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
};

/** Read changes. */
export const Change = {
    /** Read a write's latest row image: the row after an insert or update, before a delete. */
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

/** The write-order id of a log entry: the row id on SQLite, the identity column on PostgreSQL. */
export type LogId = number;

/** The tables and log id of a read of the open transaction's own changes. */
export interface TransactionSelection<Definition extends Table> {
    /** The logged tables to read. */
    readonly tables: readonly Definition[];
    /** The log id read last, the transaction's start when absent. */
    readonly after?: LogId;
}

/** The open transaction's own changes after a log id. */
export interface TransactionPage<Definition extends Table = Table> {
    /** The changes in write order. */
    readonly changes: readonly Write<Definition>[];
    /** The log id the next read continues after. */
    readonly after: LogId;
}

/** The tables and position of a change read. */
export interface ChangeSelection<Definition extends Table> {
    /** The logged tables to read. */
    readonly tables: readonly Definition[];
    /** The consumed sequence, zero for the start. */
    readonly after: number;
    /** The most changes a page has before completing its last transaction. */
    readonly limit?: number;
    /** The scopes to read. */
    readonly scopes?: readonly string[];
}

/** One committed change as the log stores it. */
interface LogEntry {
    /** The entry's sequence, null for PostgreSQL's open transaction until it commits. */
    readonly sequence: number | null;
    /** The committing transaction. */
    readonly transaction: string | null;
    /** The changed table's SQL name. */
    readonly table: string;
    /** The encoded primary key, in key order. */
    readonly key: readonly unknown[];
    /** Insert, update or delete. */
    readonly operation: "insert" | "update" | "delete";
    /** The encoded row by column name, which its columns decode. */
    readonly row: Readonly<Record<string, unknown>>;
    /** The values an update changed, before it, by column name. */
    readonly previous: Readonly<Record<string, unknown>> | null;
    /** The changed row's scope. */
    readonly scope: string;
    /** The change time. */
    readonly changedAt: number;
}

/** The log's change entries. */
const LogEntry = {
    /** Read an entry by its columns' driver types, since the log writes every entry itself. */
    read(row: Readonly<Record<string, unknown>>, dialect: Dialect): LogEntry {
        const previous = row["previous"];

        return {
            sequence: LogColumn.nullableInteger(row["sequence"]),
            transaction: LogColumn.nullableText(row["transaction"]),
            table: LogColumn.text(row["table"]),
            key: LogColumn.array(row["key"], dialect),
            operation: LogEntry.operation(row["operation"]),
            row: LogColumn.object(row["row"], dialect),
            previous: previous === null ? null : LogColumn.object(previous, dialect),
            scope: LogColumn.text(row["scope"]),
            changedAt: LogColumn.integer(row["changed_at"]),
        };
    },

    /** Read an entry's operation, failing for any other value. */
    operation(value: unknown): LogEntry["operation"] {
        if (value !== "insert" && value !== "update" && value !== "delete") {
            throw DatabaseError.of(
                "QUERY_FAILED",
                `expected an operation from the log, got ${String(value)}`,
            );
        }

        return value;
    },
};

/** The hex digits of a layout digest: 128 bits, with negligible collisions among tables. */
const LAYOUT_LENGTH = 32;

/** The committed changes of one database, in commit order. */
export class Log {
    /** The database that stores the log. */
    readonly database: DatabaseConnection;

    /** Create the log of a database. */
    constructor(database: DatabaseConnection) {
        this.database = database;
    }

    /** Create the log once, with the row scope of tables without a scope column. */
    async create(scope?: string): Promise<void> {
        const { dialect, state } = this.database;
        const epoch = insertEpoch(Uuid.v7(state.environment), state.namespace, scope);
        await this.database.executeScript(
            [...createLog(dialect, state.namespace), epoch].join(";\n"),
        );
    }

    /** Digest the layout of tables' changes: logged columns with their kinds and values. */
    static async layout(tables: readonly Table[], crypto: DigestCrypto): Promise<string> {
        // describe each table's logged columns
        const described = tables.map((table) => [
            table[TABLE].sqlName,
            Object.values(table[TABLE].logged).map(({ definition }) => [
                definition.name,
                definition.kind,
                definition.nullable,
                toJsonSchema(definition.json),
            ]),
        ]);

        return (await Digest.json(described, crypto)).slice(0, LAYOUT_LENGTH);
    }

    /** Read a table's row images before their first change between two sequences, by key. */
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

            // take each row's image before its first change
            const range = changes.filter(
                (change) => change.sequence > after && change.sequence <= upto,
            );

            return imagesOf(table, range);
        };
    }

    /** Forget the reads each change of some tables affects until the signal aborts. */
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
                if (!DatabaseError.is(error, "CHANGES_COMPACTED")) {
                    throw error;
                }
            }
        }
    }

    /** Read the changes after a sequence through another by page, refusing ranges past the head. */
    async range<Definition extends Table>(
        selection: Omit<ChangeSelection<Definition>, "limit">,
        through: number,
    ): Promise<Change<Definition>[]> {
        const changes: Change<Definition>[] = [];
        for (let reached = selection.after; reached < through;) {
            // read the next page, with its changes through the end
            const read = await this.read({ ...selection, after: reached });
            changes.push(...read.changes.filter((change) => change.sequence <= through));
            if (read.sequence <= reached) {
                throw DatabaseError.of(
                    "QUERY_INVALID",
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

    /** Read the position of the latest commit, from memory until a commit writes the log. */
    async position(): Promise<LogPosition> {
        // read the head, with the tables written since the watch caught up on a shared log
        const { commits } = this.database.state;
        const caught = commits.caughtUp;
        const isCatching = commits.isShared && this.database.driver.transaction === undefined;
        const statement =
            isCatching && caught !== undefined ? this.#headSince(caught.sequence) : this.#head();
        const head = await this.#readHead(statement);
        const sequence = latestOf(head.logged, head.horizon);
        const position = { log: head.log, epoch: head.epoch, sequence };

        // leave a sole writer's or a transaction's position as read
        if (!isCatching) {
            return position;
        }

        // start catching up at the head, the watch having heard every commit since delivery began
        if (caught === undefined) {
            commits.catchUp(position, []);
        }
        // wake every table after a restart, or once compaction removed the changes
        else if (caught.epoch !== head.epoch || caught.sequence < head.horizon) {
            commits.catchUp(position, "every");
        }
        // wake the tables of the commits the watch has yet to hear
        else if (sequence > caught.sequence) {
            commits.catchUp(position, head.written);
        }

        return position;
    }

    /** Select the log's head. */
    #head(): SQL {
        return selectHead(this.database.dialect);
    }

    /** Select the log's head with the tables commits after a sequence wrote, comma separated. */
    #headSince(after: number): SQL {
        const written =
            this.database.dialect === "sqlite"
                ? sql`group_concat(DISTINCT "table")`
                : sql`string_agg(DISTINCT "table", ',')`;

        return sql`${this.#head()}, (SELECT ${written} FROM ${sql.relation(LOG)} WHERE sequence > ${after}) AS written`;
    }

    /** Read the position of some scopes' latest change to some tables, or the horizon. */
    async changed(tables: readonly Table[], scopes: readonly string[]): Promise<LogPosition> {
        // read the newest committed entry of the scopes in the tables through the log's scope index
        const head = await this.#readHead(this.#head());
        const names = tables.map((table) => sql`${table[TABLE].sqlName}`);
        const scoped = sql.join(
            scopes.map((scope) => sql`${scope}`),
            sql`, `,
        );
        const [row] =
            names.length === 0 || scopes.length === 0
                ? []
                : await this.database.all(sql`
            SELECT sequence FROM ${sql.relation(LOG)} AS entry
            WHERE scope IN (${scoped}) AND "table" IN (${sql.join(names, sql`, `)})
                AND sequence IS NOT NULL AND ${committedEntry(this.database.dialect, sql`entry`)}
            ORDER BY sequence DESC LIMIT 1
        `);
        const sequence = row === undefined ? null : LogColumn.integer(row["sequence"]);

        return { log: head.log, epoch: head.epoch, sequence: latestOf(sequence, head.horizon) };
    }

    /** Read the log's head row: its log, epoch, latest sequence and horizon. */
    async #readHead(statement: SQL): Promise<{
        readonly log: string;
        readonly epoch: string;
        readonly logged: number | null;
        readonly horizon: number;
        readonly written: readonly string[];
    }> {
        const [row] = await this.database.all(statement);
        const written = row?.["written"];

        return {
            log: LogColumn.text(row?.["log"]),
            epoch: LogColumn.text(row?.["epoch"]),
            logged: LogColumn.nullableInteger(row?.["logged"]),
            horizon: LogColumn.integer(row?.["horizon"]),
            written: typeof written === "string" ? written.split(",") : [],
        };
    }

    /** Read the open transaction's log identifier as an SQL expression. */
    stamp(): SQL {
        // require an open transaction
        if (!this.database.driver.transaction) {
            throw new TypeError("cannot read the transaction identity outside a transaction");
        }

        // read the SQLite transaction stamp
        const dialect = this.database.dialect;
        if (dialect === "sqlite") {
            return this.database.state.isLogged
                ? sql`(SELECT id FROM ${sql.relation(LOG_TRANSACTION)} WHERE slot = 1)`
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

    /** Write as a replica: log the rows a source already derived, deriving nothing again. */
    async asReplica<Value>(run: () => Promise<Value>): Promise<Value> {
        // mark the open transaction as a replica's
        if (!this.database.driver.transaction) {
            throw new TypeError("cannot write as a replica outside a transaction");
        }
        const marker = sql.relation(LOG_REPLICA);
        await this.database.run(sql`INSERT INTO ${marker} (slot) VALUES (1)`);
        const result = await run();
        await this.database.run(sql`DELETE FROM ${marker} WHERE slot = 1`);

        return result;
    }

    /** Block the log's other commits until this transaction ends. */
    async serialize(): Promise<void> {
        if (this.database.dialect === "postgresql") {
            await this.database.run(
                sql`SELECT pg_advisory_xact_lock(${sql.raw(String(COMMIT_LOCK))}, hashtext(current_schema()))`,
            );
        }
    }

    /** Start a new epoch after a restore. */
    async renew(): Promise<string> {
        const epoch = Uuid.v7(this.database.state.environment);
        await this.database.run(
            sql`UPDATE ${sql.relation(LOG_EPOCH)} SET epoch = ${epoch} WHERE slot = 1`,
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
        const bounds = {
            logged: LogColumn.nullableInteger(rows[0]?.["logged"]),
            horizon: LogColumn.integer(rows[0]?.["horizon"]),
        };
        const horizon = bounds.horizon;
        const isCompacted = selection.tables.some((table) => table[TABLE].retention === "window");
        if (isCompacted && selection.after < horizon) {
            throw DatabaseError.of(
                "CHANGES_COMPACTED",
                `changes after ${selection.after} were compacted: list the tables again`,
            );
        }

        // complete the last transaction of a full page
        let entries = presentEntries(rows, this.database.dialect);
        const last = entries.at(-1);
        if (entries.length === limit && last !== undefined && last.transaction !== null) {
            const rest = await this.#entries(selection, tables, last.sequence, {
                transaction: last.transaction,
            });
            entries = [...entries, ...presentEntries(rest, this.database.dialect)];
        }

        // decode each entry
        const dialect = this.database.dialect;
        const changes = entries.map((entry) => ({
            sequence: entry.sequence,
            ...decodeChange(entry, tableOf(tables, entry.table), dialect),
        }));

        // skip other tables' changes unless the page is full
        const end = changes.at(-1)?.sequence ?? selection.after;
        const sequence =
            changes.length >= limit ? end : Math.max(end, latestOf(bounds.logged, bounds.horizon));

        return { changes, sequence };
    }

    /** Read the open transaction's identity, stamping it, absent outside a transaction or a log. */
    async transaction(): Promise<string | undefined> {
        return await this.database.driver.transaction?.identity.id();
    }

    /** Read the open transaction's own changes after a log id, in write order. */
    async readTransaction<Definition extends Table>(
        selection: TransactionSelection<Definition>,
    ): Promise<TransactionPage<Definition>> {
        // stamp the transaction, which finds the log, then require the log and a transaction
        const { dialect } = this.database;
        const after = selection.after ?? 0;
        const transaction = await this.transaction();
        if (dialect === "sqlite" && !this.database.state.isLogged) {
            throw new TypeError("cannot read written changes of an unlogged database");
        }
        if (transaction === undefined) {
            return { changes: [], after };
        }

        // read its entries after the log id in write order
        const byName = new Map(selection.tables.map((table) => [table[TABLE].sqlName, table]));
        const names = sql.join(
            [...byName.keys()].map((name) => sql`${name}`),
            sql`, `,
        );
        const id = sql.raw(LOG_ID[dialect]);
        const rows = await this.database.all(sql`
            SELECT ${id} AS log_id, sequence, "transaction", "table", key, operation, "row", previous, scope, changed_at
            FROM ${sql.relation(LOG)}
            WHERE "transaction" = ${transaction} AND "table" IN (${names}) AND ${id} > ${after}
            ORDER BY ${id}
        `);

        // decode each entry, continuing after the last one read
        const changes = rows.map((row) => {
            const entry = LogEntry.read(row, dialect);

            return decodeChange(entry, tableOf(byName, entry.table), dialect);
        });
        const last = rows.at(-1);

        return { changes, after: last === undefined ? after : LogColumn.integer(last["log_id"]) };
    }

    /** Wait until the log includes a sequence, returning false once the signal aborts. */
    async wait(sequence: number, signal: AbortSignal): Promise<boolean> {
        // wait on the connection
        if (this.database.driver.transaction) {
            throw new TypeError("cannot wait for changes inside a transaction");
        }

        return this.until(async () => await this.#includes(sequence), signal);
    }

    /** Settle once the log hears other writers' commits as they end. */
    listening(): Promise<void> {
        return this.database.state.commits.listen();
    }

    /** Wait until a check of the tables committed since the last check passes after a commit. */
    until(
        check: (tables: ReadonlySet<string>) => Promise<boolean>,
        signal: AbortSignal,
        generation?: number,
    ): Promise<boolean> {
        return this.database.state.commits.until(check, signal, generation);
    }

    /** Yield each page of selected changes past a sequence, or every advance, until aborted. */
    async *follow<Definition extends Table>(
        selection: ChangeSelection<Definition>,
        signal: AbortSignal,
        options: {
            /** Whether to yield each advance as a page without changes, as a keepalive. */
            readonly isAdvancing?: boolean;
        } = {},
    ): AsyncGenerator<ChangePage<Definition>> {
        // yield each page with changes or an advance as a keepalive, then wait for the next commit
        const tables = new Set(selection.tables.map((table) => table[TABLE].sqlName));
        let after = selection.after;
        while (!signal.aborted) {
            // read the page, noting the watch's generation before it
            const { generation } = this.database.state.commits;
            const page = await this.read({ ...selection, after });

            // yield a page with changes, or an advance when asked
            const isYielded =
                page.changes.length > 0 || (options.isAdvancing === true && page.sequence > after);
            const isFull = page.changes.length >= (selection.limit ?? PAGE_LIMIT);
            after = Math.max(after, page.sequence);
            if (isYielded) {
                yield page;
            }

            // read on after a full page, else wait for a commit past it
            if (!isFull) {
                const next = after + 1;
                await this.until(
                    async (committed) =>
                        (options.isAdvancing === true ||
                            [...committed].some((table) => tables.has(table))) &&
                        (await this.#includes(next)),
                    signal,
                    generation,
                );
            }
        }
    }

    /** Report whether the log includes a sequence, reading its head only past the announced one. */
    async #includes(sequence: number): Promise<boolean> {
        return (
            this.database.state.commits.sequence >= sequence ||
            (await this.position()).sequence >= sequence
        );
    }

    /** Place a consumer's slot at a sequence and end its claim, retaining changes until a time. */
    async advance(slot: string, sequence: number, expiresAt: number): Promise<void> {
        const table = sql.relation(LOG_SLOT);
        await this.database.run(sql`
            INSERT INTO ${table} (name, sequence, expires_at)
            VALUES (${slot}, ${sequence}, ${expiresAt})
            ON CONFLICT (name) DO UPDATE SET sequence = excluded.sequence, expires_at = excluded.expires_at, claim = NULL, claimed_at = NULL
        `);
    }

    /** Place a consumer's slot at the log's start unless placed, retaining changes until a time. */
    async place(slot: string, expiresAt: number): Promise<void> {
        await this.database.run(sql`
            INSERT INTO ${sql.relation(LOG_SLOT)} (name, sequence, expires_at)
            VALUES (${slot}, 0, ${expiresAt})
            ON CONFLICT (name) DO NOTHING
        `);
    }

    /** List the names of the slots starting with a prefix, in name order. */
    async slots(prefix: string): Promise<string[]> {
        const rows = await this.database.all(sql`
            SELECT name FROM ${sql.relation(LOG_SLOT)}
            WHERE substr(name, 1, ${prefix.length}) = ${prefix}
            ORDER BY name
        `);

        return rows.map((row) => LogColumn.text(row["name"]));
    }

    /** Move a consumer's slot from the position it read to another. */
    async move(slot: string, from: number, to: number, expiresAt: number): Promise<void> {
        // move a slot at the read position, placing a new one for a consumer from the start
        const table = sql.relation(LOG_SLOT);
        const moved = await this.database.all(
            from === 0
                ? sql`
                    INSERT INTO ${table} (name, sequence, expires_at)
                    VALUES (${slot}, ${to}, ${expiresAt})
                    ON CONFLICT (name) DO UPDATE SET sequence = excluded.sequence, expires_at = excluded.expires_at, claim = NULL, claimed_at = NULL
                    WHERE ${table}.sequence = ${from}
                    RETURNING name
                `
                : sql`
                    UPDATE ${table} SET sequence = ${to}, expires_at = ${expiresAt}, claim = NULL, claimed_at = NULL
                    WHERE name = ${slot} AND sequence = ${from}
                    RETURNING name
                `,
        );

        // refuse a slot another consumer moved since the read
        if (moved.length === 0) {
            throw DatabaseError.of("CONCURRENT_UPDATE", `slot ${slot} moved past ${from}`);
        }
    }

    /** Claim a consumer's page through a sequence unless claimed, absent once the slot moved on. */
    async claim(
        slot: string,
        after: number,
        through: number,
        now: number,
        expiresAt: number,
    ): Promise<LogClaim | undefined> {
        // leave the slot's claim, else claim through the sequence at the read
        const table = sql.relation(LOG_SLOT);
        const [row] = await this.database.all(
            after === 0
                ? sql`
                    INSERT INTO ${table} (name, sequence, expires_at, claim, claimed_at)
                    VALUES (${slot}, 0, ${expiresAt}, ${through}, ${now})
                    ON CONFLICT (name) DO UPDATE SET
                        claim = COALESCE(${table}.claim, excluded.claim),
                        claimed_at = COALESCE(${table}.claimed_at, excluded.claimed_at)
                    WHERE ${table}.sequence = 0
                    RETURNING claim, claimed_at
                `
                : sql`
                    UPDATE ${table}
                    SET claim = COALESCE(claim, ${through}), claimed_at = COALESCE(claimed_at, ${now})
                    WHERE name = ${slot} AND sequence = ${after}
                    RETURNING claim, claimed_at
                `,
        );

        return row === undefined ? undefined : LogClaim.read(row);
    }

    /** Read a consumer's claimed page, absent while it claims none. */
    async claimed(slot: string): Promise<LogClaim | undefined> {
        const [row] = await this.database.all(
            sql`SELECT claim, claimed_at FROM ${sql.relation(LOG_SLOT)} WHERE name = ${slot} AND claim IS NOT NULL`,
        );

        return row === undefined ? undefined : LogClaim.read(row);
    }

    /** Read the position of a consumer's slot, absent without one. */
    async slot(slot: string): Promise<number | undefined> {
        const [row] = await this.database.all(
            sql`SELECT sequence FROM ${sql.relation(LOG_SLOT)} WHERE name = ${slot}`,
        );

        return row === undefined ? undefined : LogColumn.integer(row["sequence"]);
    }

    /** Drop a slot and its claim, retaining no more changes for it. */
    async drop(slot: string): Promise<void> {
        await this.database.run(sql`DELETE FROM ${sql.relation(LOG_SLOT)} WHERE name = ${slot}`);
    }

    /** Delete old windowed changes no slot retains, and advance the horizon. */
    async compact(before: number, now: number): Promise<void> {
        await this.database.transaction(async (transaction) => {
            // find the newest removable change
            const log = sql.relation(LOG);
            const [newestRow] = await transaction.all(sql`
                SELECT max(sequence) AS sequence
                FROM ${log}
                WHERE retention = 'window'
                    AND changed_at < ${before}
                    AND sequence IS NOT NULL
            `);
            const newest = LogColumn.nullableInteger(newestRow?.["sequence"]);
            if (newest === null) {
                return;
            }

            // retain the changes after the earliest live slot
            const [retainedRow] = await transaction.all(sql`
                SELECT
                    (SELECT min(sequence) FROM ${sql.relation(LOG_SLOT)} WHERE expires_at > ${now}) AS sequence,
                    (SELECT sequence FROM ${sql.relation(LOG_HORIZON)} WHERE slot = 1) AS horizon
            `);
            const retained = {
                sequence: LogColumn.nullableInteger(retainedRow?.["sequence"]),
                horizon: LogColumn.integer(retainedRow?.["horizon"]),
            };
            const cap = retained.sequence === null ? newest : Math.min(newest, retained.sequence);
            if (cap <= retained.horizon) {
                return;
            }

            // stop before a split transaction
            const [splitRow] = await transaction.all(sql`
                SELECT min(earlier.sequence) AS first
                FROM ${log} change
                JOIN ${log} later ON later."transaction" = change."transaction" AND later.sequence > change.sequence
                JOIN ${log} earlier ON earlier."transaction" = change."transaction"
                WHERE change.sequence = ${cap}
            `);
            const first = LogColumn.nullableInteger(splitRow?.["first"]);
            const sequence = first === null ? cap : first - 1;

            // remove the windowed changes of whole transactions
            await transaction.run(sql`
                DELETE FROM ${log}
                WHERE retention = 'window'
                    AND sequence <= ${sequence}
            `);
            await transaction.run(sql`
                UPDATE ${sql.relation(LOG_HORIZON)} SET sequence = ${sequence}
                WHERE slot = 1 AND sequence < ${sequence}
            `);
        });
    }

    /** Read the positions and times of a change's transaction. */
    async bounds(sequence: number): Promise<TransactionBounds> {
        // read the transaction's first and last change
        const log = sql.relation(LOG);
        const [row] = await this.database.all(sql`
            SELECT min(sequence) AS first, max(sequence) AS last,
                min(changed_at) AS "startedAt", max(changed_at) AS "committedAt"
            FROM ${log}
            WHERE sequence = ${sequence}
                OR "transaction" = (SELECT "transaction" FROM ${log} WHERE sequence = ${sequence})
        `);
        const bounds = {
            first: LogColumn.nullableInteger(row?.["first"]),
            last: LogColumn.nullableInteger(row?.["last"]),
            startedAt: LogColumn.nullableInteger(row?.["startedAt"]),
            committedAt: LogColumn.nullableInteger(row?.["committedAt"]),
        };
        if (
            bounds.first === null ||
            bounds.last === null ||
            bounds.startedAt === null ||
            bounds.committedAt === null
        ) {
            throw DatabaseError.of("CHANGES_COMPACTED", `change ${sequence} is not in the log`);
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
    const log = sql.relation(LOG);
    const statement = new Statement(
        (value) => sql`
            WITH bounds AS (
                SELECT
                    (SELECT max(sequence) FROM ${log}) AS logged,
                    (SELECT sequence FROM ${sql.relation(LOG_HORIZON)} WHERE slot = 1) AS horizon
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

/** Decode a log entry through its table's columns. */
function decodeChange<Definition extends Table>(
    entry: Pick<
        LogEntry,
        "transaction" | "table" | "key" | "operation" | "row" | "previous" | "scope" | "changedAt"
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
        changedAt: entry.changedAt,
    };

    // take the row after an insert, before a deletion, and both around an update
    if (entry.operation === "insert") {
        return { ...common, operation: "insert", after: logged };
    } else if (entry.operation === "delete") {
        return { ...common, operation: "delete", before: logged };
    }
    const before = decodeLogged(table, { ...entry.row, ...entry.previous }, dialect);

    return { ...common, operation: "update", before, after: logged };
}

/** Decode a logged row's values by column name through its logged columns. */
function decodeLogged<Definition extends Table>(
    table: Definition,
    row: Readonly<Record<string, unknown>>,
    dialect: Dialect,
): RowImage<Definition> & Row {
    const decoded = Object.fromEntries(
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
                    ? Uint8Array.fromHex(LogColumn.text(value))
                    : column.definition.decode(value, dialect),
            ];
        }),
    );

    return Table.image(table, decoded);
}

/** Decode a logged key's values in key order through the key columns. */
function decodeKey<Definition extends Table>(
    table: Definition,
    key: readonly unknown[],
    dialect: Dialect,
): Key<Definition> {
    const decoded: Row = Object.fromEntries(
        table[TABLE].key.map((property, index) => {
            const value = key[index];
            if (value === undefined) {
                throw DatabaseError.of(
                    "QUERY_INVALID",
                    `logged key of ${table[TABLE].name} is missing ${property}`,
                );
            }

            return [property, table[TABLE].column(property).definition.decode(value, dialect)];
        }),
    );

    return mapValues<Row, Key<Definition>>(decoded, (value) => value);
}

/** Read the sequenced entries of a page, leaving out the bounds row of an empty page. */
function presentEntries(
    rows: readonly Record<string, unknown>[],
    dialect: Dialect,
): (LogEntry & { readonly sequence: number })[] {
    return rows
        .filter((row) => row["sequence"] !== null)
        .map((row) => ({
            ...LogEntry.read(row, dialect),
            sequence: LogColumn.integer(row["sequence"]),
        }));
}

/** Find the table an entry changed, failing for a table the selection does not read. */
function tableOf<Definition extends Table>(
    tables: ReadonlyMap<string, Definition>,
    name: string,
): Definition {
    const table = tables.get(name);
    if (table === undefined) {
        throw DatabaseError.of(
            "QUERY_INVALID",
            `the log changed a table the read does not select: ${name}`,
        );
    }

    return table;
}

/** Take each row's image before its first change, null for an insert. */
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
