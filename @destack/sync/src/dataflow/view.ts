import { found, zip } from "@destack/schema";
import {
    and,
    Key,
    TABLE,
    type DatabaseConnection,
    type Row,
    type ColumnValue,
    type SQL,
    type Table,
    DatabaseError,
    CHAIN_TERMS,
    Condition,
    Predicate,
    type Scalar,
    Snapshot,
    type LogPosition,
    type WindowRead,
    type RelationView,
    type Rewind,
} from "@destack/db";
import type { Node } from "../query/node.ts";
import type { Audience } from "../feed/audience.ts";
import type { RowChange } from "../query/page.ts";
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
        this.#cache = cache ?? new ViewCache(database);
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

    /** Show a database as a snapshot reads it, or as its open transaction reads it now. */
    static async of(database: DatabaseConnection, snapshot: Snapshot | undefined): Promise<View> {
        return snapshot === undefined
            ? View.latest(database)
            : new View(
                  database,
                  snapshot.position ?? (await database.log.reached()),
                  undefined,
                  snapshot,
              );
    }

    /** Read the rows of a table matching each match's values, aligned with the matches. */
    async lookup(table: Table, matches: readonly Row[]): Promise<(readonly Row[])[]> {
        // name each complete match's read
        const [first] = matches;
        if (first === undefined) {
            return [];
        }
        const sequence = this.position.sequence;
        const columns = Object.keys(first);
        const named = matches.map((match) => {
            const tuple = tupleOf(table, columns, match);

            return tuple === undefined ? undefined : { key: rowsKey(table, columns, tuple), tuple };
        });

        // read the unread matches a batch at a time
        const missing = new Map<string, Tuple>();
        for (const entry of named) {
            if (entry !== undefined && !this.#cache.rows.has(sequence, entry.key)) {
                missing.set(entry.key, entry.tuple);
            }
        }
        const entries = [...missing];
        for (let start = 0; start < entries.length; start += BATCH_VALUES) {
            const batch = entries.slice(start, start + BATCH_VALUES);
            const grouped = this.#read(
                table,
                columns,
                batch.map(([, tuple]) => tuple),
            ).then((rows) => {
                const byKey = new Map<string, Row[]>(batch.map(([key]) => [key, []]));
                for (const row of rows) {
                    const tuple = tupleOf(table, columns, row);
                    if (tuple !== undefined) {
                        byKey.get(rowsKey(table, columns, tuple))?.push(row);
                    }
                }

                return byKey;
            });
            for (const [key] of batch) {
                void this.#cache.rows.share(sequence, key, async () => found(await grouped, key));
            }
        }

        // answer each match from its read
        return Promise.all(
            named.map((entry) =>
                entry === undefined
                    ? Promise.resolve([])
                    : this.#cache.rows.share(sequence, entry.key, () =>
                          this.#read(table, columns, [entry.tuple]),
                      ),
            ),
        );
    }

    /** Read rows of a table by key, aligned with the keys. */
    async keyed(table: Table, keys: readonly Row[]): Promise<(Row | null)[]> {
        // read a single-column key as a lookup
        const [column, ...rest] = table[TABLE].key;
        if (column !== undefined && rest.length === 0) {
            const read = await this.lookup(
                table,
                keys.map((key) => ({ [column]: Key.value(table, key, column) })),
            );

            return read.map(([row = null]) => row);
        }

        // read compound keys one at a time
        return Promise.all(
            keys.map((key) =>
                this.#cache.row.share(this.position.sequence, `row:${Key.name(table, key)}`, () =>
                    this.#snapshot.row(table, key),
                ),
            ),
        );
    }

    /** Read one row by its key. */
    async row(table: Table, key: Row): Promise<Row | null> {
        const [row = null] = await this.keyed(table, [key]);

        return row;
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
    ): Promise<(readonly Row[])[]> {
        // read fresh partitions of a key path together in ranked batches
        const read = (segment: Segment) => orderedKey(node, segment, count, audience);
        const options = orderedOptions(node, count, audience, run, relations);
        const fresh = segments.filter(
            (segment) =>
                segment.after === undefined &&
                segment.value !== null &&
                !this.#cache.rows.has(this.position.sequence, read(segment)),
        );
        const path = node.path;
        if (path?.kind === "key" && fresh.length > 1) {
            this.#shareWindows(node, path.column, fresh, options, read);
        }

        // read each partition through the shared reads
        return Promise.all(
            segments.map((segment) =>
                this.#cache.rows.share(this.position.sequence, read(segment), () =>
                    this.#snapshot.ordered(node.table, {
                        ...options,
                        where: node.condition(segment.value),
                        ...(segment.after === undefined ? {} : { after: segment.after }),
                    }),
                ),
            ),
        );
    }

    /** Share ranked reads of fresh partitions of a key path in batches. */
    #shareWindows(
        node: Node,
        column: string,
        fresh: readonly Segment[],
        options: Omit<WindowRead, "where">,
        read: (segment: Segment) => string,
    ): void {
        for (let start = 0; start < fresh.length; start += BATCH_VALUES) {
            // read one batch of partitions in one ranked read
            const batch = fresh.slice(start, start + BATCH_VALUES);
            const windows = this.#snapshot.windows(node.table, {
                ...options,
                where: node.condition(),
                partition: { column, values: batch.map((segment) => segment.value) },
            });

            // share each partition's window
            for (const [index, segment] of batch.entries()) {
                void this.#cache.rows.share(this.position.sequence, read(segment), async () => {
                    const window = (await windows)[index];
                    if (window === undefined) {
                        throw new RangeError("a ranked read lacks one of its windows");
                    }

                    return window;
                });
            }
        }
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
        const selection = Predicate.render(
            node.resolve({ AND: [node.condition(), within] }),
            node.bind(admitted),
        );
        if (current === "memory") {
            return Tally.measure(this.#database, node, selection, admitted, async (rows) =>
                audience.admits(node.table, rows, this.position),
            );
        }

        return Tally.measure(this.#database, node, and(selection, current), admitted);
    }

    /** Read a node's rows before and after their related rows changed between two sequences. */
    async dependents(node: Node, after: number, upto: number): Promise<Row[]> {
        // name the values of the changed related and join rows
        const rows = new Map<string, Row>();
        for (const relation of node.relations) {
            const path = relation.path;
            const values: (ColumnValue | undefined)[] = [];
            const changed = [
                ...(await this.#changed(relation.table, after, upto)),
                ...(await this.dependents(relation, after, upto)),
            ];
            // take a key path's column
            if (path?.kind === "key") {
                values.push(...changed.map((row) => row[path.column]));
            }
            // follow a junction path through its join rows
            else if (path?.kind === "junction") {
                const targets = changed.map((row) => row[path.to.key]);
                const joins = await this.#current(path.table, path.to.column, targets);
                values.push(...joins.map((join) => join[path.from.column]));
                for (const join of await this.#changed(path.table, after, upto)) {
                    values.push(join[path.from.column]);
                }
            }
            // refuse relations of other paths
            else {
                throw new TypeError(`relation ${relation.name} follows no key or junction path`);
            }

            // read the node's rows with each value then and now
            const column = relation.link().parentColumn;
            const present = presentValues(values);
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

    /** Read a table's rows whose columns have one of some tuples. */
    #read(table: Table, columns: readonly string[], tuples: readonly Tuple[]): Promise<Row[]> {
        // read text columns through their prepared statement
        const definition = table[TABLE];
        const [single, ...others] = columns;
        if (columns.every((column) => definition.column(column).definition.kind === "text")) {
            return this.#snapshot.select(table, columns, tuples);
        }

        // match other columns in a condition
        return this.#snapshot.rows(
            table,
            single !== undefined && others.length === 0
                ? {
                      [single]: {
                          in: tuples.flatMap(([value]) => (value === undefined ? [] : [value])),
                      },
                  }
                : { OR: tuples.map((tuple) => Object.fromEntries(zip(columns, tuple))) },
        );
    }

    /** Read the rows of a table changed between two sequences, as they were and are. */
    async #changed(table: Table, after: number, upto: number): Promise<Row[]> {
        // take each changed row's image
        const images = await this.#cache.images(table, after, upto);
        const rows = [...images.values()].filter((image): image is Row => image !== null);
        const keys = [...images.keys()].map((name) => Key.parse(table, name));

        // read the current rows by a single-column key at once or by a chain of keys at a time
        const [column, ...rest] = table[TABLE].key;
        if (column !== undefined && rest.length === 0) {
            rows.push(
                ...(await this.#current(
                    table,
                    column,
                    keys.map((key) => key[column]),
                )),
            );
        } else {
            for (let start = 0; start < keys.length; start += CHAIN_TERMS) {
                const matches = Key.any(table, keys.slice(start, start + CHAIN_TERMS));
                rows.push(
                    ...(await this.#database
                        .select()
                        .from(table)
                        .where(Condition.render(matches, table))),
                );
            }
        }

        return rows;
    }

    /** Read a table's current rows whose column has one of some values. */
    async #current(
        table: Table,
        column: string,
        values: readonly (ColumnValue | undefined)[],
    ): Promise<Row[]> {
        // match the distinct present values in their JSON form
        const definition = table[TABLE].column(column).definition;
        const present = [
            ...new Set(
                presentValues(values).map((value) => scalarOf(definition.toJson(value), column)),
            ),
        ];
        const rows: Row[] = [];
        for (let start = 0; start < present.length; start += BATCH_VALUES) {
            const matches = { [column]: { in: present.slice(start, start + BATCH_VALUES) } };
            rows.push(
                ...(await this.#database
                    .select()
                    .from(table)
                    .where(Condition.render(matches, table))),
            );
        }

        return rows;
    }
}

/** Values computed once per log sequence and key, kept for a few recent sequences. */
export class Memo<Value> {
    /** The most sequences kept. */
    readonly #sequences: number;
    /** The values by sequence and key. */
    readonly #values = new Map<number, Map<string, Value>>();

    /** Keep the values of some recent sequences. */
    constructor(sequences: number) {
        this.#sequences = sequences;
    }

    /** Compute a value once per sequence and key, forgetting a failed read. */
    share(sequence: number, key: string, compute: () => Value): Value {
        // reuse the value of the same sequence and key
        let values = this.#values.get(sequence);
        const known = values?.get(key);
        if (known !== undefined) {
            return known;
        }

        // start the sequence's values and drop the oldest beyond the kept number
        if (values === undefined) {
            values = new Map<string, Value>();
            this.#values.set(sequence, values);
            if (this.#values.size > this.#sequences) {
                this.#values.delete(Math.min(...this.#values.keys()));
            }
        }

        // compute and forget a failed read
        const value = compute();
        values.set(key, value);
        if (value instanceof Promise) {
            const shared = values;
            value.catch(() => {
                if (shared.get(key) === value) {
                    shared.delete(key);
                }
            });
        }

        return value;
    }

    /** Decide whether a value for a sequence and key is shared. */
    has(sequence: number, key: string): boolean {
        return this.#values.get(sequence)?.has(key) === true;
    }

    /** Forget every value. */
    clear(): void {
        this.#values.clear();
    }
}

/** The shared reads and row images of views. */
export interface Cache {
    /** The shared reads of rows: lookups and ordered windows. */
    readonly rows: Memo<Promise<readonly Row[]>>;
    /** The shared reads of single rows by compound key. */
    readonly row: Memo<Promise<Row | null>>;
    /** The shared encodings of selected rows. */
    readonly encodings: Memo<RowChange>;
    /** Read the images a table's rows had before their first change between two sequences, by key. */
    images(table: Table, after: number, upto: number): Promise<ReadonlyMap<string, Row | null>>;
}

/** One partition an ordered read reads. */
export interface Segment {
    /** The parent value naming the partition, null for a root. */
    readonly value: ColumnValue;
    /** The row the read continues after. */
    readonly after?: Row;
}

/** A view's own shared reads, with images from the log. */
class ViewCache implements Cache {
    /** The shared reads of rows. */
    readonly rows = new Memo<Promise<readonly Row[]>>(1);
    /** The shared reads of single rows. */
    readonly row = new Memo<Promise<Row | null>>(1);
    /** The shared encodings. */
    readonly encodings = new Memo<RowChange>(1);
    /** The images of the log's changes, read once. */
    readonly #rewind: Rewind;

    /** Create the memory of a database's views. */
    constructor(database: DatabaseConnection) {
        this.#rewind = database.log.rewind();
    }

    /** Read images from the log. */
    images(table: Table, after: number, upto: number): Promise<ReadonlyMap<string, Row | null>> {
        return this.#rewind(table, after, upto);
    }
}

/** The JSON forms of a row's present values in some columns. */
type Tuple = Exclude<Scalar, null>[];

/** Write a row's values in some columns in JSON form, absent when one is missing. */
function tupleOf(table: Table, columns: readonly string[], row: Row): Tuple | undefined {
    const tuple: Tuple = [];
    for (const column of columns) {
        const value = row[column];
        if (value === null || value === undefined) {
            return undefined;
        }
        tuple.push(scalarOf(table[TABLE].column(column).definition.toJson(value), column));
    }

    return tuple;
}

/** Require a column's JSON form to be a present scalar, as keys and joins are. */
function scalarOf(json: unknown, column: string): Exclude<Scalar, null> {
    if (typeof json !== "string" && typeof json !== "number" && typeof json !== "boolean") {
        throw new TypeError(`${column} has no scalar value to read by`);
    }

    return json;
}

/** Keep the present values. */
function presentValues(values: readonly (ColumnValue | undefined)[]): Exclude<ColumnValue, null>[] {
    return values.filter(
        (value): value is Exclude<ColumnValue, null> => value !== null && value !== undefined,
    );
}

/** Name the shared read of a tuple. */
function rowsKey(table: Table, columns: readonly string[], tuple: Tuple): string {
    return `rows:${table[TABLE].sqlName}.${columns.join(",")}=${JSON.stringify(tuple)}`;
}

/** Build the key of one partition's shared ordered read. */
function orderedKey(node: Node, segment: Segment, count: number, audience: Audience): string {
    const after = segment.after === undefined ? "" : Key.name(node.table, segment.after);

    return `ordered:${node.selection}:${node.partition(segment.value)}:${after}:${count}:${audience.key}`;
}

/** Build the options every ordered read of a node's partitions shares. */
function orderedOptions(
    node: Node,
    count: number,
    audience: Audience,
    run: Run,
    relations: RelationView | undefined,
): Omit<WindowRead, "where"> {
    return {
        orderBy: node.orderBy,
        namespace: node.namespace((table: Table) => admittedSQL(audience, table)),
        limit: count,
        admits: {
            ...(audience.where(node.table) === "memory"
                ? {}
                : { current: admittedSQL(audience, node.table) }),
            image: async (row: Row) => (await run.seen(node.table, [row])).length > 0,
        },
        ...(relations === undefined ? {} : { relations }),
    };
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
