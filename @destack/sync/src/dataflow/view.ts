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
import { Snapshot, type LogPosition, type RelationView, type Rewind } from "@destack/db/log";
import type { Node } from "../query/node.ts";
import type { Audience } from "../feed/audience.ts";
import type { Run } from "./run.ts";
import { Tally } from "./tally.ts";

/** The most values one batched read names, bounded by the parameter budget. */
const BATCH_VALUES = 500;

/** The database as of a log position, with batched reads shared among its readers. */
export class View {
    /** The database read. */
    readonly #database: DatabaseConnection;
    /** The shared reads and images. */
    readonly #cache: Cache;
    /** The database as of the position. */
    readonly #snapshot: Snapshot;
    /** The position shown. */
    readonly position: LogPosition;

    /** Show a database as of a position. */
    constructor(
        database: DatabaseConnection,
        position: LogPosition,
        cache?: Cache,
        snapshot?: Snapshot,
    ) {
        // read earlier images through the cache
        this.#database = database;
        this.position = position;
        this.#cache = cache ?? new Memory(database);
        this.#snapshot =
            snapshot ??
            database.log.at(position, (table, after, upto) =>
                this.#cache.images(table, after, upto),
            );
    }

    /** Show a database as its open transaction reads it now. */
    static async latest(database: DatabaseConnection): Promise<View> {
        return new View(database, await database.log.reached(), undefined, Snapshot.live(database));
    }

    /** Read the rows of a table matching each match's values, aligned with the matches. */
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

        // read the unread matches a batch at a time
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

        // answer each match from its read
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

    /** Read rows of a table by key, aligned with the keys. */
    async keyed(table: Table, keys: readonly Row[]): Promise<(Row | undefined)[]> {
        // read a single-column key as a lookup
        const [column, ...rest] = table[TABLE].key;
        if (rest.length === 0) {
            const read = await this.lookup(
                table,
                keys.map((key) => ({ [column!]: key[column!] })),
            );

            return read.map((rows) => rows[0]);
        }

        // read compound keys one at a time
        return Promise.all(
            keys.map((key) =>
                this.#cache.share(this.position.sequence, `row:${Key.name(table, key)}`, () =>
                    this.#snapshot.row(table, key),
                ),
            ),
        );
    }

    /** Read one row by its key. */
    async row(table: Table, key: Row): Promise<Row | undefined> {
        return (await this.keyed(table, [key]))[0];
    }

    /** Read a table's rows matching a condition. */
    matching(table: Table, where: Condition): Promise<Row[]> {
        return this.#snapshot.rows(table, where);
    }

    /** Read the first visible rows of a node's partitions in order after each segment's row, up to a count each. */
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
                // name the shared read and admit its rows in SQL or in memory
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

    /** Measure an aggregate node's visible groups among the rows a condition selects, in one grouped read. */
    measure(
        node: Node,
        within: Condition,
        audience: Audience,
    ): Promise<{ readonly sequence: number; readonly tallies: Tally[] }> {
        // admit rows in memory when the audience decides none in SQL
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

    /** Read a node's rows whose related rows changed between two sequences, as they were and are. */
    async dependents(node: Node, after: number, upto: number): Promise<Row[]> {
        // name the values of the changed related and join rows
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
            // follow a junction path through its join rows
            else if (path.kind === "junction") {
                const targets = changed.map((row) => row[path.to.key]);
                const joins = await this.#current(path.table, path.to.column, targets);
                values.push(...joins.map((join) => join[path.from.column]));
                for (const join of await this.#changed(path.table, after, upto)) {
                    values.push(join[path.from.column]);
                }
            }

            // read the node's rows holding each value, then and now
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

    /** Read a table's rows whose columns hold one of some tuples. */
    #read(table: Table, columns: readonly string[], tuples: readonly Tuple[]): Promise<Row[]> {
        // match non-text columns in a condition
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

        // read through the columns' prepared statement
        return this.#snapshot.select(table, columns, tuples);
    }

    /** Read the rows of a table changed between two sequences, as they were and are. */
    async #changed(table: Table, after: number, upto: number): Promise<Row[]> {
        // take each changed row's image, then read the current rows
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

    /** Read a table's current rows whose column holds one of some values. */
    async #current(table: Table, column: string, values: readonly unknown[]): Promise<Row[]> {
        // match the distinct present values
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

/** The shared reads and row images of views. */
export interface Cache {
    /** Compute a keyed read or encoding once per log sequence. */
    share<Value>(sequence: number, key: string, compute: () => Value): Value;
    /** Decide whether a read for a sequence is shared. */
    isShared(sequence: number, key: string): boolean;
    /** Read the images a table's rows had before their first change between two sequences, by key. */
    images(table: Table, after: number, upto: number): Promise<ReadonlyMap<string, Row | null>>;
}

/** One partition an ordered read reads. */
export interface Segment {
    /** The held value naming the partition, absent for a root. */
    readonly value?: unknown;
    /** The row the read continues after. */
    readonly after?: Row;
}

/** A view's own shared reads, with images from the log. */
class Memory implements Cache {
    /** The images of the log's changes, read once. */
    readonly #rewind: Rewind;
    /** The reads made, by key. */
    readonly #reads = new Map<string, unknown>();

    /** Create the memory of a database's views. */
    constructor(database: DatabaseConnection) {
        this.#rewind = database.log.rewind();
    }

    /** Compute a keyed value once. */
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

    /** Read images from the log. */
    images(table: Table, after: number, upto: number): Promise<ReadonlyMap<string, Row | null>> {
        return this.#rewind(table, after, upto);
    }
}

/** The JSON forms of a row's values in some columns. */
type Tuple = (string | number | boolean)[];

/** Write a row's values in some columns in JSON form. */
function tupleOf(table: Table, columns: readonly string[], row: Row): Tuple {
    const definitions = table[TABLE].columns;

    return columns.map(
        (column) =>
            definitions[column]!.definition.toJson(row[column]) as string | number | boolean,
    );
}

/** Name the shared read of a tuple. */
function rowsKey(table: Table, columns: readonly string[], tuple: Tuple): string {
    return `rows:${table[TABLE].sqlName}.${columns.join(",")}=${JSON.stringify(tuple)}`;
}

/** Match the rows of a related table an audience admits in SQL. */
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
