import { aligned } from "@destack/schema";
import type { DatabaseConnection } from "../database/connection.ts";
import { retried } from "../database/retry.ts";
import { DatabaseError } from "../error/error.ts";
import { Condition, Scalar } from "../query/condition.ts";
import { Key } from "../query/key.ts";
import { Namespace } from "../query/namespace.ts";
import { Predicate } from "../query/predicate.ts";
import { Order, type OrderBy } from "../query/order.ts";
import { type RowImage, TABLE, type Select, type Table, type UniqueKey } from "../table/table.ts";
import type { Row } from "../table/row.ts";
import type { Change } from "./log.ts";
import type { LogPosition } from "./position.ts";
import { type Overlay, Snapshot } from "./snapshot.ts";

/** One row a change set writes: its image before the first write and after the last, null where absent. */
export interface RowChange {
    /** The written table. */
    readonly table: Table;
    /** The row before the first write, null for a row the set creates. */
    readonly before: Row | null;
    /** The row after the last write, null for a row the set removes. */
    readonly after: Row | null;
}

/** What a transaction read, by table: the rows it read by key and the conditions its scans met, whole rows or their logged images, which its commit checks the log against. */
export class ReadSet {
    /** The tables read, by SQL name. */
    readonly #tables = new Map<string, Table>();
    /** The names of the keys read, by table, and whether any read took the whole row. */
    readonly #keys = new Map<string, Map<string, ReadKind>>();
    /** The conditions scans met, by table, with what each read. */
    readonly #matches = new Map<
        string,
        { readonly match: ReturnType<typeof Predicate.compile>; readonly kind: ReadKind }[]
    >();

    /** Record a read of one whole row by its key. */
    row(table: Table, key: Row): void {
        this.#key(table, key, "row");
    }

    /** Record a read of one row's logged image by its key. */
    image(table: Table, key: Row): void {
        this.#key(table, key, "image");
    }

    /** Record a scan of the whole rows meeting a condition. */
    rows(table: Table, where: Condition): void {
        this.#scan(table, where, "row");
    }

    /** Record a scan of the logged images of the rows meeting a condition. */
    images(table: Table, where: Condition): void {
        this.#scan(table, where, "image");
    }

    /** Record a read of one row by its key, a whole row read outranking an image read. */
    #key(table: Table, key: Row, kind: ReadKind): void {
        // keep the strongest read of the key
        const name = this.#table(table);
        const keys = this.#keys.get(name) ?? new Map<string, ReadKind>();
        const keyName = Key.name(table, key);
        keys.set(keyName, keys.get(keyName) === "row" ? "row" : kind);
        this.#keys.set(name, keys);
    }

    /** Record a scan, compiling the condition against the table's fields. */
    #scan(table: Table, where: Condition, kind: ReadKind): void {
        // add the compiled condition to the table's scans
        const name = this.#table(table);
        const predicate = Condition.resolve(where, Namespace.fields(table));
        const matches = this.#matches.get(name) ?? [];
        this.#matches.set(name, [...matches, { match: Predicate.compile(predicate, table), kind }]);
    }

    /** The tables the transaction read. */
    get tables(): readonly Table[] {
        return [...this.#tables.values()];
    }

    /** Report whether a committed change touches a row the transaction read or a row one of its scans would select, before or after the change. */
    touches(change: Change): boolean {
        // let an update of unlogged columns alone pass reads of logged images
        const name = change.table[TABLE].sqlName;
        const encoded = (image: Row | null) =>
            JSON.stringify(image === null ? null : change.table[TABLE].encode(image));
        const isUnseen =
            change.operation === "update" && encoded(change.before) === encoded(change.after);
        const isSeen = (kind: ReadKind) => kind === "row" || !isUnseen;

        // compare the change's key and both of its images
        const read = this.#keys.get(name)?.get(Key.name(change.table, change.key));
        const isKeyRead = read !== undefined && isSeen(read);
        const images = [
            change.operation === "insert" ? null : change.before,
            change.operation === "delete" ? null : change.after,
        ];
        const isScanned = (this.#matches.get(name) ?? []).some(
            ({ match, kind }) =>
                isSeen(kind) &&
                images.some((image) => image !== null && Predicate.matches(match, image)),
        );

        return isKeyRead || isScanned;
    }

    /** Keep a table by its SQL name, answering the name. */
    #table(table: Table): string {
        const name = table[TABLE].sqlName;
        this.#tables.set(name, table);

        return name;
    }
}

/** What a transaction writes, by table and key: each row's image before its first write and after its last. */
export class ChangeSet {
    /** The written rows, by table and key name. */
    readonly #rows = new Map<string, RowChange>();

    /** Record a write of one row: the row before it, null for a creation, and after it, null for a removal. */
    write({ table, before, after }: RowChange): void {
        // key the row by its present image, refusing a table without a key
        if (table[TABLE].key.length === 0) {
            throw new TypeError(`${table[TABLE].name} has no key to write rows by`);
        }
        const image = after ?? before;
        if (image === null) {
            throw new TypeError(
                `a write to ${table[TABLE].name} has neither a row before nor after`,
            );
        }
        const name = Key.name(table, image);

        // keep the row's first image before and its latest after
        const first = this.#rows.get(name);
        this.#rows.set(name, { table, before: first === undefined ? before : first.before, after });
    }

    /** The rows that differ from their images before the set, in write order. */
    get changes(): readonly RowChange[] {
        return [...this.#rows.values()].filter(isChanged);
    }

    /** Read the written rows as an overlay of the database's, by table and key, as they stand at each read. */
    overlay(): Overlay {
        return async (table) => {
            // index the table's latest images by key name
            const name = table[TABLE].sqlName;
            const keyed = new Map<string, Row | null>();
            for (const change of this.#rows.values()) {
                const image = change.after ?? change.before;
                if (change.table[TABLE].sqlName === name && image !== null) {
                    keyed.set(Key.name(change.table, image), change.after);
                }
            }

            return keyed;
        };
    }

    /** Copy the set as it stands, to restore after a failed step. */
    copy(): ChangeSet {
        const copied = new ChangeSet();
        for (const [name, change] of this.#rows) {
            copied.#rows.set(name, change);
        }

        return copied;
    }

    /** List the rows written since an earlier copy that differ from their first image. */
    since(copied: ChangeSet): RowChange[] {
        return [...this.#rows]
            .filter(([name, change]) => copied.#rows.get(name) !== change)
            .map(([, change]) => change)
            .filter(isChanged);
    }

    /** Put back the rows of an earlier copy, dropping every write since. */
    restore(copied: ChangeSet): void {
        this.#rows.clear();
        for (const [name, change] of copied.#rows) {
            this.#rows.set(name, change);
        }
    }

    /** Read the row the set writes for a key: its latest image, null once removed, absent when unwritten. */
    written(table: Table, key: Row): Row | null | undefined {
        return this.#rows.get(Key.name(table, key))?.after;
    }

    /** Read the rows the set writes to a table, by key name: each latest image, null once removed. */
    writtenRows(table: Table): Map<string, Row | null> {
        // keep the latest image of each row of the table
        const name = table[TABLE].sqlName;
        const rows = new Map<string, Row | null>();
        for (const [key, change] of this.#rows) {
            if (change.table[TABLE].sqlName === name) {
                rows.set(key, change.after);
            }
        }

        return rows;
    }

    /** Apply the records written once and then the changes in one transaction, when no change committed after the base touched what the transaction read, else refuse with `CONCURRENT_UPDATE`. */
    async commit(
        database: DatabaseConnection,
        base: LogPosition,
        reads: ReadSet,
        records: (transaction: DatabaseConnection) => Promise<void>,
    ): Promise<void> {
        // check references once the whole state is written, and validate against every commit before the lock
        await database.transaction(
            async (transaction) => {
                // hold off other commits, and refuse once a commit after the base touched a read
                await transaction.log.serialize();
                await ChangeSet.#check(transaction, base, reads);

                // write the records kept once, before the changes they may enable
                await records(transaction);

                // insert, update or delete each changed row
                for (const { table, before, after } of this.changes) {
                    // insert a created row
                    if (before === null && after !== null) {
                        await transaction.insert(table).values(after);
                    }
                    // set a changed row's changed columns
                    else if (before !== null && after !== null) {
                        const changed = changedColumns({ table, before, after });
                        await transaction
                            .update(table)
                            .set(
                                Object.fromEntries(
                                    changed.map((column) => [column, after[column]]),
                                ),
                            )
                            .where(Key.match(table, before));
                    }
                    // delete a removed row
                    else if (before !== null) {
                        await transaction.delete(table).where(Key.match(table, before));
                    }
                }
            },
            { constraints: "deferred", isolationLevel: "read committed" },
        );
    }

    /** Refuse a commit once another commit after the base touched what the transaction read. */
    static async #check(
        transaction: DatabaseConnection,
        base: LogPosition,
        reads: ReadSet,
    ): Promise<void> {
        // require the base's epoch, whose sequences the log keeps counting
        const tables = reads.tables;
        if (tables.length === 0) {
            return;
        }
        const position = await transaction.log.position();
        if (position.epoch !== base.epoch) {
            throw new DatabaseError(
                "CONCURRENT_UPDATE",
                "the database's log restarted since the transaction read it",
            );
        }

        // read every change after the base, page by page, refusing once the log compacted some
        for (let after = base.sequence; after < position.sequence;) {
            const page = await transaction.log.read({ tables, after }).catch((error: unknown) => {
                if (error instanceof DatabaseError && error.code === "CHANGES_COMPACTED") {
                    throw new DatabaseError(
                        "CONCURRENT_UPDATE",
                        "the log no longer keeps what the transaction read against",
                    );
                }
                throw error;
            });
            if (page.changes.some((change) => reads.touches(change))) {
                throw new DatabaseError(
                    "CONCURRENT_UPDATE",
                    "another commit changed what the transaction read",
                );
            }
            if (page.sequence <= after) {
                return;
            }
            after = page.sequence;
        }
    }
}

/** How a transaction's read of several rows orders and bounds them. */
export interface RowsOptions<Read> {
    /** How the rows sort, completed by their key. */
    readonly orderBy?: OrderBy<Read>;
    /** The most rows read. */
    readonly limit?: number;
}

/** A transaction in progress over a database: the position it started at, what it read and what it writes, committed at once as a branch held in memory merges. */
export class Transaction {
    /** The database the transaction reads and commits to. */
    readonly database: DatabaseConnection;
    /** The log position the transaction started at, which its commit checks the log after, null for a transaction that only reads. */
    readonly #base: LogPosition | null;
    /** What the transaction read. */
    readonly reads = new ReadSet();
    /** What the transaction writes. */
    readonly changes = new ChangeSet();
    /** The records the transaction writes once at its commit, such as its journal entry, which nothing reads back before it commits. */
    readonly #records: ((transaction: DatabaseConnection) => Promise<void>)[] = [];
    /** The identifiers the transaction derived so far, counted from zero in every attempt. */
    #derived = 0;

    /** Start a transaction at a position, or one that only reads at null. */
    constructor(database: DatabaseConnection, base: LogPosition | null) {
        this.database = database;
        this.#base = base;
    }

    /** Start a transaction at the database's current position. */
    static async open(database: DatabaseConnection): Promise<Transaction> {
        return new Transaction(database, await database.log.position());
    }

    /** Run a transaction to its commit, running it again from a fresh start while another commit wins a race on what it read. */
    static async run<Value>(
        database: DatabaseConnection,
        attempt: (transaction: Transaction) => Promise<Value>,
        signal?: AbortSignal,
    ): Promise<Value> {
        return retried(async () => {
            // run the attempt from the log's current position and commit it
            const transaction = await Transaction.open(database);
            const value = await attempt(transaction);
            await transaction.commit();

            return value;
        }, signal);
    }

    /** Start a transaction that only reads, such as a query's, which never commits. */
    static reading(database: DatabaseConnection): Transaction {
        return new Transaction(database, null);
    }

    /** Whether the transaction only reads, such as a query's, which never commits. */
    get isReading(): boolean {
        return this.#base === null;
    }

    /** The log position the transaction started at, refusing a transaction that only reads. */
    get base(): LogPosition {
        if (this.#base === null) {
            throw new TypeError("a reading transaction starts at no position");
        }

        return this.#base;
    }

    /** Show the database as the transaction sees it, its own writes over the live rows, recording each read. */
    view(): Snapshot {
        return Snapshot.live(this.database).recording(this.reads).layer(this.changes.overlay());
    }

    /** Read one row by its key with the transaction's own writes, null when absent. */
    async row<Definition extends Table>(
        table: Definition,
        key: Key<Definition>,
    ): Promise<Select<Definition> | null> {
        this.reads.row(table, key);

        return this.#row(table, key);
    }

    /** Read one row by its key with the transaction's own writes, unrecorded. */
    async #row<Definition extends Table>(
        table: Definition,
        key: Key<Definition>,
    ): Promise<Select<Definition> | null> {
        // take the transaction's write of the row or the committed row
        const written = this.changes.written(table, key);
        if (written !== undefined) {
            return written === null ? null : aligned(selected<Definition>([written]), 0);
        }
        const [row = null] = await this.database
            .select()
            .from(table)
            .where(Key.match(table, key))
            .limit(1);

        return row;
    }

    /** Read the rows meeting a condition with the transaction's own writes, ordered and bounded as asked. */
    async rows<Definition extends Table>(
        table: Definition,
        where: Condition<RowImage<NoInfer<Definition>>> = {},
        options: RowsOptions<RowImage<NoInfer<Definition>>> = {},
    ): Promise<Select<Definition>[]> {
        this.reads.rows(table, where);

        return this.#rows(table, where, options);
    }

    /** Read the rows meeting a condition with the transaction's own writes, unrecorded. */
    async #rows<Definition extends Table>(
        table: Definition,
        where: Condition<RowImage<NoInfer<Definition>>>,
        options: RowsOptions<RowImage<NoInfer<Definition>>>,
    ): Promise<Select<Definition>[]> {
        // read the committed rows meeting the condition in order, enough for the limit once the writes replace some
        const order = Order.complete(Order.of(options.orderBy ?? {}), table);
        const written = this.changes.writtenRows(table);
        const query = this.database
            .select()
            .from(table)
            .where(Condition.render(where, table))
            .orderBy(...Order.render(order, table));
        const committed = await (options.limit === undefined
            ? query
            : query.limit(options.limit + written.size));

        // replace the rows the transaction wrote by its writes meeting the condition
        const predicate = Predicate.compile(
            Condition.resolve(where, Namespace.fields(table)),
            table,
        );
        const kept: Row[] = committed.filter((row) => !written.has(Key.name(table, row)));
        for (const row of written.values()) {
            if (row !== null && Predicate.matches(predicate, row)) {
                kept.push(row);
            }
        }

        // order and bound the rows
        const sorted = kept.toSorted((left, right) => Order.rows(order, left, right));

        return selected<Definition>(
            options.limit === undefined ? sorted : sorted.slice(0, options.limit),
        );
    }

    /** Read one row's logged columns by its key with the transaction's own writes, null when absent. */
    async image<Definition extends Table>(
        table: Definition,
        key: Key<Definition>,
    ): Promise<RowImage<Definition> | null> {
        this.reads.image(table, key);
        const row = await this.#row(table, key);

        return row === null ? null : aligned(images<Definition>([row]), 0);
    }

    /** Read the logged columns of the rows meeting a condition with the transaction's own writes, ordered and bounded as asked. */
    async images<Definition extends Table>(
        table: Definition,
        where: Condition<RowImage<NoInfer<Definition>>> = {},
        options: RowsOptions<RowImage<NoInfer<Definition>>> = {},
    ): Promise<RowImage<Definition>[]> {
        this.reads.images(table, where);

        return images<Definition>(await this.#rows(table, where, options));
    }

    /** Count the rows meeting a condition with the transaction's own writes. */
    async count<Definition extends Table>(
        table: Definition,
        where: Condition<RowImage<NoInfer<Definition>>> = {},
    ): Promise<number> {
        return (await this.images(table, where)).length;
    }

    /** Write rows, refusing all of them when one duplicates a unique key. */
    async write(...changes: readonly RowChange[]): Promise<void> {
        // refuse a duplicate of a unique key, reading each table once
        const tables = new Map<Table, RowChange[]>();
        for (const change of changes) {
            const written = tables.get(change.table);
            if (written === undefined) {
                tables.set(change.table, [change]);
            } else {
                written.push(change);
            }
        }
        for (const [table, written] of tables) {
            await this.#requireUnique(table, written);
        }

        // record each change
        for (const change of changes) {
            this.changes.write(change);
        }
    }

    /** Refuse rows sharing a unique key with another, in one recorded read. */
    async #requireUnique(table: Table, changes: readonly RowChange[]): Promise<void> {
        // take the keys each change sets, leaving out keys with nulls and ones it keeps
        const definition = table[TABLE];
        const keyed = changes.flatMap(({ before, after }) =>
            after === null
                ? []
                : definition.uniqueKeys.flatMap((key) => {
                      // decide whether the change sets the key and whether it keeps it
                      const covers = covering(table, key);
                      const isSet =
                          (key.isNullsNotDistinct ||
                              key.columns.every((column) => after[column] !== null)) &&
                          covers(after);
                      const isKept =
                          before !== null && covers(before) && sameKey(table, key, before, after);

                      return isSet && !isKept ? [{ key, covers, after }] : [];
                  }),
        );

        // skip changes setting no key
        if (keyed.length === 0) {
            return;
        }

        // read the rows sharing the scalar columns of any of the keys
        const shared = keyed.map(({ key, after }) => ({
            AND: [
                Condition.equal(
                    Object.fromEntries(
                        key.columns.flatMap((column) => {
                            const value = Scalar.safeParse(after[column]);

                            return value.success ? [[column, value.data]] : [];
                        }),
                    ),
                ),
                key.where ?? {},
            ],
        }));
        const read = await this.rows(table, { OR: shared });

        // see the rows as the changes leave them: the ones they replace gone, their new ones added
        const replaced = new Set(
            changes.flatMap(({ before }) => (before === null ? [] : [Key.name(table, before)])),
        );
        const rows = [
            ...read.filter((row) => !replaced.has(Key.name(table, row))),
            ...changes.flatMap(({ after }) => (after === null ? [] : [after])),
        ];

        // refuse the first key another row shares
        for (const { key, covers, after } of keyed) {
            const isShared = rows.some(
                (row) => row !== after && covers(row) && sameKey(table, key, row, after),
            );
            if (isShared) {
                throw new DatabaseError("DUPLICATE", "a record with the same unique key exists", {
                    key: {
                        table: definition.sqlName,
                        columns: key.columns.map(
                            (column) => definition.column(column).definition.name,
                        ),
                    },
                });
            }
        }
    }

    /** Run a step whose writes and records the transaction drops when it fails, as a savepoint does, and rethrow its failure. */
    async savepoint<Value>(run: () => Promise<Value>): Promise<Value> {
        const restore = this.#mark();
        try {
            return await run();
        } catch (error) {
            restore();
            throw error;
        }
    }

    /** Run a rehearsal over the transaction's rows, answering its value and the rows it changed, then drop its writes and records. */
    async rehearse<Value>(
        run: () => Promise<Value>,
    ): Promise<{ readonly value: Value; readonly changes: readonly RowChange[] }> {
        const marked = this.changes.copy();
        const restore = this.#mark();
        try {
            const value = await run();

            return { value, changes: this.changes.since(marked) };
        } finally {
            restore();
        }
    }

    /** Mark the writes and records so far, answering how to drop every one made since. */
    #mark(): () => void {
        const changes = this.changes.copy();
        const records = this.#records.length;

        return () => {
            this.changes.restore(changes);
            this.#records.splice(records);
        };
    }

    /** Take the position of the transaction's next derived identifier, alike in every attempt. */
    takePosition(): number {
        const position = this.#derived;
        this.#derived += 1;

        return position;
    }

    /** Write a record once at the commit, in its transaction before the changes, such as a journal entry nothing reads back before then. */
    record(write: (transaction: DatabaseConnection) => Promise<void>): void {
        this.#records.push(write);
    }

    /** Commit the records and the changes when nothing the transaction read changed since its base, else refuse with `CONCURRENT_UPDATE`. */
    commit(): Promise<void> {
        return this.changes.commit(this.database, this.base, this.reads, async (transaction) => {
            for (const write of this.#records) {
                await write(transaction);
            }
        });
    }
}

/** Type rows a transaction read through a table's columns, or wrote as that table's rows, as that table's records. */
function selected<Definition extends Table>(rows: readonly Row[]): (Select<Definition> & Row)[];
/**
 * Pass the rows on, which the database selected through the table's columns or the transaction wrote as complete rows of the table.
 *
 * @construct every row came from a select of the table or a complete write of it, which is how Select maps the table.
 */
function selected(rows: readonly Row[]): Row[] {
    return [...rows];
}

/** Type whole rows of a table as its logged records, which keep a subset of their columns. */
function images<Definition extends Table>(rows: readonly Row[]): (RowImage<Definition> & Row)[];
/**
 * Pass the rows on, whose columns include every logged column of their table.
 *
 * @construct every row is a whole row of the table, and RowImage maps the table's logged columns, a subset of them.
 */
function images(rows: readonly Row[]): Row[] {
    return [...rows];
}

/** What a read took: a whole row, or the logged image change readers see. */
type ReadKind = "row" | "image";

/** Decide whether a unique key covers a row: every row without a condition, else the rows meeting it. */
function covering(table: Table, key: UniqueKey): (row: Row) => boolean {
    if (key.where === undefined) {
        return () => true;
    }
    const predicate = Predicate.compile(
        Condition.resolve(key.where, Namespace.fields(table)),
        table,
    );

    return (row) => Predicate.matches(predicate, row);
}

/** Decide whether a row change differs from the row before it: a creation, a removal, or a changed column. */
function isChanged(change: RowChange): boolean {
    return (
        changedColumns(change).length > 0 || (change.before === null) !== (change.after === null)
    );
}

/** List the columns whose values differ between a row's images, comparing their JSON forms. */
function changedColumns({ table, before, after }: RowChange): string[] {
    // encode both images and compare each column either has
    const left = before === null ? {} : table[TABLE].encode(before);
    const right = after === null ? {} : table[TABLE].encode(after);
    const columns = new Set([...Object.keys(left), ...Object.keys(right)]);

    return [...columns].filter(
        (column) => JSON.stringify(left[column]) !== JSON.stringify(right[column]),
    );
}

/** Decide whether two rows hold the same values of a unique key's columns, comparing their JSON forms. */
function sameKey(table: Table, key: UniqueKey, left: Row, right: Row): boolean {
    const encoded = (row: Row) =>
        JSON.stringify(
            table[TABLE].encode(
                Object.fromEntries(key.columns.map((column) => [column, row[column] ?? null])),
            ),
        );

    return encoded(left) === encoded(right);
}
