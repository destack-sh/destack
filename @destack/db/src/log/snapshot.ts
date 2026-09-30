import { and, sql, type SQL } from "drizzle-orm";
import type { DatabaseConnection } from "../database/connection.ts";
import { Condition, type Match } from "../query/condition.ts";
import { CHAIN_TERMS } from "../query/predicate.ts";
import { Key } from "../query/key.ts";
import { Order } from "../query/order.ts";
import type { Computed, Namespace } from "../query/namespace.ts";
import { Expression, type Related } from "../expression/expression.ts";
import { TABLE, type Table } from "../table/table.ts";
import { latestOf, selectHead, type LogPosition } from "./position.ts";
import { fromDriver, type Row } from "../table/row.ts";
import { jsonElements, Statement } from "../query/statement.ts";
import { DatabaseError } from "../error/error.ts";
import type { Dialect } from "../dialect/dialect.ts";

/** The head columns a tuple read selects before each row's own. */
const HEAD_COLUMNS = ["epoch", "logged", "horizon"] as const;

/** The keys one read by key names. */
const KEYS_PER_READ = CHAIN_TERMS;

/** Read the images a table's rows had before their first change between two sequences, by key. */
export type Rewind = (
    table: Table,
    after: number,
    upto: number,
) => Promise<ReadonlyMap<string, Row | null>>;

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

    /** Show a database as of a position. */
    constructor(database: DatabaseConnection, position: LogPosition | undefined, rewind?: Rewind) {
        this.database = database;
        this.position = position;
        this.#rewind = rewind ?? database.log.rewind();
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

    /** Read a table's rows at the position with text columns that hold one of some tuples. */
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
        const [epoch, latest, horizon] = read[0]!;
        const sequence = this.#require(epoch as string, latestOf(latest, horizon));

        // decode the rows
        const dialect = this.database.driver.native.dialect;
        const selected = Object.entries(table[TABLE].logged);
        const key =
            HEAD_COLUMNS.length +
            selected.findIndex(([property]) => property === table[TABLE].key[0]);
        const rows = read
            .filter((values) => values[key] !== null)
            .map((values) => fromDriver(selected, values.slice(HEAD_COLUMNS.length), dialect));
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
        const images = new Map<string, Row | null>();
        let unsettled = new Map<string, Row | null>();
        let rows: Row[] = [];
        while (unsettled.size > changed) {
            changed = unsettled.size;
            const selection = and(
                render(query.where, table, namespace),
                admits?.current,
                query.after === undefined
                    ? undefined
                    : Order.after(order, table, query.after, namespace),
            )!;
            rows = await this.#read(
                table,
                selection,
                order,
                isAdmittedInMemory ? undefined : query.count + changed,
                namespace,
            );

            // take the rows as they are when live
            if (reached === undefined) {
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
            unsettled = new Map(images);
            const touched = (
                (await relations?.touched(this.position!.sequence, sequence)) ?? []
            ).filter((key) => !unsettled.has(Key.name(table, key)));
            for (const [name, row] of await this.#rowsOf(table, touched)) {
                unsettled.set(name, row);
            }
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
        const kept = decided === undefined ? current : current.filter((_, index) => decided[index]);
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

        return kept.sort((left, right) => Order.rows(order, left, right)).slice(0, query.count);
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
            Object.entries(namespace.computed).map(([name, expression]) => {
                const value = Expression.render(expression, table, namespace);

                return [
                    name,
                    Expression.kind(expression, table, namespace) === "text"
                        ? value.mapWith(String)
                        : value.mapWith(Number),
                ];
            }),
        );
        const query = this.database
            .select({ ...table[TABLE].logged, ...computed })
            .from(table)
            .where(selection);
        const ordered =
            order === undefined ? query : query.orderBy(...Order.render(order, table, namespace));
        const rows = (await (limit === undefined ? ordered : ordered.limit(limit))) as Row[];

        return rows;
    }

    /** Read the log's latest sequence within the position's epoch. */
    async #latest(): Promise<number> {
        const latest = await this.database.log.position();

        return this.#require(latest.epoch, latest.sequence);
    }

    /** Read the rows' images at the position, none when live. */
    async #since(table: Table, read?: number): Promise<ReadonlyMap<string, Row | null>> {
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
                undone.set(name, (change.before as Row | undefined) ?? null);
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
    const names = Object.keys(computed);

    return names.length === 0
        ? row
        : {
              ...row,
              ...Object.fromEntries(
                  names.map((name) => [name, Expression.evaluate(computed[name]!, row, related)]),
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

/** Build the statement reading the head and the rows holding listed tuples. */
function tupleRead(table: Table, columns: readonly string[], dialect: Dialect): Statement {
    return table.statement(`tuple:${dialect}:${columns.join(",")}`, () => {
        // join each tuple to its rows and keep a head row for an empty tuple
        const logged = Object.values(table[TABLE].logged);
        const definitions = table[TABLE].columns;
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
                            sql`${definitions[column]!} = wanted.value ->> ${sql.raw(String(index))}`,
                    ),
                    sql` AND `,
                )}`,
        );
    });
}

/** Write a row's column value in JSON form. */
function toJson(table: Table, column: string, row: Row): unknown {
    return table[TABLE].columns[column]!.definition.toJson(row[column]);
}
