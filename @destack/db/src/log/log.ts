import { sql, type SQL } from "../sql/index.ts";
import { listed, Statement, textElements } from "../query/statement.ts";
import type { DatabaseConnection } from "../database/connection.ts";
import { TABLE, type RowImage, Table } from "../table/table.ts";
import type { Dialect } from "../dialect/dialect.ts";
import { assertNever, DatabaseError } from "../error/error.ts";
import {
    COMMIT_LOCK,
    LOG_EPOCH,
    LOG_CHECKPOINT,
    LOG_HORIZON,
    LOG_REPLICA,
    LOG,
    LOG_TRANSACTION,
    insertEpoch,
} from "./schema.ts";
import { createLog } from "./trigger.ts";
import { LogEntry, type SequencedEntry } from "./entry.ts";
import type { TailRead } from "./tail.ts";
import { committedEntry, latestOf, LogColumn, selectHead, type LogPosition } from "./position.ts";
import { Snapshot, type Rewind } from "./snapshot.ts";
import type { Row } from "../table/row.ts";
import { Key } from "../query/key.ts";
import {
    Change,
    type ChangePage,
    type ChangeSelection,
    type TransactionBounds,
    type TransactionPage,
    type TransactionSelection,
    type Write,
} from "./change.ts";
import { Digest, type DigestCrypto, mapValues, present, toJsonSchema, Uuid } from "@destack/schema";

/** A named position in some tables' changes, whose checkpoint only their changes hold back. */
export type LogCursor = {
    /** The cursor's name, unique per cursor, which names its checkpoint. */
    readonly name: string;
    /** The tables whose changes the cursor reads. */
    readonly tables: readonly Table[];
};

/** Cursors as the checkpoint table stores them. */
export const LogCursor = {
    /** Write a cursor's tables as the JSON array of their SQL names. */
    tables(cursor: LogCursor): string {
        return JSON.stringify(cursor.tables.map((table) => table[TABLE].sqlName));
    },
};

/** A cursor's claimed page: where it ends, and when it was claimed. */
export interface LogClaim {
    /** The log sequence the page ends at. */
    readonly through: number;
    /** When the page was claimed, in Unix milliseconds. */
    readonly claimedAt: number;
}

/** Claims as the checkpoint table stores them. */
export const LogClaim = {
    /** Read a claim from its checkpoint row. */
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
        // create the log and its epoch, then mark the connection logged
        const { dialect, state } = this.database;
        const epoch = insertEpoch(Uuid.v7(state.environment), state.namespace, scope);
        await this.database.executeScript(
            [...createLog(dialect, state.namespace), epoch].join(";\n"),
        );
        state.logging = "logged";
    }

    /** Digest the layout of a table's changes: its logged columns with their kinds and values. */
    static async layout(table: Table, crypto: DigestCrypto): Promise<string> {
        const described = Object.values(table[TABLE].logged).map(({ definition }) => [
            definition.name,
            definition.kind,
            definition.nullable,
            toJsonSchema(definition.json),
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
                if (!DatabaseError.is(error, "CHANGES_NOT_FOUND")) {
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
                    `log read is invalid: the log ends at ${read.sequence}, before '${through}'`,
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
    position(): Promise<LogPosition> {
        return this.#position();
    }

    /** Run a read of the log in its span, recording by whom and whether it found changes. */
    #readTraced<Page>(
        reader: "tail" | "own",
        run: () => Promise<Page>,
        found: (page: Page) => number,
    ): Promise<Page> {
        // name the reader
        const attributes = {
            "db.namespace": this.database.state.name,
            "destack.log.reader": reader,
        };

        // read, noting whether the read found changes
        let outcome: "changes" | "empty" = "empty";
        const read = async () => {
            const page = await run();
            outcome = found(page) > 0 ? "changes" : "empty";

            return page;
        };

        return this.database.state.scope.span("destack.log.read", attributes, read, {
            duration: {
                attributes: () => ({ ...attributes, "destack.log.read.outcome": outcome }),
            },
        });
    }

    /** Read the position of the latest commit from the log's head. */
    async #position(): Promise<LogPosition> {
        // read the head, with the tables written since the watch caught up on a shared log
        const { commits } = this.database.state;
        const caught = commits.caughtUp;
        const isCatching = commits.isShared && this.database.driver.transaction === undefined;
        const [row] =
            isCatching && caught !== undefined
                ? await HEAD_READS.since.all(this.database, { after: caught.sequence })
                : await HEAD_READS.head.all(this.database);
        const head = Log.#parseHead(row);
        const sequence = latestOf(head.logged, head.horizon);
        const position = { log: head.log, epoch: head.epoch, sequence };

        // keep a head read outside a transaction, which sees committed entries only
        if (this.database.driver.transaction === undefined) {
            commits.observe(position);
        }

        // leave a sole writer's or a transaction's position as read
        if (!isCatching) {
            return position;
        }

        // start catching up at the head, the watch having received every commit since delivery began
        if (caught === undefined) {
            commits.catchUp(position, []);
        }
        // wake every table after a restart, or once compaction removed the changes
        else if (caught.epoch !== head.epoch || caught.sequence < head.horizon) {
            commits.catchUp(position, "every");
        }
        // wake the tables of the commits the watch has yet to receive
        else if (sequence > caught.sequence) {
            commits.catchUp(position, head.written);
        }

        return position;
    }

    /** Read the position of some scopes' latest change to some tables, or the horizon. */
    changed(tables: readonly Table[] | "every", scopes: readonly string[]): Promise<LogPosition> {
        return this.#changed(tables, scopes);
    }

    /** Seek some scopes' latest change to some tables on the log's scope index. */
    async #changed(
        tables: readonly Table[] | "every",
        scopes: readonly string[],
    ): Promise<LogPosition> {
        // seek each scope's newest committed entry beside the head, in one statement
        const [row] =
            tables === "every"
                ? await HEAD_READS.changed.all(this.database, { scopes: JSON.stringify(scopes) })
                : await HEAD_READS.changedIn.all(this.database, {
                      scopes: JSON.stringify(scopes),
                      tables: JSON.stringify(tables.map((table) => table[TABLE].sqlName)),
                  });
        const head = Log.#parseHead(row);
        const changed = LogColumn.nullableInteger(row?.["changed"]);

        return { log: head.log, epoch: head.epoch, sequence: latestOf(changed, head.horizon) };
    }

    /** Parse the log's head row. */
    static #parseHead(row: Readonly<Record<string, unknown>> | undefined): LogHead {
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
            return this.database.state.logging === "logged"
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
        // require an open transaction
        const transaction = this.database.driver.transaction;
        if (transaction === undefined) {
            throw new TypeError("cannot write as a replica outside a transaction");
        }

        // mark the transaction as a replica's at its first write that derives rows
        const marker = sql.relation(LOG_REPLICA);
        let isMarked = false;
        transaction.replicaMark = async () => {
            isMarked = true;
            await this.database.run(sql`INSERT INTO ${marker} (slot) VALUES (1)`);
        };
        try {
            // run the writes, then unmark a marked transaction
            const result = await run();
            if (isMarked) {
                await this.database.run(sql`DELETE FROM ${marker} WHERE slot = 1`);
            }

            return result;
        } finally {
            transaction.replicaMark = undefined;
        }
    }

    /** Block the log's other commits until this transaction ends. */
    async serialize(): Promise<void> {
        if (this.database.dialect === "postgresql") {
            await SERIALIZE.run(this.database, {
                schema: present(this.database.state.schema, "the database's schema"),
            });
        }
    }

    /** Start a new epoch after a restore. */
    async renew(): Promise<string> {
        // write a new epoch
        const epoch = Uuid.v7(this.database.state.environment);
        await this.database.run(
            sql`UPDATE ${sql.relation(LOG_EPOCH)} SET epoch = ${epoch} WHERE slot = 1`,
        );

        // forget the process's tail, cached reads and known head, which read the old epoch
        this.database.state.tail.reset();
        this.database.state.commits.expire();

        return epoch;
    }

    /** Read committed changes after a sequence, ending each page with a whole transaction. */
    read<Definition extends Table>(
        selection: ChangeSelection<Definition>,
    ): Promise<ChangePage<Definition>> {
        return this.#readTraced(
            "own",
            () => this.#read(selection),
            (page) => page.changes.length,
        );
    }

    /** Read a page of committed changes and its bounds in one statement. */
    async #read<Definition extends Table>(
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
                "CHANGES_NOT_FOUND",
                `changes after '${selection.after}' not found: they were compacted, so list the tables again`,
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

        return { changes, sequence, isLast: changes.length < limit };
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
        if (dialect === "sqlite" && this.database.state.logging !== "logged") {
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

        return this.waitUntil(async () => await this.#includes(sequence), signal);
    }

    /** Settle once the log receives other writers' commits as they end. */
    listening(): Promise<void> {
        return this.database.state.commits.listen();
    }

    /** Wait until a check of the tables committed since the last check passes after a commit. */
    waitUntil(
        check: (tables: ReadonlySet<string>) => Promise<boolean>,
        signal: AbortSignal,
        generation?: number,
    ): Promise<boolean> {
        // leave out the log's own checkpoints, which cursors and positions write
        const written = async (tables: ReadonlySet<string>) => {
            const rows = new Set([...tables].filter((table) => table !== LOG_CHECKPOINT));

            // skip a commit that wrote checkpoints alone
            if (tables.size > 0 && rows.size === 0) {
                return false;
            }

            return await check(rows);
        };

        return this.database.state.commits.waitUntil(written, signal, generation);
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
            // read the page from the process's shared tail, noting the watch's generation before it
            const { generation } = this.database.state.commits;
            const page = await this.#next({ ...selection, after }, generation);

            // yield a page with changes, or an advance when asked
            const isYielded =
                page.changes.length > 0 || (options.isAdvancing === true && page.sequence > after);
            after = Math.max(after, page.sequence);
            if (isYielded) {
                yield page;
            }

            // read on after a full page, else wait for a commit since the read
            if (page.isLast) {
                await this.waitUntil(
                    async (committed) =>
                        options.isAdvancing === true
                            ? committed.size > 0
                            : [...committed].some((table) => tables.has(table)),
                    signal,
                    generation,
                );
            }
        }
    }

    /** Read the next page a reader woken by commits sees, from the process's tail. */
    next<Definition extends Table>(
        selection: ChangeSelection<Definition>,
    ): Promise<ChangePage<Definition>> {
        return this.#next(selection, this.database.state.commits.generation);
    }

    /** Read a woken reader's next page from the process's shared tail, or on its own behind it. */
    async #next<Definition extends Table>(
        selection: ChangeSelection<Definition>,
        generation: number,
    ): Promise<ChangePage<Definition>> {
        // read inside a transaction on its own, as the tail reads committed changes only
        if (this.database.driver.transaction !== undefined) {
            return await this.read(selection);
        }

        // read past the tail once for every follower woken by the same commits
        const { tail } = this.database.state;
        await tail.advance(generation, selection.after, (from) =>
            this.#readTraced(
                "tail",
                () => this.#tailRead(from),
                (read) => read.entries.length,
            ),
        );

        // take the follower's tables from the tail, else read them on its own behind it
        const tables = new Map(selection.tables.map((table) => [table[TABLE].sqlName, table]));
        const scopes = selection.scopes === undefined ? undefined : new Set(selection.scopes);
        const limit = selection.limit ?? PAGE_LIMIT;
        const page = tail.page(selection.after, new Set(tables.keys()), scopes, limit);
        if (page === undefined) {
            return await this.read(selection);
        }

        // decode the follower's entries
        const dialect = this.database.dialect;
        const changes = page.entries.map((entry) => ({
            sequence: entry.sequence,
            ...decodeChange(entry, tableOf(tables, entry.table), dialect),
        }));

        return { changes, sequence: page.sequence, isLast: page.isLast };
    }

    /** Read a page of every table's entries past a sequence, completing its last transaction. */
    async #tailRead(after: number): Promise<TailRead<SequencedEntry>> {
        // read a page of every table's entries and the log's bounds in one statement
        const rows = await ENTRY_READS.every.all(this.database, {
            after,
            limit: PAGE_LIMIT,
            transaction: null,
        });
        const logged = LogColumn.nullableInteger(rows[0]?.["logged"]);
        const horizon = LogColumn.integer(rows[0]?.["horizon"]);

        // complete the last transaction of a full page
        let entries = presentEntries(rows, this.database.dialect);
        const isLast = entries.length < PAGE_LIMIT;
        const last = entries.at(-1);
        if (!isLast && last !== undefined && last.transaction !== null) {
            const rest = await ENTRY_READS.everyRest.all(this.database, {
                after: last.sequence,
                limit: null,
                transaction: last.transaction,
            });
            entries = [...entries, ...presentEntries(rest, this.database.dialect)];
        }

        // end a full page at its last entry, else at the head
        const end = entries.at(-1)?.sequence ?? after;
        const sequence = isLast ? Math.max(end, latestOf(logged, horizon)) : end;

        return { entries, sequence, horizon, isLast };
    }

    /** Report whether the log includes a sequence, reading its head only past the announced one. */
    async #includes(sequence: number): Promise<boolean> {
        return (
            this.database.state.commits.sequence >= sequence ||
            (await this.position()).sequence >= sequence
        );
    }

    /** Place a cursor's checkpoint at a sequence, ending its claim, retaining changes a while. */
    async advance(cursor: LogCursor, sequence: number, expiresAt: number): Promise<void> {
        const table = sql.relation(LOG_CHECKPOINT);
        await this.database.run(sql`
            INSERT INTO ${table} (name, sequence, expires_at, tables)
            VALUES (${cursor.name}, ${sequence}, ${expiresAt}, ${LogCursor.tables(cursor)})
            ON CONFLICT (name) DO UPDATE SET sequence = excluded.sequence, expires_at = excluded.expires_at, tables = excluded.tables, claim = NULL, claimed_at = NULL
        `);
    }

    /** Place a cursor's checkpoint at the log's oldest retained change unless placed. */
    async place(cursor: LogCursor, expiresAt: number): Promise<void> {
        await this.database.run(sql`
            INSERT INTO ${sql.relation(LOG_CHECKPOINT)} (name, sequence, expires_at, tables)
            VALUES (
                ${cursor.name},
                COALESCE((SELECT sequence FROM ${sql.relation(LOG_HORIZON)} WHERE slot = 1), 0),
                ${expiresAt},
                ${LogCursor.tables(cursor)}
            )
            ON CONFLICT (name) DO NOTHING
        `);
    }

    /** List the names of the checkpoints starting with a prefix, in name order. */
    async checkpoints(prefix: string): Promise<string[]> {
        const rows = await this.database.all(sql`
            SELECT name FROM ${sql.relation(LOG_CHECKPOINT)}
            WHERE substr(name, 1, ${prefix.length}) = ${prefix}
            ORDER BY name
        `);

        return rows.map((row) => LogColumn.text(row["name"]));
    }

    /** Move a cursor's checkpoint from the position it read to another. */
    async move(cursor: LogCursor, from: number, to: number, expiresAt: number): Promise<void> {
        // move a checkpoint at the read position, placing a new one for a cursor from the start
        const table = sql.relation(LOG_CHECKPOINT);
        const tables = LogCursor.tables(cursor);
        const moved = await this.database.all(
            from === 0
                ? sql`
                    INSERT INTO ${table} (name, sequence, expires_at, tables)
                    VALUES (${cursor.name}, ${to}, ${expiresAt}, ${tables})
                    ON CONFLICT (name) DO UPDATE SET sequence = excluded.sequence, expires_at = excluded.expires_at, tables = excluded.tables, claim = NULL, claimed_at = NULL
                    WHERE ${table}.sequence = ${from}
                    RETURNING name
                `
                : sql`
                    UPDATE ${table} SET sequence = ${to}, expires_at = ${expiresAt}, tables = ${tables}, claim = NULL, claimed_at = NULL
                    WHERE name = ${cursor.name} AND sequence = ${from}
                    RETURNING name
                `,
        );

        // refuse a checkpoint another cursor moved since the read
        if (moved.length === 0) {
            throw DatabaseError.of(
                "UPDATE_CONFLICT",
                `cannot read from checkpoint '${cursor.name}': it moved past '${from}'`,
            );
        }
    }

    /** Claim a cursor's page through a sequence unless claimed, absent once it moved on. */
    async claim(
        cursor: LogCursor,
        after: number,
        through: number,
        now: number,
        expiresAt: number,
    ): Promise<LogClaim | undefined> {
        // leave the checkpoint's claim, else claim through the sequence at the read
        const table = sql.relation(LOG_CHECKPOINT);
        const [row] = await this.database.all(
            after === 0
                ? sql`
                    INSERT INTO ${table} (name, sequence, expires_at, tables, claim, claimed_at)
                    VALUES (${cursor.name}, 0, ${expiresAt}, ${LogCursor.tables(cursor)}, ${through}, ${now})
                    ON CONFLICT (name) DO UPDATE SET
                        claim = COALESCE(${table}.claim, excluded.claim),
                        claimed_at = COALESCE(${table}.claimed_at, excluded.claimed_at)
                    WHERE ${table}.sequence = 0
                    RETURNING claim, claimed_at
                `
                : sql`
                    UPDATE ${table}
                    SET claim = COALESCE(claim, ${through}), claimed_at = COALESCE(claimed_at, ${now})
                    WHERE name = ${cursor.name} AND sequence = ${after}
                    RETURNING claim, claimed_at
                `,
        );

        return row === undefined ? undefined : LogClaim.read(row);
    }

    /** Read a cursor's claimed page, absent while it claims none. */
    async claimed(name: string): Promise<LogClaim | undefined> {
        const [row] = await this.database.all(
            sql`SELECT claim, claimed_at FROM ${sql.relation(LOG_CHECKPOINT)} WHERE name = ${name} AND claim IS NOT NULL`,
        );

        return row === undefined ? undefined : LogClaim.read(row);
    }

    /** Read a reader's checkpoint, absent without one. */
    async checkpoint(name: string): Promise<number | undefined> {
        const [row] = await this.database.all(
            sql`SELECT sequence FROM ${sql.relation(LOG_CHECKPOINT)} WHERE name = ${name}`,
        );

        return row === undefined ? undefined : LogColumn.integer(row["sequence"]);
    }

    /** Move several cursors' checkpoints forward to a sequence in one statement, never backward. */
    async forward(
        cursors: readonly LogCursor[],
        sequence: number,
        expiresAt: number,
    ): Promise<void> {
        // write no cursors without a statement
        if (cursors.length === 0) {
            return;
        }
        const table = sql.relation(LOG_CHECKPOINT);
        const rows = sql.join(
            cursors.map(
                (cursor) =>
                    sql`(${cursor.name}, ${sequence}, ${expiresAt}, ${LogCursor.tables(cursor)})`,
            ),
            sql`, `,
        );
        await this.database.run(sql`
            INSERT INTO ${table} (name, sequence, expires_at, tables) VALUES ${rows}
            ON CONFLICT (name) DO UPDATE SET sequence = excluded.sequence, expires_at = excluded.expires_at, tables = excluded.tables
            WHERE ${table}.sequence <= excluded.sequence
        `);
    }

    /** Read the sequences of several checkpoints by name in one statement. */
    async checkpointed(names: readonly string[]): Promise<Map<string, number>> {
        // answer no names without a statement
        if (names.length === 0) {
            return new Map();
        }
        const named = sql.join(
            names.map((name) => sql`${name}`),
            sql`, `,
        );
        const rows = await this.database.all(
            sql`SELECT name, sequence FROM ${sql.relation(LOG_CHECKPOINT)} WHERE name IN (${named})`,
        );

        return new Map(
            rows.map((row) => [LogColumn.text(row["name"]), LogColumn.integer(row["sequence"])]),
        );
    }

    /** Drop a checkpoint and its claim, retaining no more changes for it. */
    async drop(name: string): Promise<void> {
        await this.database.run(
            sql`DELETE FROM ${sql.relation(LOG_CHECKPOINT)} WHERE name = ${name}`,
        );
    }

    /** Delete old changes no checkpoint retains, answering when the oldest left was made. */
    async compact(before: number, now: number): Promise<number | undefined> {
        return await this.database.transaction(async (transaction) => {
            // find the newest removable change and the oldest windowed one
            const log = sql.relation(LOG);
            const [newestRow] = await transaction.all(sql`
                SELECT
                    max(CASE WHEN changed_at < ${before} THEN sequence END) AS sequence,
                    min(changed_at) AS "changedAt"
                FROM ${log}
                WHERE retention = 'window' AND sequence IS NOT NULL
            `);
            const newest = LogColumn.nullableInteger(newestRow?.["sequence"]);
            const oldest = LogColumn.nullableInteger(newestRow?.["changedAt"]) ?? undefined;
            if (newest === null) {
                return oldest;
            }

            // move each unclaimed checkpoint past the removable changes when none is of its tables
            const checkpoints = sql.relation(LOG_CHECKPOINT);
            await transaction.run(sql`
                UPDATE ${checkpoints} SET sequence = ${newest}
                WHERE claim IS NULL AND sequence < ${newest} AND NOT EXISTS (
                    SELECT 1 FROM ${log} AS entry
                    WHERE entry.sequence > ${checkpoints}.sequence
                        AND entry.sequence <= ${newest}
                        AND ${listed(sql`entry."table"`, sql`${checkpoints}.tables`)}
                )
            `);

            // retain the changes after the earliest live checkpoint
            const [retainedRow] = await transaction.all(sql`
                SELECT
                    (SELECT min(sequence) FROM ${sql.relation(LOG_CHECKPOINT)} WHERE expires_at > ${now}) AS sequence,
                    (SELECT sequence FROM ${sql.relation(LOG_HORIZON)} WHERE slot = 1) AS horizon
            `);
            const retained = {
                sequence: LogColumn.nullableInteger(retainedRow?.["sequence"]),
                horizon: LogColumn.integer(retainedRow?.["horizon"]),
            };
            const cap = retained.sequence === null ? newest : Math.min(newest, retained.sequence);
            if (cap <= retained.horizon) {
                return oldest;
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

            // read when the oldest windowed change left was made
            const [oldestRow] = await transaction.all(sql`
                SELECT min(changed_at) AS "changedAt"
                FROM ${log}
                WHERE retention = 'window' AND sequence IS NOT NULL
            `);

            return LogColumn.nullableInteger(oldestRow?.["changedAt"]) ?? undefined;
        });
    }

    /** Compact once due, answering the moment after which compacting is due again. */
    async compactIfDue(window: number, due: number, now: number): Promise<number> {
        // wait until past due, when the oldest change is older than the window
        if (now <= due) {
            return due;
        }

        // compact, then wait for the oldest change left to leave the window, else a whole window
        const oldest = await this.compact(now - window, now);

        return oldest !== undefined && oldest + window >= now ? oldest + window : now + window;
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
            throw DatabaseError.of("CHANGES_NOT_FOUND", `change ${sequence} not found`);
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
        const isScoped = selection.scopes !== undefined;
        const statement =
            options.transaction === undefined
                ? isScoped
                    ? ENTRY_READS.scopedPage
                    : ENTRY_READS.page
                : isScoped
                  ? ENTRY_READS.scopedRest
                  : ENTRY_READS.rest;

        return statement.all(this.database, {
            after,
            tables: JSON.stringify([...tables.keys()]),
            scopes: JSON.stringify(selection.scopes ?? []),
            limit: options.limit ?? null,
            transaction: options.transaction ?? null,
        });
    }
}

/** The log's head row: its log, epoch, latest sequence, horizon and written tables. */
interface LogHead {
    /** The log's identifier. */
    readonly log: string;
    /** The log's epoch. */
    readonly epoch: string;
    /** The latest logged sequence, null before the first commit. */
    readonly logged: number | null;
    /** The sequence compaction removed the changes up to. */
    readonly horizon: number;
    /** The SQL names of the tables commits after a sequence wrote, empty when unread. */
    readonly written: readonly string[];
}

/** The statements reading the log's head alone, or with the tables or scopes changed. */
const HEAD_READS = {
    head: new Statement((_value, { dialect }) => selectHead(dialect)),
    since: new Statement(
        (value, { dialect }) =>
            sql`${selectHead(dialect)}, (SELECT ${
                dialect === "sqlite"
                    ? sql`group_concat(DISTINCT "table")`
                    : sql`string_agg(DISTINCT "table", ',')`
            } FROM ${sql.relation(LOG)} WHERE sequence > ${value("after")}) AS written`,
    ),
    changed: changedRead(false),
    changedIn: changedRead(true),
};

/** The statement taking the log's commit lock in a PostgreSQL schema. */
const SERIALIZE = new Statement(
    (value) =>
        sql`SELECT pg_advisory_xact_lock(${sql.raw(String(COMMIT_LOCK))}, hashtext(${value("schema")}))`,
);

/** The statements reading log entries after a sequence: a page or a transaction's rest. */
const ENTRY_READS = {
    page: entryRead({ tables: "listed", isScoped: false, part: "page" }),
    scopedPage: entryRead({ tables: "listed", isScoped: true, part: "page" }),
    rest: entryRead({ tables: "listed", isScoped: false, part: "rest" }),
    scopedRest: entryRead({ tables: "listed", isScoped: true, part: "rest" }),
    every: entryRead({ tables: "every", isScoped: false, part: "page" }),
    everyRest: entryRead({ tables: "every", isScoped: false, part: "rest" }),
};

/** Which entries a read takes: its tables, whether in listed scopes only, and which part. */
interface EntryShape {
    /** The tables read: those the selection lists, or every table, as a shared tail reads. */
    readonly tables: "listed" | "every";
    /** Whether the read keeps the listed scopes only. */
    readonly isScoped: boolean;
    /** A page up to a limit, or the rest of one transaction. */
    readonly part: "page" | "rest";
}

/** Build the statement reading the head and some scopes' latest committed change. */
function changedRead(isListed: boolean): Statement {
    return new Statement((value, { dialect }) => {
        // seek each scope's newest committed entry on the log's scope index
        const inTables = isListed ? sql`AND ${listed(sql`"table"`, value("tables"))}` : sql``;
        const seek = sql`(SELECT sequence FROM ${sql.relation(LOG)} AS entry
            WHERE scope = wanted.value ${inTables}
                AND sequence IS NOT NULL AND ${committedEntry(dialect, sql`entry`)}
            ORDER BY sequence DESC LIMIT 1)`;

        return sql`${selectHead(dialect)}, (SELECT max(${seek}) FROM ${textElements(value("scopes"), "wanted")}) AS changed`;
    });
}

/** Build the statement reading log entries after a sequence. */
function entryRead(shape: EntryShape): Statement {
    // select the bounds and entries
    const log = sql.relation(LOG);

    return new Statement(
        (value) => sql`
            WITH bounds AS (
                SELECT
                    (SELECT max(sequence) FROM ${log}) AS logged,
                    (SELECT sequence FROM ${sql.relation(LOG_HORIZON)} WHERE slot = 1) AS horizon
            ), entries AS (
                SELECT sequence, "transaction", "table", key, operation, "row", previous, scope, changed_at
                FROM ${log}
                WHERE sequence > ${value("after")}
                    ${shape.part === "rest" ? sql`AND "transaction" = ${value("transaction")}` : sql``}
                    ${shape.tables === "listed" ? sql`AND ${listed(sql`"table"`, value("tables"))}` : sql``}
                    ${shape.isScoped ? sql`AND ${listed(sql`scope`, value("scopes"))}` : sql``}
                ORDER BY sequence
                ${shape.part === "rest" ? sql`` : sql`LIMIT ${value("limit")}`}
            )
            SELECT bounds.logged, bounds.horizon, entries.*
            FROM bounds
            LEFT JOIN entries ON 1 = 1
            ORDER BY entries.sequence
        `,
    );
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
                    `logged key of '${table[TABLE].name}' is invalid: it is missing '${property}'`,
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
): SequencedEntry[] {
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
            `log read is invalid: the log changed table '${name}', which the read does not select`,
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
