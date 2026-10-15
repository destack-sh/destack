import { aligned, canonicalize } from "@destack/schema";
import type { DatabaseConnection } from "../database/connection.ts";
import { type Dialect, PARAMETER_BUDGET } from "../dialect/dialect.ts";
import { elementValue, jsonElements, jsonOf, Statement } from "../query/statement.ts";
import { ColumnCipher, SEALED_DIALECT } from "../database/cipher.ts";
import { sql } from "../sql/index.ts";
import { classifyError, DatabaseError } from "../error/error.ts";
import { Condition, Scalar } from "../query/condition.ts";
import { Key } from "../query/key.ts";
import { Namespace } from "../query/namespace.ts";
import { Predicate } from "../query/predicate.ts";
import { Order } from "../query/order.ts";
import type { QueryOptions } from "../query/option.ts";
import { type RowImage, TABLE, type Select, Table, type UniqueKey } from "../table/table.ts";
import type { Row } from "../table/row.ts";
import type { Change } from "./log.ts";
import type { LogPosition } from "./position.ts";
import { type Overlay, Snapshot } from "./snapshot.ts";
import { RelationalQueryBuilder } from "./query.ts";

/** How often a transaction runs at most before a lost serialization race answers as unavailable. */
const TRANSACTION_ATTEMPTS = 5;

/** The backoff before the second attempt in milliseconds, doubled per later one and jittered. */
const BACKOFF_MILLISECONDS = 10;

/** One row a change set writes: its image before the first and after the last write, or null. */
export interface RowChange {
    /** The written table. */
    readonly table: Table;
    /** The row before the first write, null for a row the set creates. */
    readonly before: Row | null;
    /** The row after the last write, null for a row the set removes. */
    readonly after: Row | null;
}

/** What a transaction read, by table, which its commit checks the log against. */
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
        this.match(table, Condition.resolve(where, Namespace.fields(table)), "row");
    }

    /** Record a scan of the logged images of the rows meeting a condition. */
    images(table: Table, where: Condition): void {
        this.match(table, Condition.resolve(where, Namespace.fields(table)), "image");
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

    /** Record a scan of the rows a predicate matches, whole or as logged images. */
    match(table: Table, predicate: Predicate, kind: ReadKind): void {
        // add the compiled predicate to the table's scans
        const name = this.#table(table);
        const matches = this.#matches.get(name) ?? [];
        this.#matches.set(name, [...matches, { match: Predicate.compile(predicate, table), kind }]);
    }

    /** The tables the transaction read. */
    get tables(): readonly Table[] {
        return [...this.#tables.values()];
    }

    /** Report whether a committed change touches a read row, or a row a scan would select. */
    touches(change: Change): boolean {
        // let an update of only unlogged columns pass reads of logged images
        const name = change.table[TABLE].sqlName;
        const encoded = (image: Row | null) =>
            canonicalize(image === null ? null : change.table[TABLE].encode(image));
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

/** What a transaction writes by table and key: each row before its first and after its last. */
export class ChangeSet {
    /** The written rows, by table and key name. */
    readonly #rows = new Map<string, RowChange>();

    /** Record a write of one row: before, null for a creation, and after, null for a removal. */
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

    /** Read the written rows as an overlay of the database's, by table and key, at each read. */
    overlay(): Overlay {
        return async (table) => {
            // index the table's latest images by key name
            const name = table[TABLE].sqlName;
            const keyed = new Map<string, Row | null>();
            for (const change of this.#rows.values()) {
                const image = change.after ?? change.before;
                if (change.table[TABLE].sqlName === name && image !== null) {
                    const after = change.after === null ? null : table[TABLE].image(change.after);
                    keyed.set(Key.name(change.table, image), after);
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

    /** Read the set's row for a key: its latest image, null once removed, absent if unwritten. */
    written(table: Table, key: Row): Row | null | undefined {
        return this.#rows.get(Key.name(table, key))?.after;
    }

    /** Read the set's rows of a table by key name: each latest image, null once removed. */
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

    /** Apply the records once and then the changes in one transaction. */
    async commit(
        database: DatabaseConnection,
        base: LogPosition,
        reads: ReadSet,
        records: (transaction: DatabaseConnection) => Promise<void>,
    ): Promise<void> {
        // validate reads before the lock and check references after the writes
        await database.transaction(
            async (transaction) => {
                // hold off other commits, and refuse once a later commit touched a read, in one round trip
                const serialized = transaction.log.serialize();
                await ChangeSet.requireCurrent(transaction, base, reads);
                await serialized;

                // send the records, then each run of one table's changes, in one round trip
                await Promise.all([
                    records(transaction),
                    ...ChangeSet.#runs(this.changes, transaction.dialect).map((run) =>
                        ChangeSet.#apply(transaction, run),
                    ),
                ]);
            },
            { constraints: "deferred", isolationLevel: "read committed" },
        );
    }

    /** Group consecutive inserts, plain updates or deletes of one table, other updates one each. */
    static #runs(changes: Iterable<RowChange>, dialect: Dialect): RowChange[][] {
        const runs: RowChange[][] = [];
        for (const change of changes) {
            // extend a run of the same table's same batched operation, else start another
            const current = runs.at(-1);
            const operation = ChangeSet.#operation(change, dialect);
            const last = current === undefined ? undefined : aligned(current, 0);
            const isExtended =
                current !== undefined &&
                last !== undefined &&
                last.table === change.table &&
                operation !== "single" &&
                ChangeSet.#operation(last, dialect) === operation;
            if (isExtended) {
                current.push(change);
            } else {
                runs.push([change]);
            }
        }

        return runs;
    }

    /** Read how a change applies: inserted, updated or deleted in batches, else on its own. */
    static #operation(
        change: RowChange,
        dialect: Dialect,
    ): "insert" | "update" | "delete" | "single" {
        // batch insertions and removals
        const { table, before, after } = change;
        if (before === null) {
            return "insert";
        } else if (after === null) {
            return "delete";
        }

        // batch an update keeping its key and its unique values, its columns readable from JSON
        const definition = table[TABLE];
        const changed = changedColumns(change);
        const unique = new Set(definition.uniqueKeys.flatMap((key) => key.columns));
        const isBytes = [...definition.key, ...changed].some(
            (column) => definition.column(column).definition.kind === "binary",
        );
        const isPlain =
            Key.name(table, before) === Key.name(table, after) &&
            !changed.some((column) => unique.has(column)) &&
            !(isBytes && dialect === "sqlite");

        return isPlain ? "update" : "single";
    }

    /** Apply one run of changes: batched inserts and deletes, updates by changed columns. */
    static async #apply(transaction: DatabaseConnection, run: readonly RowChange[]): Promise<void> {
        // read the run's table and operation from its first change
        const first = aligned(run, 0);
        const { table } = first;
        const operation = ChangeSet.#operation(first, transaction.dialect);

        // insert created rows, a batch within the parameter budget per statement
        if (operation === "insert") {
            const rows = run.flatMap((change) => (change.after === null ? [] : [change.after]));
            const size = Math.max(1, Math.floor(PARAMETER_BUDGET / table[TABLE].entries.length));
            const batches = Array.from({ length: Math.ceil(rows.length / size) }, (_, index) =>
                rows.slice(index * size, (index + 1) * size),
            );
            await Promise.all(batches.map((batch) => transaction.insert(table).values(batch)));
        }
        // delete removed rows by key
        else if (operation === "delete") {
            await transaction.remove(
                table,
                run.flatMap((change) => (change.before === null ? [] : [change.before])),
            );
        }
        // update plain rows together
        else if (operation === "update") {
            await ChangeSet.#update(transaction, run);
        }
        // update a row moving its key or unique values on its own, in write order
        else {
            await ChangeSet.#set(transaction, first);
        }
    }

    /** Set one changed row's changed columns where its key was. */
    static async #set(transaction: DatabaseConnection, change: RowChange): Promise<void> {
        // write the changed columns where the row's key was
        const { table } = change;
        const { before, after } = ChangeSet.#kept(change);
        const changed = changedColumns(change);
        await transaction
            .update(table)
            .set(Object.fromEntries(changed.map((column) => [column, after[column]])))
            .where(Key.match(table, before));
    }

    /** Read a changed row's images before and after, refusing a creation or a removal. */
    static #kept(change: RowChange): { readonly before: Row; readonly after: Row } {
        const { table, before, after } = change;
        if (before === null || after === null) {
            throw new TypeError(
                `a change of a ${table[TABLE].name} row has no row before or after`,
            );
        }

        return { before, after };
    }

    /** Update plain rows, one statement per set of changed columns, a lone row by itself. */
    static async #update(
        transaction: DatabaseConnection,
        run: readonly RowChange[],
    ): Promise<void> {
        // group the rows by the columns they change, sending every group at once
        const groups = Map.groupBy(run, (change) => changedColumns(change).join(","));
        await Promise.all(
            [...groups.values()].map((group) => ChangeSet.#updateGroup(transaction, group)),
        );
    }

    /** Update one group of rows changing the same columns: a lone row by itself, else together. */
    static async #updateGroup(
        transaction: DatabaseConnection,
        group: readonly RowChange[],
    ): Promise<void> {
        // update a lone row by its own statement
        const first = aligned(group, 0);
        if (group.length === 1) {
            return ChangeSet.#set(transaction, first);
        }

        // update the group from one JSON array of each row's key and changed values
        const { table } = first;
        const key = table[TABLE].key;
        const changed = changedColumns(first);
        const tuples = group.map((change) => {
            const { before, after } = ChangeSet.#kept(change);

            return [
                ...key.map((column) => jsonOf(table, column, before)),
                ...changed.map((column) => ChangeSet.#element(transaction, table, column, after)),
            ];
        });
        await tupleUpdate(table, key, changed, transaction.dialect).all(transaction, {
            tuples: JSON.stringify(tuples),
        });
    }

    /** Write a changed column's JSON array element, sealing a sensitive value. */
    static #element(
        transaction: DatabaseConnection,
        table: Table,
        column: string,
        row: Row,
    ): unknown {
        // take a value without sealing as its JSON form
        const { definition } = table[TABLE].column(column);
        const value = row[column];
        if (definition.sealing === undefined || value === undefined || value === null) {
            return jsonOf(table, column, row);
        }

        // seal the driver value under the transaction's cipher
        const cipher = ColumnCipher.require(transaction.state.cipher, { definition });

        return cipher.seal(
            definition.encode(value, SEALED_DIALECT),
            definition.name,
            definition.sealing,
        );
    }

    /** Refuse with `CONCURRENT_UPDATE` once a later commit touched a transaction's reads. */
    static async requireCurrent(
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
                        "the log no longer has the changes the transaction read",
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

/** A transaction in progress over a database: its start position, reads and writes. */
export class Transaction {
    /** The database the transaction reads and commits to. */
    readonly database: DatabaseConnection;
    /** The transaction's start position, checked against the log at commit, null if read-only. */
    readonly #base: LogPosition | null;
    /** What the transaction read. */
    readonly reads = new ReadSet();
    /** What the transaction writes. */
    readonly changes = new ChangeSet();
    /** The records the transaction writes once at commit, such as its journal entry. */
    readonly #records: ((transaction: DatabaseConnection) => Promise<void>)[] = [];
    /** The identifiers the transaction derived so far, counted from zero in every attempt. */
    #derived = 0;
    /** The committed rows the transaction read by key, each read once and checked at commit. */
    readonly #committed = new Map<string, Row | null>();

    /** Start a transaction at a position, or one that only reads at null. */
    constructor(database: DatabaseConnection, base: LogPosition | null) {
        this.database = database;
        this.#base = base;
    }

    /** Start a transaction at the database's current position. */
    static async open(database: DatabaseConnection): Promise<Transaction> {
        return new Transaction(database, await database.log.position());
    }

    /** Run a transaction to commit, restarting while another commit wins a race on its reads. */
    static async run<Value>(
        database: DatabaseConnection,
        attempt: (transaction: Transaction) => Promise<Value>,
        signal?: AbortSignal,
    ): Promise<Value> {
        return Transaction.retry(async () => {
            // run the attempt from the log's current position and commit it
            const transaction = await Transaction.open(database);
            const value = await attempt(transaction);
            await transaction.commit();

            return value;
        }, signal);
    }

    /** Run an attempt opening and committing its own transaction again while it loses a race. */
    static async retry<Value>(run: () => Promise<Value>, signal?: AbortSignal): Promise<Value> {
        for (let attempt = 1; ; attempt++) {
            try {
                return await run();
            } catch (error) {
                // rethrow any other failure, and a lost race once the attempts run out
                const classified = classifyError(error);
                if (!isLostRace(classified) || attempt >= TRANSACTION_ATTEMPTS) {
                    throw classified;
                }

                // wait a jittered, doubling while before the next attempt
                const ceiling = BACKOFF_MILLISECONDS * 2 ** (attempt - 1);
                await new Promise((resolve) => {
                    setTimeout(resolve, Math.random() * ceiling);
                });
                signal?.throwIfAborted();
            }
        }
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
            throw new TypeError("a reading transaction has no start position");
        }

        return this.#base;
    }

    /** Show the database as the transaction sees it, its writes over live rows, recording reads. */
    view(): Snapshot {
        return Snapshot.live(this.database).recording(this.reads).layer(this.changes.overlay());
    }

    /** Read a table's whole rows with the transaction's own writes, recording each read. */
    read<Definition extends Table>(
        table: Definition,
    ): RelationalQueryBuilder<Definition, Select<Definition>> {
        return new RelationalQueryBuilder(
            table,
            async (query) => {
                const { where = {} } = query;
                this.reads.rows(table, where);

                return this.#rows(table, Condition.resolve(where, Namespace.fields(table)), query);
            },
            async (key) => {
                this.reads.row(table, key);

                return this.#row(table, key);
            },
        );
    }

    /** Read one row by its key with the transaction's own writes, unrecorded. */
    async #row<Definition extends Table>(
        table: Definition,
        key: Key,
    ): Promise<Select<Definition> | null> {
        // take the transaction's write of the row or the committed row
        const written = this.changes.written(table, key);
        if (written !== undefined) {
            return written === null ? null : Table.complete(table, written);
        }

        // take the committed row this transaction read before
        const name = `${table[TABLE].sqlName}\u0000${Key.name(table, key)}`;
        const known = this.#committed.get(name);
        if (known !== undefined) {
            return known === null ? null : Table.complete(table, known);
        }

        // read the committed row once
        const [row = null] = await this.database
            .select()
            .from(table)
            .where(Key.match(table, key))
            .limit(1);
        this.#committed.set(name, row);

        return row;
    }

    /** Read the rows meeting a condition with the transaction's own writes, unrecorded. */
    async #rows<Definition extends Table>(
        table: Definition,
        where: Predicate,
        options: QueryOptions<RowImage<NoInfer<Definition>>>,
    ): Promise<Select<Definition>[]> {
        // read enough committed rows in order to fill the limit after the writes replace some
        const order = Order.complete(Order.of(options.orderBy ?? {}), table);
        const written = this.changes.writtenRows(table);
        const query = this.database
            .select()
            .from(table)
            .where(Predicate.render(where, Predicate.bind(table, {})))
            .orderBy(...Order.render(order, table));
        const committed = await (options.limit === undefined
            ? query
            : query.limit(options.limit + written.size));

        // replace the rows the transaction wrote by its writes meeting the condition
        const predicate = Predicate.compile(where, table);
        const kept: Row[] = committed.filter((row) => !written.has(Key.name(table, row)));
        for (const row of written.values()) {
            if (row !== null && Predicate.matches(predicate, row)) {
                kept.push(row);
            }
        }

        // order and bound the rows
        const sorted = kept.toSorted((left, right) => Order.rows(order, left, right));
        const bounded = options.limit === undefined ? sorted : sorted.slice(0, options.limit);

        return bounded.map((row) => Table.complete(table, row));
    }

    /** Count the rows meeting a condition with the transaction's own writes. */
    async count<Definition extends Table>(
        table: Definition,
        where: Condition<RowImage<NoInfer<Definition>>> = {},
    ): Promise<number> {
        return (await this.view().read(table).findMany({ where })).length;
    }

    /** Write rows, refusing all of them when one duplicates a unique key. */
    async write(...changes: readonly RowChange[]): Promise<void> {
        // refuse a predefined row, identical in every database
        for (const { table, before, after } of changes) {
            const image = after ?? before;
            if (image !== null && Key.isPredefined(table, image)) {
                throw new DatabaseError(
                    "INVALID_RECORD",
                    `cannot write predefined row ${Key.name(table, image)} of ${table[TABLE].name}`,
                );
            }
        }

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

        // read the rows sharing the scalar values of any of the keys in one recorded read
        const shared = Condition.any(
            keyed.map(({ key, after }) => ({
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
            })),
            table,
        );
        this.reads.match(table, shared, "row");
        const read: Row[] = await this.#rows(table, shared, {});

        // see the rows as the changes leave them: the ones they replace gone, their new ones added
        const replaced = new Set(
            changes.flatMap(({ before }) => (before === null ? [] : [Key.name(table, before)])),
        );
        const rows = [
            ...read.filter((row) => !replaced.has(Key.name(table, row))),
            ...changes.flatMap(({ after }) => (after === null ? [] : [after])),
        ];

        // index the rows each key covers by their key value once per key
        const indexed = new Map<UniqueKey, Map<string, Row[]>>();
        const holders = (key: UniqueKey, covers: (row: Row) => boolean) => {
            // reuse a key's index, else build it
            const known = indexed.get(key);
            if (known !== undefined) {
                return known;
            }
            const held = Map.groupBy(rows.filter(covers), (row) => keyOf(table, key, row));
            indexed.set(key, held);

            return held;
        };

        // refuse the first key another row shares
        for (const { key, covers, after } of keyed) {
            const sharing = holders(key, covers).get(keyOf(table, key, after)) ?? [];
            const isShared = sharing.some((row) => row !== after);
            if (isShared) {
                // refuse a key a later commit took as a lost race
                if (this.#base !== null) {
                    await ChangeSet.requireCurrent(this.database, this.#base, this.reads);
                }
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

    /** Run a step whose writes and records drop on failure, as a savepoint does, and rethrow. */
    async savepoint<Value>(run: () => Promise<Value>): Promise<Value> {
        const restore = this.#mark();
        try {
            return await run();
        } catch (error) {
            restore();
            throw error;
        }
    }

    /** Run a rehearsal over the transaction's rows, returning its value and changed rows. */
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

    /** Write a record once at commit, before the changes, such as a journal entry. */
    record(write: (transaction: DatabaseConnection) => Promise<void>): void {
        this.#records.push(write);
    }

    /** Commit the records and changes if no read changed since the base. */
    commit(): Promise<void> {
        return this.changes.commit(this.database, this.base, this.reads, async (transaction) => {
            await Promise.all(this.#records.map((write) => write(transaction)));
        });
    }
}

/** What a read took: a whole row, or the logged image change readers see. */
type ReadKind = "row" | "image";

/** Decide whether a unique key covers a row: every row without a condition, else matches. */
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

/** Decide whether a row change differs from the row before: a creation, removal or change. */
function isChanged(change: RowChange): boolean {
    return (
        changedColumns(change).length > 0 || (change.before === null) !== (change.after === null)
    );
}

/** List the columns whose values differ between a row's images, comparing their canonical JSON. */
function changedColumns({ table, before, after }: RowChange): string[] {
    // encode both images and compare each column either has
    const left = before === null ? {} : table[TABLE].encode(before);
    const right = after === null ? {} : table[TABLE].encode(after);
    const columns = new Set([...Object.keys(left), ...Object.keys(right)]);

    return [...columns].filter(
        (column) => canonicalColumn(left[column]) !== canonicalColumn(right[column]),
    );
}

/** Write a column's JSON value canonically, absent for a column the image lacks. */
function canonicalColumn(value: unknown): string | undefined {
    return value === undefined ? undefined : canonicalize(value);
}

/** Decide whether two rows have equal unique key values, comparing their canonical JSON. */
function sameKey(table: Table, key: UniqueKey, left: Row, right: Row): boolean {
    return keyOf(table, key, left) === keyOf(table, key, right);
}

/** Write a row's value of a unique key as canonical JSON of the key's encoded columns. */
function keyOf(table: Table, key: UniqueKey, row: Row): string {
    return canonicalize(
        table[TABLE].encode(
            Object.fromEntries(key.columns.map((column) => [column, row[column] ?? null])),
        ),
    );
}

/** Report whether a failure lost a race: a serialization failure, deadlock or busy database. */
function isLostRace(error: unknown): boolean {
    return error instanceof DatabaseError && error.code === "CONCURRENT_UPDATE";
}

/** Update rows by key from a JSON array of each row's key and changed values, in that order. */
function tupleUpdate(
    table: Table,
    key: readonly string[],
    changed: readonly string[],
    dialect: Dialect,
): Statement {
    return table[TABLE].statement(`update:${dialect}:${changed.join(",")}`, () => {
        // set each changed column from its element after the key's
        const definition = table[TABLE];
        const set = changed.map(
            (column, index) =>
                sql`${sql.identifier(definition.column(column).definition.name)} = ${elementValue(definition.column(column), key.length + index, dialect)}`,
        );
        const matches = key.map(
            (column, index) =>
                sql`${definition.column(column)} = ${elementValue(definition.column(column), index, dialect)}`,
        );

        return new Statement(
            (value) =>
                sql`UPDATE ${table} SET ${sql.join(set, sql`, `)} FROM ${jsonElements(value("tuples"), "wanted")} WHERE ${sql.join(matches, sql` AND `)}`,
        );
    });
}
