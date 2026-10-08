import {
    count,
    max,
    min,
    sql,
    sum,
    TABLE,
    type DatabaseConnection,
    type SQL,
    Order,
    type Scalar,
    type Row,
    type ColumnValue,
    DriverValue,
    headFields,
    latestOf,
} from "@destack/db";
import { schema } from "@destack/schema";
import type { Visibility, Node } from "../query/node.ts";
import type { Aggregate, Measure } from "@destack/db";

/** A selected count. */
const COUNT = schema
    .union([schema.number(), schema.bigint(), schema.string()])
    .transform(Number)
    .pipe(schema.number().int().nonnegative());

/** A selected sum as exact decimal text, null over no values. */
const SUM = schema.string().nullable();

/** A selected text value. */
const SELECTED_TEXT = schema.string();

/** A selected number, as drivers return it. */
const SELECTED_NUMBER = schema
    .union([schema.number(), schema.bigint(), schema.string()])
    .transform(Number)
    .pipe(schema.number());

/** The sum of no values. */
const EMPTY_SUM: Sum = { partials: [], count: 0 };

/** One group's measures, kept current from the log sequence of its count. */
export class Tally implements Result {
    /** The aggregate node. */
    readonly #node: Node;
    /** The group's values in JSON form, the join value first for an include. */
    readonly group: Record<string, Scalar>;
    /** The measures by name. */
    readonly #measures: Readonly<Record<string, Measure>>;
    /** The columns sums and averages add up. */
    readonly #summed: readonly string[];
    /** The rows of the group. */
    #count = 0;
    /** The sums of the added-up columns, by property. */
    readonly #sums = new Map<string, Sum>();
    /** The extreme values by measure name, null while the group has no value. */
    readonly #extremes = new Map<string, ColumnValue>();
    /** The log sequence the tally has changes up to. */
    sequence: number;
    /** The cached measures. */
    #values: Record<string, Scalar> | undefined;

    /** Start a tally from a count read at a log sequence. */
    constructor(node: Node, group: Record<string, Scalar>, measurement: Measurement) {
        // take the count's rows, sums and extremes
        const measures = aggregateOf(node).values;
        this.#node = node;
        this.group = group;
        this.#measures = measures;
        this.#summed = summedColumns(measures);
        this.sequence = measurement.sequence;
        this.#count = measurement.rows;
        for (const [column, summed] of Object.entries(measurement.sums)) {
            this.#sums.set(column, summed);
        }
        for (const [name, measure] of Object.entries(measures)) {
            if (measure.function === "min" || measure.function === "max") {
                this.#extremes.set(name, measurement.extremes[name] ?? null);
            }
        }
    }

    /** Measure an aggregate node's groups among the rows a selection matches, at one log sequence. */
    static async measure(
        database: DatabaseConnection,
        node: Node,
        selection: SQL,
        visibility?: Visibility,
        admit?: (rows: readonly Row[]) => Promise<ReadonlySet<number>>,
    ): Promise<{ readonly sequence: number; readonly tallies: Tally[] }> {
        // tally real numbers and admitted rows in memory
        const aggregate = aggregateOf(node);
        const grouping = node.grouping;
        const summed = summedColumns(aggregate.values);
        if (admit !== undefined || summed.some((name) => node.kindOf(name) === "real")) {
            return Tally.#tally(database, node, selection, visibility, admit);
        }

        // select the group columns and each measure's terms
        const extremes = Object.entries(aggregate.values).flatMap(([name, measure]) =>
            measure.function === "min" || measure.function === "max"
                ? [[name, measure] as const]
                : [],
        );
        const fields = measuredFields(node, grouping, summed, extremes, visibility);

        // read the groups by select list position beside the log's head
        const { sequence, rows } = await headed(database, (reader) =>
            reader
                .select({ group: fields, head: headFields(reader.dialect) })
                .from(node.table)
                .where(selection)
                .groupBy(...grouping.map((_, index) => sql.raw(String(index + 1)))),
        );

        return {
            sequence,
            tallies: rows.map((row) =>
                measuredTally(node, row, sequence, grouping, summed, extremes),
            ),
        };
    }

    /** Tally the matched and admitted rows by group in memory, at one log sequence. */
    static async #tally(
        database: DatabaseConnection,
        node: Node,
        selection: SQL,
        visibility: Visibility | undefined,
        admit: ((rows: readonly Row[]) => Promise<ReadonlySet<number>>) | undefined,
    ): Promise<{ readonly sequence: number; readonly tallies: Tally[] }> {
        // select the measured and computed columns
        const measured = Object.values(aggregateOf(node).values).flatMap((measure) =>
            measure.function === "count" ? [] : [measure.column],
        );
        const names = new Set(
            [...node.grouping, ...measured].flatMap((name) => node.columnsOf(name)),
        );
        const fields = {
            ...(admit === undefined ? {} : node.table[TABLE].logged),
            ...Object.fromEntries([...names].map((name) => [name, node.column(name)])),
            ...Object.fromEntries(
                Object.keys(node.extras).map((name) => [name, valueOf(node, name, visibility)]),
            ),
        };

        // read them beside the log's head
        const read = await headed(database, (reader) =>
            reader
                .select({ group: fields, head: headFields(reader.dialect) })
                .from(node.table)
                .where(selection),
        );
        const sequence = read.sequence;
        const rows = read.rows.map((row) => rowOf(row));

        // tally each admitted row in its group
        const admitted = admit === undefined ? undefined : await admit(rows);
        const tallies = new Map<string, Tally>();
        for (const [index, row] of rows.entries()) {
            if (admitted !== undefined && !admitted.has(index)) {
                continue;
            }
            const group = node.groupOf(row);
            const key = JSON.stringify(group);
            const tally = tallies.get(key) ?? Tally.empty(node, group, sequence);
            tallies.set(key, tally);
            tally.count(row, 1);
        }

        return { sequence, tallies: [...tallies.values()] };
    }

    /** Start an empty tally at a log sequence. */
    static empty(node: Node, group: Record<string, Scalar>, sequence: number): Tally {
        return new Tally(node, group, { sequence, rows: 0, sums: {}, extremes: {} });
    }

    /** The rows of the group. */
    get rows(): number {
        return this.#count;
    }

    /** Whether the group has no rows. */
    get isEmpty(): boolean {
        return this.#count === 0;
    }

    /** Count a row in or out, returning false when counting out took an extreme. */
    count(row: Row, sign: 1 | -1): boolean {
        // clear the cache and add up the row
        this.#values = undefined;
        this.#count += sign;
        this.#add(row, sign);

        // keep or lose each extreme
        let isExact = true;
        for (const [name, measure] of Object.entries(this.#measures)) {
            if (measure.function !== "min" && measure.function !== "max") {
                continue;
            }
            const value = row[measure.column] ?? null;
            if (value === null) {
                continue;
            }
            const extreme = this.#extremes.get(name) ?? null;
            const order = extreme === null ? undefined : Order.values(value, extreme);
            if (sign < 0) {
                isExact &&= order !== 0;
            } else if (
                order === undefined ||
                (measure.function === "min" ? order < 0 : order > 0)
            ) {
                this.#extremes.set(name, value);
            }
        }

        return isExact;
    }

    /** Read the measures in JSON form. */
    values(): Record<string, Scalar> {
        this.#values ??= Object.fromEntries(
            Object.entries(this.#measures).map(([name, measure]) => [
                name,
                this.#value(name, measure),
            ]),
        );

        return this.#values;
    }

    /** Read each average's sum and count of present values. */
    averages(): Record<string, Average> {
        return Object.fromEntries(
            Object.entries(this.#measures).flatMap(([name, measure]) => {
                if (measure.function !== "avg") {
                    return [];
                }
                const summed = this.#sums.get(measure.column) ?? EMPTY_SUM;

                return [
                    [
                        name,
                        {
                            sum: this.#node.scalar(measure.column, total(summed)),
                            count: summed.count,
                        },
                    ],
                ];
            }),
        );
    }

    /** Read one measure in JSON form. */
    #value(name: string, measure: Measure): Scalar {
        // count rows
        if (measure.function === "count") {
            return this.#count;
        }

        // add up a column, averaging over present values
        if (measure.function === "sum" || measure.function === "avg") {
            const summed = this.#sums.get(measure.column) ?? EMPTY_SUM;
            if (summed.count === 0) {
                return null;
            }

            return measure.function === "avg"
                ? Number(total(summed)) / summed.count
                : this.#node.scalar(measure.column, total(summed));
        }

        // write an extreme in JSON form
        return this.#node.scalar(measure.column, this.#extremes.get(name) ?? null);
    }

    /** Add or subtract a row's summed columns. */
    #add(row: Row, sign: 1 | -1): void {
        for (const column of this.#summed) {
            // add an exact integer or a number to its kind of sum
            const value = row[column];
            const entry = this.#sums.get(column);
            if (value === null || value === undefined) {
                continue;
            } else if (typeof value === "bigint" && (entry === undefined || "exact" in entry)) {
                const exact = (entry?.exact ?? 0n) + BigInt(sign) * value;
                this.#sums.set(column, { exact, count: (entry?.count ?? 0) + sign });
            } else if (typeof value === "number" && (entry === undefined || "partials" in entry)) {
                const partials = grow(entry?.partials ?? [], sign * value);
                this.#sums.set(column, { partials, count: (entry?.count ?? 0) + sign });
            } else {
                throw new TypeError(`${column} adds up values of another kind than its sum`);
            }
        }
    }
}

/** A group's measures as a page sends them. */
export interface Result {
    /** The rows of the group. */
    readonly rows: number;
    /** Whether the group has no rows. */
    readonly isEmpty: boolean;
    /** Read the measures in JSON form. */
    values(): Record<string, Scalar>;
    /** Read each average's sum and count of present values. */
    averages(): Record<string, Average>;
}

/** A group's measures as one SQL count read them. */
export interface Measurement {
    /** The log sequence the count read at. */
    readonly sequence: number;
    /** The rows of the group. */
    readonly rows: number;
    /** The sums of the added-up columns, by property. */
    readonly sums: Readonly<Record<string, Sum>>;
    /** The extreme values by measure name. */
    readonly extremes: Readonly<Record<string, ColumnValue>>;
}

/** One column's sum over a group, exact for bigints and as nonoverlapping partials for numbers, and its count of present values. */
type Sum =
    | {
          /** The exact sum of a bigint column. */
          readonly exact: bigint;
          /** The rows with a value. */
          readonly count: number;
      }
    | {
          /** The exact nonoverlapping partials of a number column's sum. */
          readonly partials: readonly number[];
          /** The rows with a value. */
          readonly count: number;
      };

/** An average's terms: the sum of present values in JSON form, and their count. */
export interface Average {
    /** The sum. */
    readonly sum: Scalar;
    /** The count of present values. */
    readonly count: number;
}

/** Select a computed value, a number as a number. */
function valueOf(node: Node, name: string, visibility: Visibility | undefined): SQL {
    const value = sql`${node.sql(name, visibility)}`;

    return node.kindOf(name) === "text"
        ? value.mapWith((selected) => SELECTED_TEXT.parse(selected))
        : value.mapWith((selected) => SELECTED_NUMBER.parse(selected));
}

/** Select an aggregate node's group columns and measure terms by select list name. */
function measuredFields(
    node: Node,
    grouping: readonly string[],
    summed: readonly string[],
    extremes: readonly (readonly [string, Extract<Measure, { readonly column: string }>])[],
    visibility: Visibility | undefined,
): Record<string, SQL> {
    return {
        ...Object.fromEntries(
            grouping.map((name, index): [string, SQL] => [
                `g${index}`,
                sql`${node.sql(name, visibility)}`,
            ]),
        ),
        rows: count(),
        ...Object.fromEntries(
            summed.flatMap((name, index): [string, SQL][] => [
                [`s${index}`, sum(node.sql(name, visibility))],
                [`c${index}`, count(node.sql(name, visibility))],
            ]),
        ),
        ...Object.fromEntries(
            extremes.map(([, measure], index): [string, SQL] => [
                `e${index}`,
                extremeOf(node, measure, visibility),
            ]),
        ),
    };
}

/** Start a tally from one measured group's selected row. */
function measuredTally(
    node: Node,
    row: Readonly<Record<string, unknown>>,
    sequence: number,
    grouping: readonly string[],
    summed: readonly string[],
    extremes: readonly (readonly [string, Measure])[],
): Tally {
    // read the group's values
    const group = node.groupOf(
        Object.fromEntries(
            grouping.map((name, index) => [name, DriverValue.parse(row[`g${index}`])]),
        ),
    );

    // read the sums as exact decimals or partials
    const sums = Object.fromEntries(
        summed.map((name, index) => {
            const text = SUM.parse(row[`s${index}`]) ?? "0";
            const present = COUNT.parse(row[`c${index}`]);

            return [
                name,
                node.kindOf(name) === "bigint"
                    ? { exact: BigInt(text), count: present }
                    : { partials: [Number(text)], count: present },
            ];
        }),
    );
    const read = Object.fromEntries(
        extremes.map(([name], index) => [name, DriverValue.parse(row[`e${index}`])]),
    );

    return new Tally(node, group, {
        sequence,
        rows: COUNT.parse(row["rows"]),
        sums,
        extremes: read,
    });
}

/** Select a measure's extreme. */
function extremeOf(
    node: Node,
    measure: Extract<Measure, { readonly column: string }>,
    visibility: Visibility | undefined,
): SQL {
    // select the measured column's extreme as a number for numeric computed values
    const operand = node.sql(measure.column, visibility);
    const extreme = measure.function === "min" ? min(operand) : max(operand);
    const computed = node.extras[measure.column];

    return computed !== undefined && node.kindOf(measure.column) !== "text"
        ? extreme.mapWith((value) => SELECTED_NUMBER.parse(value))
        : extreme;
}

/** List the distinct summed columns. */
export function summedColumns(measures: Readonly<Record<string, Measure>>): string[] {
    return [
        ...new Set(
            Object.values(measures).flatMap((measure) =>
                measure.function === "sum" || measure.function === "avg" ? [measure.column] : [],
            ),
        ),
    ];
}

/** Add a number to nonoverlapping partials exactly, as Shewchuk's expansion sum does. */
function grow(partials: readonly number[], value: number): number[] {
    // add the value through each partial
    const next: number[] = [];
    let running = value;
    for (const partial of partials) {
        const [larger, smaller] =
            Math.abs(running) < Math.abs(partial) ? [partial, running] : [running, partial];
        const high = larger + smaller;
        const low = smaller - (high - larger);
        if (low !== 0) {
            next.push(low);
        }
        running = high;
    }
    next.push(running);

    return next;
}

/** Read a sum: the exact integer, or the partials rounded to the nearest number. */
function total(summed: Sum): number | bigint {
    // add the partials from the largest down
    if ("exact" in summed) {
        return summed.exact;
    }
    const partials = summed.partials;
    let index = partials.length - 1;
    let high = partials[index] ?? 0;
    let low = 0;
    while (index > 0) {
        index -= 1;
        const next = partials[index] ?? 0;
        const rounded = high + next;
        low = next - (rounded - high);
        high = rounded;
        if (low !== 0) {
            break;
        }
    }

    // round half to even
    const below = partials[index - 1] ?? 0;
    if (index > 0 && ((low < 0 && below < 0) || (low > 0 && below > 0))) {
        const doubled = low * 2;
        const rounded = high + doubled;
        if (doubled === rounded - high) {
            high = rounded;
        }
    }

    // write an exact zero unsigned
    return high === 0 ? 0 : high;
}

/** Read an aggregate node's aggregate. */
function aggregateOf(node: Node): Aggregate {
    if (node.aggregate === undefined) {
        throw new TypeError(`${node.name} measures no rows`);
    }

    return node.aggregate;
}

/** Read a selected row's values as column values. */
function rowOf(row: Readonly<Record<string, unknown>>): Row {
    return Object.fromEntries(
        Object.entries(row).map(([name, value]) => [name, columnValueOf(value)]),
    );
}

/** Read one selected value, decoded by its column or as a driver value. */
function columnValueOf(value: unknown): ColumnValue {
    return typeof value === "object" && value !== null && !(value instanceof Uint8Array)
        ? schema.json().parse(value)
        : DriverValue.parse(value);
}

/** Read rows beside the log's head in one statement, or with the head in one read transaction when there are none. */
async function headed<Group>(
    database: DatabaseConnection,
    read: (reader: DatabaseConnection) => PromiseLike<
        readonly {
            readonly group: Group;
            readonly head: {
                readonly epoch: string;
                readonly logged: number | null;
                readonly horizon: number;
            };
        }[]
    >,
): Promise<{ readonly sequence: number; readonly rows: Group[] }> {
    // take the head of a read with rows
    const rows = await read(database);
    const [first] = rows;
    if (first !== undefined) {
        return {
            sequence: latestOf(first.head.logged, first.head.horizon),
            rows: rows.map((row) => row.group),
        };
    }

    // read the head and the rows of an empty read consistently
    return database.transaction(
        async (transaction) => ({
            sequence: (await transaction.log.position()).sequence,
            rows: (await read(transaction)).map((row) => row.group),
        }),
        { isolationLevel: "repeatable read", isReadOnly: true },
    );
}
