import { and, sql, type SQL } from "drizzle-orm";
import type { DatabaseConnection } from "../database/connection.ts";
import { Condition, type Match } from "../query/condition.ts";
import { CHAIN_TERMS } from "../query/predicate.ts";
import { Key } from "../query/key.ts";
import { Order } from "../query/order.ts";
import type { Computed, Namespace } from "../query/namespace.ts";
import { Expression, type Related } from "../query/expression.ts";
import { TABLE, type Table } from "../table/table.ts";
import { latestOf, selectHead, type LogPosition } from "./position.ts";
import { fromDriver, type Row } from "../table/row.ts";
import { jsonElements, Statement } from "../query/statement.ts";
import { DatabaseError } from "../error/error.ts";

/** The log head's columns a tuple read selects before each row's own, so the rows and their position come from one statement. */
const HEAD_COLUMNS = ["epoch", "logged", "horizon"] as const;

/** The keys one read of rows by key names: one flat chain, which SQLite plans as index lookups where nested chains scan. */
const KEYS_PER_READ = CHAIN_TERMS;

/**
 * Read the images a table's rows had before their first change after one sequence, up to another, by key.
 *
 * A row inserted after the sequence has a null image.
 */
export type Rewind = (
    table: Table,
    after: number,
    upto: number,
) => Promise<ReadonlyMap<string, Row | null>>;

/** What an ordered read admits beyond a condition: SQL over current rows, and the same decision on earlier images. */
export interface Admission {
    /** Match the current rows admitted. */
    readonly current: SQL;
    /** Decide whether an earlier image is admitted. */
    readonly image: (row: Row) => Promise<boolean>;
}

/** How a snapshot follows a table's relations in memory as of its position, which its namespace follows in SQL over current rows. */
export interface RelationView {
    /** Decide whether a related row met a condition for a row, as of the snapshot's position. */
    decide(via: string, where: Condition | undefined, row: Row): Promise<boolean>;
    /** Read the keys of the rows whose related rows changed after one sequence, up to another. */
    touched(after: number, upto: number): Promise<readonly Row[]>;
    /** Resolve the related values a row's lookups read, as of the snapshot's position. */
    resolve(row: Row): Promise<Related>;
}

/**
 * The database as it was at a log position: current rows, with the images later changes replaced.
 *
 * It reads logged columns, which the log records the images of, exactly as they were at the position.
 * It reaches back to the log's horizon for tables whose changes compaction removes, and through all history otherwise.
 */
export class Snapshot {
    /** The database read. */
    readonly database: DatabaseConnection;
    /** The position shown, absent for the database as each read finds it. */
    readonly position: LogPosition | undefined;
    /** Where the earlier images of changed rows come from. */
    readonly #rewind: Rewind;

    /** Show a database as of a position, reading the images of later changes once from the log unless given another source. */
    constructor(database: DatabaseConnection, position: LogPosition | undefined, rewind?: Rewind) {
        this.database = database;
        this.position = position;
        this.#rewind = rewind ?? database.log.rewind();
    }

    /** Show a database as each read finds it, with its transaction's own writes, as decisions made now read it. */
    static live(database: DatabaseConnection): Snapshot {
        return new Snapshot(database, undefined);
    }

    /** Read a table's rows a condition over its columns matches. */
    async rows(table: Table, where: Condition): Promise<Row[]> {
        // read the current rows, and the images of rows changed since the position
        const { rows, sequence } = await this.#read(table, render(where, table));
        const images = await this.#since(table, sequence);

        // keep unchanged rows, and the earlier images the condition matches
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

    /**
     * Read a table's rows whose text columns hold one of some tuples of values in their JSON form, as of the position.
     *
     * One prepared statement per table and columns reads the log's head with the rows, and the tuples drive the table's index one at a time.
     * The images of rows changed since the position count where they hold one of the tuples.
     */
    async select(
        table: Table,
        columns: readonly string[],
        tuples: readonly (readonly unknown[])[],
    ): Promise<Row[]> {
        // read the head, then the current rows each tuple names, in one statement
        if (tuples.length === 0) {
            return [];
        }
        const read = await tupleRead(table, columns).values(this.database, {
            tuples: JSON.stringify(tuples),
        });
        const [epoch, latest, horizon] = read[0]!;
        const sequence = this.#require(epoch as string, latestOf(latest, horizon));

        // decode the rows, skipping the tuples that named none
        const dialect = this.database.driver.native.dialect;
        const selected = Object.entries(table[TABLE].logged);
        const key =
            HEAD_COLUMNS.length +
            selected.findIndex(([property]) => property === table[TABLE].key[0]);
        const rows = read
            .filter((values) => values[key] !== null)
            .map((values) => fromDriver(selected, values.slice(HEAD_COLUMNS.length), dialect));
        const images = await this.#since(table, sequence);

        // keep unchanged rows, and the earlier images holding one of the tuples
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
        // read the row now, and its earlier image when it changed since the position
        const { rows, sequence } = await this.#read(table, Key.match(table, key));
        const images = await this.#since(table, sequence);
        const name = Key.name(table, key);

        return images.has(name) ? (images.get(name) ?? undefined) : rows[0];
    }

    /** Read up to a count of a table's rows a condition matches, in an order after a row, among the rows an admission admits. */
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
        // read enough current rows to outnumber the count by every row changed, or whose relations changed, since the position
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
                query.admits?.current,
                query.after === undefined
                    ? undefined
                    : Order.after(order, table, query.after, namespace),
            )!;
            const read = await this.#read(
                table,
                selection,
                order,
                query.count + changed,
                namespace,
            );
            rows = read.rows;

            // take the rows as they are for the live database
            if (reached === undefined) {
                break;
            }

            // extend the images by the first changes of rows beyond the sequence read before
            for (const [name, image] of await this.#rewind(table, reached, read.sequence)) {
                if (!images.has(name)) {
                    images.set(name, image);
                }
            }
            reached = Math.max(reached, read.sequence);
            unsettled = new Map(images);
            const touched = (
                (await relations?.touched(this.position!.sequence, read.sequence)) ?? []
            ).filter((key) => !unsettled.has(Key.name(table, key)));
            for (const [name, row] of await this.#rowsOf(table, touched)) {
                unsettled.set(name, row);
            }
        }

        // overlay the rows as they were of changed rows and of rows whose relations changed, which the condition and the admission admit
        const match = Condition.compile(query.where, table);
        const kept =
            unsettled.size === 0
                ? rows
                : rows.filter((row) => !unsettled.has(Key.name(table, row)));
        for (const image of unsettled.values()) {
            const augmented =
                image === null
                    ? null
                    : augment(image, namespace.computed, await relations?.resolve(image));
            if (
                augmented !== null &&
                (await decides(match, query.where, augmented, relations)) &&
                (query.after === undefined || Order.rows(order, augmented, query.after) > 0) &&
                (query.admits === undefined || (await query.admits.image(augmented)))
            ) {
                kept.push(augmented);
            }
        }

        return kept.sort((left, right) => Order.rows(order, left, right)).slice(0, query.count);
    }

    /** Read rows by their keys as of the position, a chunk of keys per read, null for keys no row held. */
    async #rowsOf(table: Table, keys: readonly Row[]): Promise<Map<string, Row | null>> {
        // name every key missing until a read finds its row
        const found = new Map<string, Row | null>(keys.map((key) => [Key.name(table, key), null]));
        for (let start = 0; start < keys.length; start += KEYS_PER_READ) {
            // match the chunk's keys by their JSON values
            const matches = Key.any(table, keys.slice(start, start + KEYS_PER_READ));
            for (const row of await this.rows(table, matches)) {
                found.set(Key.name(table, row), row);
            }
        }

        return found;
    }

    /**
     * Read a table's logged columns of the rows a selection admits, and a log sequence the rows are at or before, within the position's epoch.
     *
     * Reading the sequence after the rows suffices: rows changed since the position read as their earlier images.
     */
    async #read(
        table: Table,
        selection: SQL,
        order?: Order,
        limit?: number,
        namespace: Namespace = { computed: {} },
    ): Promise<{ readonly rows: Row[]; readonly sequence: number }> {
        // read the rows with their computed values, then the latest position, which is at or after the rows' state
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

        return { rows, sequence: await this.#latest() };
    }

    /** Read the log's latest sequence, which reads made before it are at or before, within the position's epoch. */
    async #latest(): Promise<number> {
        const latest = await this.database.log.position();

        return this.#require(latest.epoch, latest.sequence);
    }

    /** Read the images rows changed after the position up to a sequence had, none for the live database. */
    #since(table: Table, sequence: number): Promise<ReadonlyMap<string, Row | null>> {
        return this.position === undefined
            ? Promise.resolve(new Map())
            : this.#rewind(table, this.position.sequence, sequence);
    }

    /** Require a head of the position's history, which a restore replaces with a new epoch, returning its sequence. */
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

/** Render a condition over a table's columns and namespace. */
function render(where: Condition, table: Table, namespace: Namespace = { computed: {} }): SQL {
    return Condition.render(where, Condition.bind(table, {}, namespace));
}

/** Add a row's computed values to it, reading related rows through the host. */
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

/** Decide a compiled condition on a row as of the position, reading its relations only when the columns leave it unknown. */
async function decides(
    match: Match,
    where: Condition,
    row: Row,
    relations: RelationView | undefined,
): Promise<boolean> {
    // decide by the columns alone, leaving relations unknown
    const binding = {
        column: (name: string) => row[name],
        parameter: () => null,
        exists: () => undefined,
    };
    const decided = match(binding);
    if (decided !== undefined || relations === undefined) {
        return decided === true;
    }

    // read each relation's answer, then decide again
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

/** Read the prepared statement selecting the log's head with a table's logged columns of the rows whose text columns hold a listed tuple, once per table and columns. */
function tupleRead(table: Table, columns: readonly string[]): Statement {
    return table.statement(`tuple:${columns.join(",")}`, () => {
        // join each listed tuple to its rows, keeping a row of the head for a tuple naming none
        const logged = Object.values(table[TABLE].logged);
        const definitions = table[TABLE].columns;
        const head = sql.join(
            HEAD_COLUMNS.map((name) => sql`head.${sql.identifier(name)}`),
            sql`, `,
        );

        return new Statement(
            (value) => sql`SELECT ${head}, ${sql.join(logged, sql`, `)}
                FROM (${selectHead()}) AS head
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

/** Write a row's column value in the JSON form tuples hold it in. */
function toJson(table: Table, column: string, row: Row): unknown {
    return table[TABLE].columns[column]!.definition.toJson(row[column]);
}
