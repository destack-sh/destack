import { and, inArray, sql, type SQL } from "../sql/index.ts";
import { Projection } from "../query/selection.ts";
import type { DatabaseConnection } from "../database/connection.ts";
import { Condition, Scalar } from "../query/condition.ts";
import { Predicate, type Match } from "../query/predicate.ts";
import { Key } from "../query/key.ts";
import { Change } from "./log.ts";
import { Order, type OrderBy } from "../query/order.ts";
import { Namespace, type Extras } from "../query/namespace.ts";
import { Expression, type RelationReader } from "../expression/expression.ts";
import { TABLE, Table, type RowImage } from "../table/table.ts";
import { headFields, latestOf, LogColumn, selectHead, type LogPosition } from "./position.ts";
import type { Row } from "../table/row.ts";
import type { ReadSet } from "./commit.ts";
import type { Column, ColumnValue } from "../table/column.ts";
import { elementValue, jsonElements, jsonOf, Statement } from "../query/statement.ts";
import { DatabaseError } from "../error/error.ts";
import type { Dialect } from "../dialect/dialect.ts";

/** The name of a ranked read's rank column. */
const RANK = "destack_rank";

/** The head columns a tuple read selects before each row's own. */
const HEAD_COLUMNS = ["epoch", "logged", "horizon"] as const;

/** A read of the first matching rows in an order. */
export interface WindowRead<Read = Row> {
    /** The condition the rows meet. */
    readonly where: Condition<Read>;
    /** How the rows sort, completed by the primary key. */
    readonly orderBy: OrderBy<Read>;
    /** The most rows of each window. */
    readonly limit: number;
    /** What the read admits beyond the condition. */
    readonly admits?: Admission;
    /** The computed values and relations the condition and order read. */
    readonly namespace?: Namespace;
    /** How the read follows the table's relations in memory. */
    readonly relations?: RelationView;
}

/** The log's head as one statement saw it: epoch, newest sequence and highest compacted one. */
interface Head {
    /** The log's epoch. */
    readonly epoch: string;
    /** The newest logged sequence, null before any. */
    readonly logged: number | null;
    /** The highest compacted sequence. */
    readonly horizon: number;
}

/** The partitions of a windowed read: the rows whose column has each of some values. */
export interface Partition {
    /** The column, by property. */
    readonly column: string;
    /** The values, each for one partition. */
    readonly values: readonly ColumnValue[];
}

/** Read the images a table's rows had before their first change between two sequences, by key. */
export type Rewind = (
    table: Table,
    after: number,
    upto: number,
) => Promise<ReadonlyMap<string, Row | null>>;

/** Read a layer's rows over a table's rows by key name: a row, or null for a removed one. */
export type Overlay = (table: Table) => Promise<ReadonlyMap<string, Row | null>>;

/** What an ordered read admits beyond a condition. */
export interface Admission {
    /** Match the admitted current rows, absent to decide in memory. */
    readonly current?: SQL;
    /** Decide whether a row image is admitted. */
    readonly image: (row: Row) => Promise<boolean>;
}

/** How a snapshot follows a table's relations in memory. */
export interface RelationView {
    /** Decide whether a related row met a condition, as of the position. */
    decide(via: string, where: Condition, row: Row): Promise<boolean>;
    /** Read the keys of rows with related rows that changed between two sequences. */
    touched(after: number, upto: number): Promise<readonly Row[]>;
    /** Resolve a row's lookups, as of the position. */
    resolve(row: Row): Promise<RelationReader>;
}

/** The database's logged columns as they were at a log position. */
export class Snapshot {
    /** The database read. */
    readonly database: DatabaseConnection;
    /** The position, absent for the live database. */
    readonly position: LogPosition | undefined;
    /** The source of earlier row images. */
    readonly #rewind: Rewind;
    /** The rows a layer puts over the database's, such as a branch's, absent for none. */
    readonly #overlay: Overlay | undefined;
    /** The record of the snapshot's reads, checked at a mutation's commit, if any. */
    readonly #reads: ReadSet | undefined;
    /** The rows changed related rows touched, read once per relations, table and sequence. */
    readonly #touched = new WeakMap<RelationView, Map<string, Promise<Map<string, Row | null>>>>();

    /** Show a database as of a position, under an overlay when one is given. */
    constructor(
        database: DatabaseConnection,
        position: LogPosition | undefined,
        rewind?: Rewind,
        overlay?: Overlay,
        reads?: ReadSet,
    ) {
        // keep the database, the position, how earlier and overlaid rows read, and what records the reads
        this.database = database;
        this.position = position;
        this.#rewind = rewind ?? database.log.rewind();
        this.#overlay = overlay;
        this.#reads = reads;
    }

    /** Whether reads go to the live database: no position, no overlay and no recording. */
    get isLive(): boolean {
        return (
            this.position === undefined && this.#overlay === undefined && this.#reads === undefined
        );
    }

    /** Show the same database and position under an overlay. */
    layer(overlay: Overlay): Snapshot {
        return new Snapshot(this.database, this.position, this.#rewind, overlay, this.#reads);
    }

    /** Show the same view, recording each read. */
    recording(reads: ReadSet): Snapshot {
        return new Snapshot(this.database, this.position, this.#rewind, this.#overlay, reads);
    }

    /** Show the live database with its transaction's writes. */
    static live(database: DatabaseConnection): Snapshot {
        return new Snapshot(database, undefined);
    }

    /** Read a table's rows matching a condition. */
    async rows<Definition extends Table>(
        table: Definition,
        where: Condition<RowImage<NoInfer<Definition>>>,
    ): Promise<RowImage<Definition>[]> {
        // record the scan
        this.#reads?.images(table, where);

        // read the current rows and the changed rows' images
        const predicate = Condition.resolve(where, Namespace.fields(table));
        const { rows, sequence } = await this.#read(table, render(predicate, table));
        const images = await this.#since(table, sequence);

        // keep unchanged rows and matching images
        const match = Predicate.compile(predicate, table);
        const kept =
            images.size === 0 ? rows : rows.filter((row) => !images.has(Key.name(table, row)));
        for (const image of images.values()) {
            if (image !== null && Predicate.matches(match, image)) {
                kept.push(image);
            }
        }

        return logged(kept);
    }

    /** Read a table's rows at the position whose columns have one of some tuples in JSON form. */
    async select<Definition extends Table>(
        table: Definition,
        columns: readonly string[],
        tuples: readonly (readonly unknown[])[],
    ): Promise<RowImage<Definition>[]> {
        // read the head and each tuple's rows in one statement, recording each tuple's rows
        if (tuples.length === 0) {
            return [];
        }
        this.#reads?.match(
            table,
            Condition.any(
                tuples.map((tuple) =>
                    Condition.equal(
                        Object.fromEntries(
                            columns.map((column, index) => [column, Scalar.parse(tuple[index])]),
                        ),
                    ),
                ),
                table,
            ),
            "image",
        );
        const read = await tupleRead(table, columns, this.database.dialect).values(this.database, {
            tuples: JSON.stringify(tuples),
        });
        const [epoch, latest, horizon] = read[0] ?? [];
        const sequence = this.#require(
            LogColumn.text(epoch),
            latestOf(LogColumn.nullableInteger(latest), LogColumn.integer(horizon)),
        );

        // decode the rows
        const dialect = this.database.dialect;
        const selected = Object.entries(table[TABLE].logged);
        const key =
            HEAD_COLUMNS.length +
            selected.findIndex(([property]) => property === table[TABLE].key[0]);
        const rows = read
            .filter((values) => values[key] !== null)
            .map((values) => decodeValues(selected, values.slice(HEAD_COLUMNS.length), dialect));
        const images = await this.#since(table, sequence);

        // keep unchanged rows and matching images
        const wanted = new Set(tuples.map((tuple) => JSON.stringify(tuple)));
        const match = (row: Row) =>
            wanted.has(JSON.stringify(columns.map((column) => jsonOf(table, column, row))));
        const kept =
            images.size === 0 ? rows : rows.filter((row) => !images.has(Key.name(table, row)));
        for (const image of images.values()) {
            if (image !== null && match(image)) {
                kept.push(image);
            }
        }

        return logged(kept);
    }

    /** Read one row by its key, null when it did not exist at the position. */
    async row<Definition extends Table>(
        table: Definition,
        key: Key<Definition>,
    ): Promise<RowImage<Definition> | null> {
        // record the read
        this.#reads?.image(table, key);

        // read the row and its earlier image
        const { rows, sequence } = await this.#read(table, Key.match(table, key));
        const images = await this.#since(table, sequence);
        const name = Key.name(table, key);
        const row = images.has(name) ? images.get(name) : rows[0];
        const [present = null] = row === undefined || row === null ? [] : logged<Definition>([row]);

        return present;
    }

    /** Read up to a count of admitted matching rows ordered after a row, with computed values. */
    async ordered<Definition extends Table, Extra = {}>(
        table: Definition,
        query: WindowRead<RowImage<NoInfer<Definition>> & Extra> & { readonly after?: Row },
    ): Promise<(RowImage<Definition> & Row)[]> {
        const [rows] = await this.#windows(table, query, undefined);
        if (rows === undefined) {
            throw new RangeError("an ordered read lacks its one window");
        }

        return logged(rows);
    }

    /** Read up to a count of admitted rows per partition of a column's values, in order. */
    async windows<Definition extends Table, Extra = {}>(
        table: Definition,
        query: WindowRead<RowImage<NoInfer<Definition>> & Extra> & {
            readonly partition: Partition;
        },
    ): Promise<(RowImage<Definition> & Row)[][]> {
        const windows = await this.#windows(table, query, query.partition);

        return windows.map((window) => logged(window));
    }

    /** Read the first rows of one window, or of each partition's window in ranked rounds. */
    async #windows<Read>(
        table: Table,
        query: WindowRead<Read> & { readonly after?: Row },
        partition: Partition | undefined,
    ): Promise<Row[][]> {
        // record the scan, then read enough current rows to cover every changed row
        this.#reads?.images(table, query.where ?? {});
        const admits = query.admits;
        const isAdmittedInMemory = admits !== undefined && admits.current === undefined;
        const order = Order.complete(Order.of(query.orderBy), table);
        const namespace = query.namespace ?? { extras: {} };
        const predicate = Condition.resolve(query.where, Namespace.fields(table, namespace));
        const relations = query.relations;
        let changed = -1;
        let reached = this.position?.sequence;
        const overlay = (await this.#overlay?.(table)) ?? new Map<string, Row | null>();
        const images = new Map(overlay);
        let unsettled = new Map(overlay);
        let rows: Row[] = [];
        while (unsettled.size > changed) {
            changed = unsettled.size;
            const limit = isAdmittedInMemory ? undefined : query.limit + changed;
            const read = await this.#readCurrent(table, query, partition, order, predicate, limit);
            rows = read.rows;

            // take the rows as they are when live
            const position = this.position;
            if (position === undefined || reached === undefined) {
                break;
            }

            // extend the images past the earlier read
            const sequence = read.sequence ?? (await this.#latest());
            unsettled = await this.#extendImages(
                table,
                images,
                relations,
                reached,
                sequence,
                position.sequence,
            );
            reached = Math.max(reached, sequence);
        }

        // overlay the matching admitted images
        const kept = await this.#merge(table, rows, unsettled, query, order, predicate);

        // sort each window and keep its first rows
        const windows = windowsOf(table, kept, partition);

        return windows.map((window) =>
            window.toSorted((left, right) => Order.rows(order, left, right)).slice(0, query.limit),
        );
    }

    /** Read the current rows matching a windowed read up to a limit per window. */
    #readCurrent<Read>(
        table: Table,
        query: WindowRead<Read> & { readonly after?: Row },
        partition: Partition | undefined,
        order: Order,
        predicate: Predicate,
        limit: number | undefined,
    ): Promise<{ readonly rows: Row[]; readonly sequence: number | undefined }> {
        // select the matching admitted rows after the read's row
        const namespace = query.namespace ?? { extras: {} };
        const selection = and(
            render(predicate, table, namespace),
            query.admits?.current,
            query.after === undefined
                ? undefined
                : Order.after(order, table, query.after, namespace),
        );

        return partition === undefined
            ? this.#read(table, selection, order, limit, namespace)
            : this.#ranked(table, selection, partition, order, limit, namespace);
    }

    /** Add the images of the rows changed past the earlier read. */
    async #extendImages(
        table: Table,
        images: Map<string, Row | null>,
        relations: RelationView | undefined,
        reached: number,
        sequence: number,
        position: number,
    ): Promise<Map<string, Row | null>> {
        // extend the images past the earlier read
        for (const [name, image] of await this.#rewind(table, reached, sequence)) {
            if (!images.has(name)) {
                images.set(name, image);
            }
        }

        // add the rows whose related rows changed
        const settling = new Map(images);
        const touched =
            relations === undefined
                ? new Map<string, Row | null>()
                : await this.#settled(table, relations, position, sequence);
        for (const [name, row] of touched) {
            if (!settling.has(name)) {
                settling.set(name, row);
            }
        }

        return settling;
    }

    /** Keep admitted current rows without an image, and add admitted images after a read's row. */
    async #merge<Read>(
        table: Table,
        rows: Row[],
        images: ReadonlyMap<string, Row | null>,
        query: WindowRead<Read> & { readonly after?: Row },
        order: Order,
        predicate: Predicate,
    ): Promise<Row[]> {
        // keep the current rows no image replaces
        const { admits, relations } = query;
        const namespace = query.namespace ?? { extras: {} };
        const match = Predicate.compile(predicate, table);
        const current =
            images.size === 0 ? rows : rows.filter((row) => !images.has(Key.name(table, row)));

        // admit them in memory when the read admits no rows in SQL
        const decided =
            admits !== undefined && admits.current === undefined
                ? await Promise.all(current.map((row) => admits.image(row)))
                : undefined;
        const kept =
            decided === undefined ? current : current.filter((_, index) => decided[index] === true);

        // add the admitted images matching the read after its row
        for (const image of images.values()) {
            const augmented =
                image === null
                    ? null
                    : augment(image, namespace.extras, await relations?.resolve(image));
            if (
                augmented !== null &&
                (await decides(match, predicate, table, augmented, relations)) &&
                (query.after === undefined || Order.rows(order, augmented, query.after) > 0) &&
                (admits === undefined || (await admits.image(augmented)))
            ) {
                kept.push(augmented);
            }
        }

        return kept;
    }

    /** Read the rows related changes touched between the position and a sequence. */
    #settled(
        table: Table,
        relations: RelationView,
        from: number,
        sequence: number,
    ): Promise<Map<string, Row | null>> {
        // share the read of the same table and sequence
        const reads =
            this.#touched.get(relations) ?? new Map<string, Promise<Map<string, Row | null>>>();
        this.#touched.set(relations, reads);
        const name = `${table[TABLE].sqlName}:${sequence}`;
        const known = reads.get(name);
        if (known !== undefined) {
            return known;
        }

        // read the touched keys' rows as of the position
        const read = relations.touched(from, sequence).then((keys) => this.#rowsOf(table, keys));
        reads.set(name, read);

        return read;
    }

    /** Read rows by key as of the position, null for missing keys. */
    async #rowsOf(table: Table, keys: readonly Row[]): Promise<Map<string, Row | null>> {
        // start every key as missing
        const found = new Map<string, Row | null>(keys.map((key) => [Key.name(table, key), null]));

        // read the keys' rows in one statement
        const tuples = keys.map((key) => Key.tuple(table, key));
        for (const row of await this.select(table, table[TABLE].key, tuples)) {
            found.set(Key.name(table, row), row);
        }

        return found;
    }

    /** Read the logged columns of admitted rows with the log's head in one statement. */
    async #read(
        table: Table,
        selection: SQL,
        order?: Order,
        limit?: number,
        namespace: Namespace = { extras: {} },
    ): Promise<{ readonly rows: Row[]; readonly sequence: number | undefined }> {
        // read the rows with their computed values beside the head
        const query = this.database
            .select(this.#fields(table, namespace))
            .from(table)
            .where(selection);
        const ordered =
            order === undefined ? query : query.orderBy(...Order.render(order, table, namespace));

        return this.#take(await (limit === undefined ? ordered : ordered.limit(limit)));
    }

    /** Read the first rows of each partition's window in one ranked statement, with the head. */
    async #ranked(
        table: Table,
        selection: SQL | undefined,
        partition: Partition,
        order: Order,
        limit: number | undefined,
        namespace: Namespace,
    ): Promise<{ readonly rows: Row[]; readonly sequence: number | undefined }> {
        // rank the matching rows of the partitions by the order
        const dialect = this.database.dialect;
        const projection = new Projection(this.#fields(table, namespace));
        const column = table[TABLE].column(partition.column);
        const rank = sql`ROW_NUMBER() OVER (PARTITION BY ${column} ORDER BY ${sql.join(
            Order.render(order, table, namespace),
            sql`, `,
        )})`;
        const ranked = sql`SELECT ${projection.sql()}, ${rank} AS ${sql.identifier(RANK)} FROM ${table}
            WHERE ${and(selection, inArray(column, partition.values))}`;

        // keep each partition's first rows
        const statement =
            limit === undefined
                ? ranked
                : sql`SELECT * FROM (${ranked}) AS ${sql.identifier("ranked")}
                    WHERE ${sql.identifier("ranked")}.${sql.identifier(RANK)} <= ${limit}`;
        const read = await this.database.values(statement);

        return this.#take(projection.decode(read, dialect, new Set(), this.database.state.cipher));
    }

    /** Select a table's logged columns and computed values beside the log's head. */
    #fields(table: Table, namespace: Namespace) {
        const extras = Object.fromEntries(
            Object.entries(namespace.extras).map(([name, expression]) => [
                name,
                Expression.select(expression, table, namespace),
            ]),
        );

        return {
            head: headFields(this.database.dialect),
            row: { ...table[TABLE].logged, ...extras },
        };
    }

    /** Take the rows of a read and its head's sequence, absent when live or without rows. */
    #take(read: readonly { readonly head: Head; readonly row: Row }[]): {
        readonly rows: Row[];
        readonly sequence: number | undefined;
    } {
        const [first] = read;
        const sequence =
            this.position === undefined || first === undefined
                ? undefined
                : this.#require(first.head.epoch, latestOf(first.head.logged, first.head.horizon));

        return { rows: read.map((entry) => entry.row), sequence };
    }

    /** Read the log's latest sequence within the position's epoch. */
    async #latest(): Promise<number> {
        const latest = await this.database.log.position();

        return this.#require(latest.epoch, latest.sequence);
    }

    /** Read the rows' images at the position under the overlay, none when live without one. */
    async #since(table: Table, read?: number): Promise<ReadonlyMap<string, Row | null>> {
        // put the overlay's rows over the position's images
        const images = await this.#rewound(table, read);
        const overlay = await this.#overlay?.(table);

        return overlay === undefined || overlay.size === 0
            ? images
            : new Map([...images, ...overlay]);
    }

    /** Read the rows' images at the position, none when live. */
    async #rewound(table: Table, read?: number): Promise<ReadonlyMap<string, Row | null>> {
        // show the live database unchanged
        if (this.position === undefined) {
            return new Map();
        }

        // take the sequence of the read
        const sequence = read ?? (await this.#latest());

        // undo the committed changes after the position
        const images = await this.#rewind(table, this.position.sequence, sequence);
        if (!this.database.driver.transaction) {
            return images;
        }

        // undo the transaction's own writes
        const undone = new Map(images);
        for (const change of await this.database.log.written([table])) {
            const name = Key.name(table, change.key);
            if (!undone.has(name)) {
                undone.set(name, Change.before(change));
            }
        }

        return undone;
    }

    /** Require a head of the position's epoch, returning its sequence. */
    #require(epoch: string, sequence: number): number {
        if (this.position !== undefined && epoch !== this.position.epoch) {
            throw new DatabaseError(
                "STALE_EPOCH",
                `position of epoch ${this.position.epoch} is not in the log's epoch ${epoch}`,
            );
        }

        return sequence;
    }
}

/** Type rows a snapshot decoded by a table's logged columns as that table's logged records. */
function logged<Definition extends Table>(rows: Row[]): (RowImage<Definition> & Row)[];
/**
 * Pass the rows on, whose decoders and log keep exactly the table's logged columns.
 *
 * @construct the snapshot decoded every row through the logged columns, as RowImage maps the table.
 */
function logged(rows: Row[]): Row[] {
    return rows;
}

/** Render a predicate over a table. */
function render(predicate: Predicate, table: Table, namespace?: Namespace): SQL {
    return Predicate.render(predicate, Predicate.bind(table, {}, namespace));
}

/** Add a row's computed values. */
function augment(row: Row, extras: Extras, related?: RelationReader): Row {
    const entries = Object.entries(extras);

    return entries.length === 0
        ? row
        : {
              ...row,
              ...Object.fromEntries(
                  entries.map(([name, expression]) => [
                      name,
                      Expression.evaluate(expression, row, related),
                  ]),
              ),
          };
}

/** Decide a predicate on a row and read relations only when needed. */
async function decides(
    match: Match,
    predicate: Predicate,
    table: Table,
    row: Row,
    relations: RelationView | undefined,
): Promise<boolean> {
    // decide only by the fields
    const binding = {
        field: (name: string) => row[name],
        placeholder: Predicate.unbound,
        exists: () => undefined,
        search: (words: readonly string[]) => table[TABLE].requireFullText().matches(row, words),
    };
    const decided = match(binding);
    if (decided !== undefined || relations === undefined) {
        return decided === true;
    }

    // read each relation's answer, by relation and condition
    const answers = new Map<string, Map<Condition, boolean>>();
    for (const { via, where } of Predicate.relations(predicate)) {
        const answered = answers.get(via) ?? new Map<Condition, boolean>();
        answered.set(where, await relations.decide(via, where, row));
        answers.set(via, answered);
    }

    return (
        match({
            ...binding,
            exists: (via, where) => answers.get(via)?.get(where),
        }) === true
    );
}

/** Build the statement reading the head and the rows with listed tuples. */
function tupleRead(table: Table, columns: readonly string[], dialect: Dialect): Statement {
    return table[TABLE].statement(`tuple:${dialect}:${columns.join(",")}`, () => {
        // join each tuple to its rows and keep a head row for an empty tuple
        const loggedColumns = Object.values(table[TABLE].logged);
        const head = sql.join(
            HEAD_COLUMNS.map((name) => sql`head.${sql.identifier(name)}`),
            sql`, `,
        );

        return new Statement(
            (value) => sql`SELECT ${head}, ${sql.join(loggedColumns, sql`, `)}
                FROM (${selectHead(dialect)}) AS head
                CROSS JOIN ${jsonElements(value("tuples"), "wanted")}
                LEFT JOIN ${table} ON ${sql.join(
                    columns.map(
                        (column, index) =>
                            sql`${table[TABLE].column(column)} = ${elementValue(table[TABLE].column(column), index, dialect)}`,
                    ),
                    sql` AND `,
                )}`,
        );
    });
}

/** Read a driver row, an array of values, by the property of each selected column. */
function decodeValues(
    columns: readonly (readonly [string, Column])[],
    values: readonly unknown[],
    dialect: Dialect,
): Row {
    const read: Record<string, ColumnValue> = {};
    for (const [position, [property, column]] of columns.entries()) {
        const value = values[position];
        read[property] = value === null ? null : column.definition.decode(value, dialect);
    }

    return read;
}

/** Split rows into the windows of a partition's values, or one window without a partition. */
function windowsOf(table: Table, rows: readonly Row[], partition: Partition | undefined): Row[][] {
    // keep every row in one window
    if (partition === undefined) {
        return [[...rows]];
    }

    // name each partition's window by its value in JSON form
    const column = table[TABLE].column(partition.column).definition;
    const name = (value: ColumnValue) =>
        JSON.stringify(value === null ? null : column.toJson(value));
    const windows = partition.values.map((value) => {
        const window: Row[] = [];

        return { name: name(value), window };
    });
    const byName = new Map(windows.map(({ name: named, window }) => [named, window]));

    // place each row in the window of its column's value
    for (const row of rows) {
        const value = row[partition.column];
        if (value !== undefined) {
            byName.get(name(value))?.push(row);
        }
    }

    return windows.map(({ window }) => window);
}
