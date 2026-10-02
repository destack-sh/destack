import { schema } from "@destack/schema";
import { CHAIN_TERMS, and, sql, type SQL } from "../sql/index.ts";
import type { DatabaseConnection } from "../database/connection.ts";
import { Condition, type Match } from "../query/condition.ts";
import { Key } from "../query/key.ts";
import { Order } from "../query/order.ts";
import type { Computed, Namespace } from "../query/namespace.ts";
import { Expression, type Related } from "../expression/expression.ts";
import { TABLE, Table } from "../table/table.ts";
import { latestOf, LogInteger, selectHead, type LogPosition } from "./position.ts";
import type { Row } from "../table/row.ts";
import type { Column, ColumnValue } from "../table/column.ts";
import { jsonElements, Statement } from "../query/statement.ts";
import { DatabaseError } from "../error/error.ts";
import type { Dialect } from "../dialect/dialect.ts";

/** The head columns a tuple read selects before each row's own. */
const HEAD_COLUMNS = ["epoch", "logged", "horizon"] as const;

/** The head of a read: the epoch, the newest logged and the highest compacted sequence. */
const HEAD = schema.tuple([schema.string(), LogInteger.nullable(), LogInteger]);

/** The most keys one read selects by key. */
const KEYS_PER_READ = CHAIN_TERMS;

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
    async rows(table: Table, where: Condition): Promise<Row[]> {
        // read the current rows and the changed rows' images
        const rows = await this.#read(table, render(where, table));
        const images = await this.#since(table);

        // keep unchanged rows and matching images
        const match = Condition.compile(where, table);
        const kept =
            images.size === 0 ? rows : rows.filter((row) => !images.has(Key.name(table, row)));
        for (const image of images.values()) {
            if (image !== null && Condition.matches(match, image)) {
                kept.push(image);
            }
        }

        return kept;
    }

    /** Read a table's rows at the position with text columns that have one of some tuples. */
    async select(
        table: Table,
        columns: readonly string[],
        tuples: readonly (readonly unknown[])[],
    ): Promise<Row[]> {
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

        return kept;
    }

    /** Read one row by its key, absent when it did not exist at the position. */
    async row(table: Table, key: Row): Promise<Row | undefined> {
        // read the row and its earlier image
        const rows = await this.#read(table, Key.match(table, key));
        const images = await this.#since(table);
        const name = Key.name(table, key);

        return images.has(name) ? (images.get(name) ?? undefined) : rows[0];
    }

    /** Read up to a count of admitted matching rows in an order after a row. */
    async ordered(
        table: Table,
        query: {
            readonly where: Condition;
            readonly order: Order;
            readonly after?: Row;
            readonly count: number;
            readonly admits?: Admission;
            readonly namespace?: Namespace;
            readonly relations?: RelationView;
        },
    ): Promise<Row[]> {
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
            rows = await this.#read(
                table,
                selection,
                order,
                isAdmittedInMemory ? undefined : query.count + changed,
                namespace,
            );

            // take the rows as they are when live
            const position = this.position;
            if (position === undefined || reached === undefined) {
                break;
            }

            // extend the images past the earlier read
            const sequence = await this.#latest();
            for (const [name, image] of await this.#rewind(table, reached, sequence)) {
                if (!images.has(name)) {
                    images.set(name, image);
                }
            }
            reached = Math.max(reached, sequence);
            const settling = new Map(images);
            const touched = ((await relations?.touched(position.sequence, sequence)) ?? []).filter(
                (key) => !settling.has(Key.name(table, key)),
            );
            for (const [name, row] of await this.#rowsOf(table, touched)) {
                settling.set(name, row);
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

        return kept.toSorted((left, right) => Order.rows(order, left, right)).slice(0, query.count);
    }

    /** Read rows by key as of the position, null for missing keys. */
    async #rowsOf(table: Table, keys: readonly Row[]): Promise<Map<string, Row | null>> {
        // start every key as missing
        const found = new Map<string, Row | null>(keys.map((key) => [Key.name(table, key), null]));
        for (let start = 0; start < keys.length; start += KEYS_PER_READ) {
            // match the chunk's keys
            const matches = Key.any(table, keys.slice(start, start + KEYS_PER_READ));
            for (const row of await this.rows(table, matches)) {
                found.set(Key.name(table, row), row);
            }
        }

        return found;
    }

    /** Read a table's logged columns of the admitted rows, as of the position. */
    async #read(
        table: Table,
        selection: SQL,
        order?: Order,
        limit?: number,
        namespace: Namespace = { computed: {} },
    ): Promise<Row[]> {
        // read the rows with their computed values
        const computed = Object.fromEntries(
            Object.entries(namespace.computed).map(([name, expression]) => [
                name,
                Expression.select(expression, table, namespace),
            ]),
        );
        const query = this.database
            .select({ ...table[TABLE].logged, ...computed })
            .from(table)
            .where(selection);
        const ordered =
            order === undefined ? query : query.orderBy(...Order.render(order, table, namespace));

        return limit === undefined ? ordered : ordered.limit(limit);
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
                undone.set(name, change.before ?? null);
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
        const logged = Object.values(table[TABLE].logged);
        const head = sql.join(
            HEAD_COLUMNS.map((name) => sql`head.${sql.identifier(name)}`),
            sql`, `,
        );

        return new Statement(
            (value) => sql`SELECT ${head}, ${sql.join(logged, sql`, `)}
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
