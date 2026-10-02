import { schema } from "@destack/schema";
import { CHAIN_TERMS, and, inArray, sql, type SQL } from "../sql/index.ts";
import { Projection } from "../query/selection.ts";
import type { DatabaseConnection } from "../database/connection.ts";
import { Condition, type Match } from "../query/condition.ts";
import { Key } from "../query/key.ts";
import { Change } from "./log.ts";
import { Order } from "../query/order.ts";
import type { Computed, Namespace } from "../query/namespace.ts";
import { Expression, type Related } from "../expression/expression.ts";
import { TABLE, Table, type Logged } from "../table/table.ts";
import { headFields, latestOf, LogInteger, selectHead, type LogPosition } from "./position.ts";
import type { Row } from "../table/row.ts";
import type { Column, ColumnValue } from "../table/column.ts";
import { jsonElements, Statement } from "../query/statement.ts";
import { DatabaseError } from "../error/error.ts";
import type { Dialect } from "../dialect/dialect.ts";

/** The name of a ranked read's rank column. */
const RANK = "destack_rank";

/** The head columns a tuple read selects before each row's own. */
const HEAD_COLUMNS = ["epoch", "logged", "horizon"] as const;

/** The head of a read: the epoch, the newest logged and the highest compacted sequence. */
const HEAD = schema.tuple([schema.string(), LogInteger.nullable(), LogInteger]);

/** The most keys one read selects by key. */
const KEYS_PER_READ = CHAIN_TERMS;

/** A read of the first matching rows in an order. */
export interface OrderedRead {
    /** The condition the rows meet. */
    readonly where: Condition;
    /** The order of the rows. */
    readonly order: Order;
    /** The most rows of each window. */
    readonly count: number;
    /** What the read admits beyond the condition. */
    readonly admits?: Admission;
    /** The computed values and relations the condition and order read. */
    readonly namespace?: Namespace;
    /** How the read follows the table's relations in memory. */
    readonly relations?: RelationView;
}

/** The log's head as one statement saw it: its epoch, newest logged sequence and highest compacted one. */
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
    /** The values, each naming one partition. */
    readonly values: readonly ColumnValue[];
}

/** Read the images a table's rows had before their first change between two sequences, by key. */
export type Rewind = (
    table: Table,
    after: number,
    upto: number,
) => Promise<ReadonlyMap<string, Row | null>>;

/** Read the rows a layer puts over a table's rows, by key name: a row, or null for a removed one. */
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
    decide(via: string, where: Condition | undefined, row: Row): Promise<boolean>;
    /** Read the keys of rows with related rows that changed between two sequences. */
    touched(after: number, upto: number): Promise<readonly Row[]>;
    /** Resolve a row's lookups, as of the position. */
    resolve(row: Row): Promise<Related>;
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
    /** The rows changed related rows touched, read once per relations, table and sequence. */
    readonly #touched = new WeakMap<RelationView, Map<string, Promise<Map<string, Row | null>>>>();

    /** Show a database as of a position, under an overlay when one is given. */
    constructor(
        database: DatabaseConnection,
        position: LogPosition | undefined,
        rewind?: Rewind,
        overlay?: Overlay,
    ) {
        // keep the database, the position, and how earlier and overlaid rows read
        this.database = database;
        this.position = position;
        this.#rewind = rewind ?? database.log.rewind();
        this.#overlay = overlay;
    }

    /** Show the same database and position under an overlay. */
    layer(overlay: Overlay): Snapshot {
        return new Snapshot(this.database, this.position, this.#rewind, overlay);
    }

    /** Show the live database with its transaction's writes. */
    static live(database: DatabaseConnection): Snapshot {
        return new Snapshot(database, undefined);
    }

    /** Read a table's rows matching a condition. */
    async rows<Definition extends Table>(
        table: Definition,
        where: Condition,
    ): Promise<Logged<Definition>[]> {
        // read the current rows and the changed rows' images
        const { rows, sequence } = await this.#read(table, render(where, table));
        const images = await this.#since(table, sequence);

        // keep unchanged rows and matching images
        const match = Condition.compile(where, table);
        const kept =
            images.size === 0 ? rows : rows.filter((row) => !images.has(Key.name(table, row)));
        for (const image of images.values()) {
            if (image !== null && Condition.matches(match, image)) {
                kept.push(image);
            }
        }

        return logged(kept);
    }

    /** Read a table's rows at the position with text columns that have one of some tuples. */
    async select<Definition extends Table>(
        table: Definition,
        columns: readonly string[],
        tuples: readonly (readonly unknown[])[],
    ): Promise<Logged<Definition>[]> {
        // read the head and each tuple's rows in one statement
        if (tuples.length === 0) {
            return [];
        }
        const read = await tupleRead(table, columns, this.database.dialect).values(this.database, {
            tuples: JSON.stringify(tuples),
        });
        const [epoch, latest, horizon] = HEAD.parse(read[0]?.slice(0, HEAD_COLUMNS.length));
        const sequence = this.#require(epoch, latestOf(latest, horizon));

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
            wanted.has(JSON.stringify(columns.map((column) => toJson(table, column, row))));
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
    ): Promise<Logged<Definition> | null> {
        // read the row and its earlier image
        const { rows, sequence } = await this.#read(table, Key.match(table, key));
        const images = await this.#since(table, sequence);
        const name = Key.name(table, key);
        const row = images.has(name) ? images.get(name) : rows[0];
        const [present = null] = row === undefined || row === null ? [] : logged<Definition>([row]);

        return present;
    }

    /** Read up to a count of admitted matching rows in an order after a row, with their computed values. */
    async ordered<Definition extends Table>(
        table: Definition,
        query: OrderedRead & { readonly after?: Row },
    ): Promise<(Logged<Definition> & Row)[]> {
        const [rows] = await this.#windows(table, query, undefined);
        if (rows === undefined) {
            throw new RangeError("an ordered read lacks its one window");
        }

        return logged(rows);
    }

    /** Read up to a count of admitted matching rows of each partition some values of a column name, in an order, aligned with the values. */
    async windows<Definition extends Table>(
        table: Definition,
        query: OrderedRead & { readonly partition: Partition },
    ): Promise<(Logged<Definition> & Row)[][]> {
        const windows = await this.#windows(table, query, query.partition);

        return windows.map((window) => logged(window));
    }

    /** Read the first rows of one window, or of each partition's window in one ranked read per round. */
    async #windows(
        table: Table,
        query: OrderedRead & { readonly after?: Row },
        partition: Partition | undefined,
    ): Promise<Row[][]> {
        // read enough current rows to cover every changed row
        const admits = query.admits;
        const isAdmittedInMemory = admits !== undefined && admits.current === undefined;
        const order = Order.complete(query.order, table);
        const namespace = query.namespace ?? { computed: {} };
        const relations = query.relations;
        let changed = -1;
        let reached = this.position?.sequence;
        const overlay = (await this.#overlay?.(table)) ?? new Map<string, Row | null>();
        const images = new Map(overlay);
        let unsettled = new Map(overlay);
        let rows: Row[] = [];
        while (unsettled.size > changed) {
            changed = unsettled.size;
            const selection = and(
                render(query.where, table, namespace),
                admits?.current,
                query.after === undefined
                    ? undefined
                    : Order.after(order, table, query.after, namespace),
            );
            const limit = isAdmittedInMemory ? undefined : query.count + changed;
            const read =
                partition === undefined
                    ? await this.#read(table, selection, order, limit, namespace)
                    : await this.#ranked(table, selection, partition, order, limit, namespace);
            rows = read.rows;

            // take the rows as they are when live
            const position = this.position;
            if (position === undefined || reached === undefined) {
                break;
            }

            // extend the images past the earlier read
            const sequence = read.sequence ?? (await this.#latest());
            for (const [name, image] of await this.#rewind(table, reached, sequence)) {
                if (!images.has(name)) {
                    images.set(name, image);
                }
            }
            reached = Math.max(reached, sequence);
            const settling = new Map(images);
            const touched =
                relations === undefined
                    ? new Map<string, Row | null>()
                    : await this.#settled(table, relations, position.sequence, sequence);
            for (const [name, row] of touched) {
                if (!settling.has(name)) {
                    settling.set(name, row);
                }
            }
            unsettled = settling;
        }

        // overlay the matching admitted images
        const match = Condition.compile(query.where, table);
        const current =
            unsettled.size === 0
                ? rows
                : rows.filter((row) => !unsettled.has(Key.name(table, row)));
        const decided = isAdmittedInMemory
            ? await Promise.all(current.map((row) => admits.image(row)))
            : undefined;
        const kept =
            decided === undefined ? current : current.filter((_, index) => decided[index] === true);
        for (const image of unsettled.values()) {
            const augmented =
                image === null
                    ? null
                    : augment(image, namespace.computed, await relations?.resolve(image));
            if (
                augmented !== null &&
                (await decides(match, query.where, augmented, relations)) &&
                (query.after === undefined || Order.rows(order, augmented, query.after) > 0) &&
                (admits === undefined || (await admits.image(augmented)))
            ) {
                kept.push(augmented);
            }
        }

        // sort each window and keep its first rows
        const windows = windowsOf(table, kept, partition);

        return windows.map((window) =>
            window.toSorted((left, right) => Order.rows(order, left, right)).slice(0, query.count),
        );
    }

    /** Read the rows changed related rows touched between the position and a sequence, once per relations, table and sequence. */
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
        const definition = table[TABLE];
        const isText = definition.key.every(
            (property) => definition.column(property).definition.kind === "text",
        );

        // read text keys in one statement, and others a chain at a time
        if (isText) {
            const tuples = keys.map((key) =>
                definition.key.map((property) => Key.value(table, key, property)),
            );
            for (const row of await this.select(table, definition.key, tuples)) {
                found.set(Key.name(table, row), row);
            }
        } else {
            for (let start = 0; start < keys.length; start += KEYS_PER_READ) {
                const matches = Key.any(table, keys.slice(start, start + KEYS_PER_READ));
                for (const row of await this.rows(table, matches)) {
                    found.set(Key.name(table, row), row);
                }
            }
        }

        return found;
    }

    /** Read a table's logged columns of the admitted rows with the log's head in one statement, the head's sequence absent when live or without rows. */
    async #read(
        table: Table,
        selection: SQL,
        order?: Order,
        limit?: number,
        namespace: Namespace = { computed: {} },
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

    /** Read the first rows of each partition's window in one statement, ranked within the partition, with the log's head. */
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

        return this.#take(
            projection.decode<{ readonly head: Head; readonly row: Row }>(read, dialect, new Set()),
        );
    }

    /** Select a table's logged columns and computed values beside the log's head. */
    #fields(table: Table, namespace: Namespace) {
        const computed = Object.fromEntries(
            Object.entries(namespace.computed).map(([name, expression]) => [
                name,
                Expression.select(expression, table, namespace),
            ]),
        );

        return {
            head: headFields(this.database.dialect),
            row: { ...table[TABLE].logged, ...computed },
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
function logged<Definition extends Table>(rows: Row[]): (Logged<Definition> & Row)[];
/** Pass the rows on, whose decoders and log keep exactly the table's logged columns. */
function logged(rows: Row[]): Row[] {
    return rows;
}

/** Render a condition over a table. */
function render(where: Condition, table: Table, namespace: Namespace = { computed: {} }): SQL {
    return Condition.render(where, Condition.bind(table, {}, namespace));
}

/** Add a row's computed values. */
function augment(row: Row, computed: Computed, related?: Related): Row {
    const entries = Object.entries(computed);

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

/** Decide a condition on a row and read relations only when needed. */
async function decides(
    match: Match,
    where: Condition,
    row: Row,
    relations: RelationView | undefined,
): Promise<boolean> {
    // decide by the columns alone
    const binding = {
        column: (name: string) => row[name],
        parameter: () => null,
        exists: () => undefined,
    };
    const decided = match(binding);
    if (decided !== undefined || relations === undefined) {
        return decided === true;
    }

    // read each relation's answer and decide again
    const answers = new Map<string, boolean>();
    for (const { via, where: related } of Condition.relations(where)) {
        answers.set(
            JSON.stringify([via, related ?? null]),
            await relations.decide(via, related, row),
        );
    }

    return (
        match({
            ...binding,
            exists: (via, related) => answers.get(JSON.stringify([via, related ?? null])),
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
                            sql`${table[TABLE].column(column)} = wanted.value ->> ${sql.raw(String(index))}`,
                    ),
                    sql` AND `,
                )}`,
        );
    });
}

/** Write a row's column value in JSON form. */
function toJson(table: Table, column: string, row: Row): unknown {
    const value = row[column];

    return value === undefined || value === null
        ? null
        : table[TABLE].column(column).definition.toJson(value);
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
