import {
    and,
    Key,
    TABLE,
    type DatabaseConnection,
    type Row,
    type SQL,
    type Table,
} from "@destack/db";
import { DatabaseError } from "@destack/db/error";
import { CHAIN_TERMS, Condition, type Scalar } from "@destack/db/query";
import type { LogPosition, RelationView, Rewind, Snapshot } from "@destack/db/log";
import type { Node } from "../query/node.ts";
import type { Audience } from "../feed/audience.ts";
import type { Run } from "./run.ts";
import { Tally } from "./tally.ts";

/** The values one batched read names at most, an OR chain well within the parameter budget and a condition's terms. */
const BATCH_VALUES = 500;

/**
 * The database as of a log position, with every read batched and shared among the readers deciding that position.
 *
 * Rows changed since the position read as their earlier images.
 */
export class View {
    /** The database read. */
    readonly #database: DatabaseConnection;
    /** Where reads are shared and images found. */
    readonly #cache: Cache;
    /** The database as of the position. */
    readonly #snapshot: Snapshot;
    /** The position shown. */
    readonly position: LogPosition;

    /** Show a database as of a position, sharing reads through a feed or the view's own memory. */
    constructor(database: DatabaseConnection, position: LogPosition, cache?: Cache) {
        // read earlier images through the cache
        this.#database = database;
        this.position = position;
        this.#cache = cache ?? new Memory(database);
        this.#snapshot = database.log.at(position, (table, after, upto) =>
            this.#cache.images(table, after, upto),
        );
    }

    /** Show a database as of its latest position, sharing reads in the view's own memory. */
    static async latest(database: DatabaseConnection): Promise<View> {
        return new View(database, await database.log.position());
    }

    /**
     * Read the rows of a table whose columns hold each match's values, aligned with the matches, which name the same columns.
     *
     * Matches no reader read yet are read a batch at a time; a match with a missing value holds no rows.
     */
    async lookup(table: Table, matches: readonly Row[]): Promise<(readonly Row[])[]> {
        // name each complete match's read
        if (matches.length === 0) {
            return [];
        }
        const sequence = this.position.sequence;
        const columns = Object.keys(matches[0]!);
        const tuples = matches.map((match) =>
            columns.every((column) => match[column] !== null && match[column] !== undefined)
                ? tupleOf(table, columns, match)
                : undefined,
        );
        const keys = tuples.map((tuple) =>
            tuple === undefined ? undefined : rowsKey(table, columns, tuple),
        );

        // read the matches no reader read yet, a batch at a time, grouping each batch's rows by match
        const missing = new Map<string, Tuple>();
        for (const [index, key] of keys.entries()) {
            if (key !== undefined && !this.#cache.isShared(sequence, key)) {
                missing.set(key, tuples[index]!);
            }
        }
        const entries = [...missing];
        const shared = new Map<string, Promise<readonly Row[]>>();
        for (let start = 0; start < entries.length; start += BATCH_VALUES) {
            const batch = entries.slice(start, start + BATCH_VALUES);
            const grouped = this.#read(
                table,
                columns,
                batch.map(([, tuple]) => tuple),
            ).then((rows) => {
                const byKey = new Map<string, Row[]>(batch.map(([key]) => [key, []]));
                for (const row of rows) {
                    byKey.get(rowsKey(table, columns, tupleOf(table, columns, row)))?.push(row);
                }

                return byKey;
            });
            for (const [key] of batch) {
                shared.set(
                    key,
                    this.#cache.share(sequence, key, async () => (await grouped).get(key)!),
                );
            }
        }

        // answer each match from its batch or an earlier reader's read, reading one alone after a failed read
        return Promise.all(
            keys.map((key, index) =>
                key === undefined
                    ? Promise.resolve([])
                    : (shared.get(key) ??
                      this.#cache.share(sequence, key, () =>
                          this.#read(table, columns, [tuples[index]!]),
                      )),
            ),
        );
    }

    /** Read rows of a table by their keys, aligned with the keys, absent where no row held a key at the position. */
    async keyed(table: Table, keys: readonly Row[]): Promise<(Row | undefined)[]> {
        // read a table keyed by one column as the rows holding each value
        const [column, ...rest] = table[TABLE].key;
        if (rest.length === 0) {
            const read = await this.lookup(
                table,
                keys.map((key) => ({ [column!]: key[column!] })),
            );

            return read.map((rows) => rows[0]);
        }

        // read a compound key's rows one key at a time, shared per position
        return Promise.all(
            keys.map((key) =>
                this.#cache.share(this.position.sequence, `row:${Key.name(table, key)}`, () =>
                    this.#snapshot.row(table, key),
                ),
            ),
        );
    }

    /** Read one row by its key, absent when it did not exist at the position. */
    async row(table: Table, key: Row): Promise<Row | undefined> {
        return (await this.keyed(table, [key]))[0];
    }

    /** Read a table's rows a condition over its columns matches. */
    matching(table: Table, where: Condition): Promise<Row[]> {
        return this.#snapshot.rows(table, where);
    }

    /**
     * Read the first rows of a node's partitions an audience sees, in the node's order after each segment's row, up to a count each.
     *
     * Each partition reads through its own index-ordered statement, all at once; audiences that decide alike share each read.
     * The run decides the audience's view of rows changed since the position.
     */
    ordered(
        node: Node,
        segments: readonly Segment[],
        count: number,
        audience: Audience,
        run: Run,
        relations: RelationView | undefined,
    ): Promise<Row[][]> {
        return Promise.all(
            segments.map((segment) => {
                // name the read, which audiences deciding alike share, and admit its rows in SQL or in memory
                const after =
                    segment.after === undefined ? "" : Key.name(node.table, segment.after);
                const key = `ordered:${node.selection}:${node.partition(segment.value)}:${after}:${count}:${audience.key}`;
                const current = audience.where(node.table);

                return this.#cache.share(this.position.sequence, key, () =>
                    this.#snapshot.ordered(node.table, {
                        where: node.condition(segment.value),
                        order: node.order,
                        namespace: node.namespace((table) => admittedSQL(audience, table)),
                        ...(segment.after === undefined ? {} : { after: segment.after }),
                        count,
                        admits: {
                            ...(current === "memory" ? {} : { current }),
                            image: async (row) => (await run.seen(node.table, [row])).length > 0,
                        },
                        ...(relations === undefined ? {} : { relations }),
                    }),
                );
            }),
        );
    }

    /**
     * Measure an aggregate node's groups among its rows a condition over columns selects that an audience sees, in one grouped read.
     *
     * The read counts the rows as of the log sequence it returns, at or after the view's position.
     */
    measure(
        node: Node,
        within: Condition,
        audience: Audience,
    ): Promise<{ readonly sequence: number; readonly tallies: Tally[] }> {
        // tally the rows the audience admits in memory when it decides none in SQL
        const admitted = (table: Table) => admittedSQL(audience, table);
        const current = audience.where(node.table);
        const selection = Condition.render(
            Condition.all(node.condition(), within),
            node.bind(admitted),
        );
        if (current === "memory") {
            return Tally.measure(this.#database, node, selection, admitted, async (rows) =>
                audience.admits(node.table, rows, this.position),
            );
        }

        return Tally.measure(this.#database, node, and(selection, current)!, admitted);
    }

    /**
     * Read a node's rows whose related rows changed after one sequence, up to another, as they were and are: a superset a snapshot decides again.
     *
     * A relation's own relations reach through its rows whose related rows changed.
     */
    async dependents(node: Node, after: number, upto: number): Promise<Row[]> {
        // name the held values of the related rows and join rows that changed, as they were and are
        const rows = new Map<string, Row>();
        for (const relation of node.relations) {
            const path = relation.path!;
            const values: unknown[] = [];
            const changed = [
                ...(await this.#changed(relation.table, after, upto)),
                ...(await this.dependents(relation, after, upto)),
            ];
            // take a key path's column
            if (path.kind === "key") {
                values.push(...changed.map((row) => row[path.column]));
            }
            // follow a junction path through its current and changed join rows
            else if (path.kind === "junction") {
                const targets = changed.map((row) => row[path.to.key]);
                const joins = await this.#current(path.table, path.to.column, targets);
                values.push(...joins.map((join) => join[path.from.column]));
                for (const join of await this.#changed(path.table, after, upto)) {
                    values.push(join[path.from.column]);
                }
            }

            // read the node's rows holding each value, as of the position and now
            const column = relation.parentColumn!;
            const present = values.filter((entry) => entry !== null && entry !== undefined);
            const read = await this.lookup(
                node.table,
                present.map((value) => ({ [column]: value })),
            );
            for (const row of [
                ...read.flat(),
                ...(await this.#current(node.table, column, present)),
            ]) {
                rows.set(node.keyOf(row), row);
            }
        }

        return [...rows.values()];
    }

    /** Read a table's rows whose columns hold one of some tuples of values: through a prepared statement for text columns, else through a condition. */
    #read(table: Table, columns: readonly string[], tuples: readonly Tuple[]): Promise<Row[]> {
        // match the rows holding a listed tuple in a condition over non-text columns
        const definitions = table[TABLE].columns;
        if (columns.some((column) => definitions[column]!.definition.kind !== "text")) {
            return this.#snapshot.rows(
                table,
                columns.length === 1
                    ? Condition.oneOf(
                          columns[0]!,
                          tuples.map((tuple) => tuple[0]!),
                      )
                    : Condition.any(
                          ...tuples.map((tuple) =>
                              Condition.all(
                                  ...columns.map((column, index) =>
                                      Condition.eq(column, tuple[index]!),
                                  ),
                              ),
                          ),
                      ),
            );
        }

        // read them through the columns' prepared statement
        return this.#snapshot.select(table, columns, tuples);
    }

    /** Read the rows of a table changed after one sequence, up to another, as they were then and are now. */
    async #changed(table: Table, after: number, upto: number): Promise<Row[]> {
        // take each changed row's image, then read the rows as they are now, a chain of keys per read
        const images = await this.#cache.images(table, after, upto);
        const rows = [...images.values()].filter((image): image is Row => image !== null);
        const keys = [...images.keys()].map((name) => Key.parse(table, name));
        for (let start = 0; start < keys.length; start += CHAIN_TERMS) {
            const matches = Key.any(table, keys.slice(start, start + CHAIN_TERMS));
            const now = await this.#database
                .select()
                .from(table)
                .where(Condition.render(matches, Condition.bind(table)));
            rows.push(...(now as Row[]));
        }

        return rows;
    }

    /** Read a table's current rows whose column holds one of some values, a chain of values per read. */
    async #current(table: Table, column: string, values: readonly unknown[]): Promise<Row[]> {
        // match the distinct present values by their JSON forms
        const definition = table[TABLE].columns[column]!.definition;
        const present = [
            ...new Set(values.filter((value) => value !== null && value !== undefined)),
        ].map((value) => definition.toJson(value) as Exclude<Scalar, null>);
        const rows: Row[] = [];
        for (let start = 0; start < present.length; start += CHAIN_TERMS) {
            const matches = Condition.oneOf(column, present.slice(start, start + CHAIN_TERMS));
            const current = await this.#database
                .select()
                .from(table)
                .where(Condition.render(matches, Condition.bind(table)));
            rows.push(...(current as Row[]));
        }

        return rows;
    }
}

/** Where views share their reads per log sequence and find the images of changed rows: a feed for its subscribers, or a view's own. */
export interface Cache {
    /** Compute once per log sequence what a key names: a read or an encoding. */
    share<Value>(sequence: number, key: string, compute: () => Value): Value;
    /** Decide whether a read for a sequence is shared already. */
    isShared(sequence: number, key: string): boolean;
    /** Read the images a table's rows had before their first change after one sequence, up to another, by key. */
    images(table: Table, after: number, upto: number): Promise<ReadonlyMap<string, Row | null>>;
}

/** One partition an ordered read reads: its held value, absent for a root, and the row to read after. */
export interface Segment {
    /** The held value naming the partition, absent for a root's one partition. */
    readonly value?: unknown;
    /** The row the read continues after, absent from the start. */
    readonly after?: Row;
}

/** A view's own reads: shared within the view, with images from the log. */
class Memory implements Cache {
    /** The images of the log's changes, read once. */
    readonly #rewind: Rewind;
    /** The reads made, by key. */
    readonly #reads = new Map<string, unknown>();

    /** Share reads of one database's views. */
    constructor(database: DatabaseConnection) {
        this.#rewind = database.log.rewind();
    }

    /** Compute what a key names once. */
    share<Value>(_sequence: number, key: string, compute: () => Value): Value {
        if (!this.#reads.has(key)) {
            this.#reads.set(key, compute());
        }

        return this.#reads.get(key) as Value;
    }

    /** Decide whether a key was read. */
    isShared(_sequence: number, key: string): boolean {
        return this.#reads.has(key);
    }

    /** Read the images from the log through the views' shared memory of its changes. */
    images(table: Table, after: number, upto: number): Promise<ReadonlyMap<string, Row | null>> {
        return this.#rewind(table, after, upto);
    }
}

/** The JSON forms of a row's values in some columns, which conditions compare. */
type Tuple = (string | number | boolean)[];

/** Write a row's values in some columns in the JSON form conditions compare. */
function tupleOf(table: Table, columns: readonly string[], row: Row): Tuple {
    const definitions = table[TABLE].columns;

    return columns.map(
        (column) =>
            definitions[column]!.definition.toJson(row[column]) as string | number | boolean,
    );
}

/** Name the shared read of a table's rows whose columns hold a tuple of values. */
function rowsKey(table: Table, columns: readonly string[], tuple: Tuple): string {
    return `rows:${table[TABLE].sqlName}.${columns.join(",")}=${JSON.stringify(tuple)}`;
}

/** Match the rows of a related table an audience admits in SQL, refusing tables it decides in memory, which no relation reads. */
function admittedSQL(audience: Audience, table: Table): SQL {
    const where = audience.where(table);
    if (where === "memory") {
        throw new DatabaseError(
            "INVALID_QUERY",
            `relations read no rows of ${table[TABLE].name}, which the audience decides in memory`,
        );
    }

    return where;
}
