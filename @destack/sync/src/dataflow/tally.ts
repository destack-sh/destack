import { count, max, min, sql, sum, type DatabaseConnection, type SQL } from "@destack/db";
import { Order, type Scalar } from "@destack/db/query";
import type { Visibility, Node } from "../query/node.ts";
import type { Measure } from "../query/query.ts";
import type { Row } from "@destack/db";

/** One group's measures, kept current row by row from the log sequence its last count read. */
export class Tally implements Result {
    /** The aggregate node whose rows the tally measures. */
    readonly #node: Node;
    /** The group's values in their JSON form, the held row's join value first for an include. */
    readonly group: Record<string, Scalar>;
    /** The measures by name. */
    readonly #measures: Readonly<Record<string, Measure>>;
    /** The columns sums and averages add up. */
    readonly #summed: readonly string[];
    /** The rows the group holds. */
    #count = 0;
    /** The sums of the added-up columns, by property. */
    readonly #sums = new Map<string, Sum>();
    /** The extreme measures' values, by measure name, null while the group holds no value. */
    readonly #extremes = new Map<string, unknown>();
    /** The log sequence the tally holds changes up to; it counts later changes only. */
    sequence: number;
    /** The measures as last read, until a row counts in or out. */
    #values: Record<string, Scalar> | undefined;

    /** Start a tally of a node's group at a log sequence, from a count read at that sequence. */
    constructor(node: Node, group: Record<string, Scalar>, count: Measurement) {
        // take the count's rows, sums and extremes at its sequence
        const measures = node.aggregate!.values;
        this.#node = node;
        this.group = group;
        this.#measures = measures;
        this.#summed = summedColumns(measures);
        this.sequence = count.sequence;
        this.#count = count.rows;
        for (const [column, sum] of Object.entries(count.sums)) {
            this.#sums.set(column, sum);
        }
        for (const [name, measure] of Object.entries(measures)) {
            if (measure.function === "min" || measure.function === "max") {
                this.#extremes.set(name, count.extremes[name] ?? null);
            }
        }
    }

    /** Read the measures of an aggregate node's groups among the rows a selection matches, at one log sequence. */
    static async measure(
        database: DatabaseConnection,
        node: Node,
        selection: SQL,
        visibility?: Visibility,
    ): Promise<{ readonly sequence: number; readonly tallies: Tally[] }> {
        // tally real numbers from their values, since SQL sums them in an order of its own
        const aggregate = node.aggregate!;
        const grouping = node.grouping;
        const summed = summedColumns(aggregate.values);
        if (summed.some((name) => node.kindOf(name) === "real")) {
            return Tally.#tally(database, node, selection, visibility);
        }

        // select the group columns and every measure's parts
        const extremes = Object.entries(aggregate.values).filter(
            ([, measure]) => measure.function === "min" || measure.function === "max",
        );
        const fields = {
            ...Object.fromEntries(
                grouping.map((name, index) => [`g${index}`, node.sql(name, visibility)]),
            ),
            rows: count(),
            ...Object.fromEntries(
                summed.flatMap((name, index) => [
                    [`s${index}`, sum(node.sql(name, visibility))],
                    [`c${index}`, count(node.sql(name, visibility))],
                ]),
            ),
            ...Object.fromEntries(
                extremes.map(([, measure], index) => [
                    `e${index}`,
                    extremeOf(node, measure, visibility),
                ]),
            ),
        };

        // read them grouped by select list position, which names a computed group alike in both dialects, in one read transaction with its sequence
        const { sequence, rows } = await database.transaction(
            async (transaction) => ({
                sequence: await transaction.log.latest(),
                rows: (await transaction
                    .select(fields as never)
                    .from(node.table)
                    .where(selection)
                    .groupBy(...grouping.map((_, index) => sql.raw(String(index + 1))))) as Record<
                    string,
                    unknown
                >[],
            }),
            { isolationLevel: "repeatable read", isReadOnly: true },
        );

        return {
            sequence,
            tallies: rows.map(
                (row) =>
                    new Tally(
                        node,
                        node.groupOf(
                            Object.fromEntries(
                                grouping.map((name, index) => [name, row[`g${index}`]]),
                            ),
                        ),
                        {
                            sequence,
                            rows: Number(row.rows),
                            sums: Object.fromEntries(
                                summed.map((name, index) => [
                                    name,
                                    {
                                        sum:
                                            node.kindOf(name) === "bigint"
                                                ? BigInt(
                                                      (row[`s${index}`] as
                                                          | string
                                                          | number
                                                          | null) ?? 0,
                                                  )
                                                : [Number(row[`s${index}`] ?? 0)],
                                        count: Number(row[`c${index}`]),
                                    },
                                ]),
                            ),
                            extremes: Object.fromEntries(
                                extremes.map(([name], index) => [name, row[`e${index}`] ?? null]),
                            ),
                        },
                    ),
            ),
        };
    }

    /** Read the measured columns of the rows a selection matches, at one log sequence, and tally them by group in memory. */
    static async #tally(
        database: DatabaseConnection,
        node: Node,
        selection: SQL,
        visibility: Visibility | undefined,
    ): Promise<{ readonly sequence: number; readonly tallies: Tally[] }> {
        // select the columns the groups and measures read, and every computed value
        const measured = Object.values(node.aggregate!.values).flatMap((measure) =>
            measure.column === undefined ? [] : [measure.column],
        );
        const names = new Set(
            [...node.grouping, ...measured].flatMap((name) => node.columnsOf(name)),
        );
        const fields = {
            ...Object.fromEntries([...names].map((name) => [name, node.columns[name]!])),
            ...Object.fromEntries(
                Object.keys(node.computed).map((name) => [name, valueOf(node, name, visibility)]),
            ),
        };

        // read them in one read transaction with its sequence
        const { sequence, rows } = await database.transaction(
            async (transaction) => ({
                sequence: await transaction.log.latest(),
                rows: (await transaction.select(fields).from(node.table).where(selection)) as Row[],
            }),
            { isolationLevel: "repeatable read", isReadOnly: true },
        );

        // tally each row in its group
        const tallies = new Map<string, Tally>();
        for (const row of rows) {
            const group = node.groupOf(row);
            const key = JSON.stringify(group);
            const tally = tallies.get(key) ?? Tally.empty(node, group, sequence);
            tallies.set(key, tally);
            tally.count(row, 1);
        }

        return { sequence, tallies: [...tallies.values()] };
    }

    /** Start an empty tally of a node's group as of a log sequence. */
    static empty(node: Node, group: Record<string, Scalar>, sequence: number): Tally {
        return new Tally(node, group, { sequence, rows: 0, sums: {}, extremes: {} });
    }

    /** The rows the group holds. */
    get rows(): number {
        return this.#count;
    }

    /** Whether the group holds no rows. */
    get isEmpty(): boolean {
        return this.#count === 0;
    }

    /**
     * Measurement a row holding its computed values in or out, returning false when counting out took an extreme, which only a recount restores.
     *
     * Counting in keeps each extreme the row passes.
     */
    count(row: Row, sign: 1 | -1): boolean {
        // forget the measures read before, and count the row with its added-up values
        this.#values = undefined;
        this.#count += sign;
        this.#add(row, sign);

        // keep or lose each extreme the row holds
        let isExact = true;
        for (const [name, measure] of Object.entries(this.#measures)) {
            const value = measure.column === undefined ? null : (row[measure.column] ?? null);
            if (value === null || (measure.function !== "min" && measure.function !== "max")) {
                continue;
            }
            const extreme = this.#extremes.get(name);
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

    /** Read the measures in their JSON form, once until a row counts in or out. */
    values(): Record<string, Scalar> {
        this.#values ??= Object.fromEntries(
            Object.entries(this.#measures).map(([name, measure]) => [
                name,
                this.#value(name, measure),
            ]),
        );

        return this.#values;
    }

    /** Read each average's sum and count of present values, which let a copy add predictions to it. */
    parts(): Record<string, Part> {
        return Object.fromEntries(
            Object.entries(this.#measures)
                .filter(([, measure]) => measure.function === "avg")
                .map(([name, measure]) => {
                    const { sum, count } = this.#sums.get(measure.column!) ?? { sum: [], count: 0 };

                    return [
                        name,
                        { sum: this.#node.json(measure.column!, total(sum)) as Scalar, count },
                    ];
                }),
        );
    }

    /** Read one measure in its JSON form. */
    #value(name: string, measure: Measure): Scalar {
        // count rows
        if (measure.function === "count") {
            return this.#count;
        }

        // add up a column, averaging over the rows holding a value
        if (measure.function === "sum" || measure.function === "avg") {
            const { sum, count } = this.#sums.get(measure.column!) ?? { sum: [], count: 0 };

            return count === 0
                ? null
                : measure.function === "avg"
                  ? Number(total(sum)) / count
                  : (this.#node.json(measure.column!, total(sum)) as Scalar);
        }

        // write an extreme in the column's JSON form
        const extreme = this.#extremes.get(name) ?? null;

        return extreme === null ? null : (this.#node.json(measure.column!, extreme) as Scalar);
    }

    /** Add or subtract a row's values of each added-up column. */
    #add(row: Row, sign: 1 | -1): void {
        for (const column of this.#summed) {
            const value = row[column];
            if (value === null || value === undefined) {
                continue;
            }
            const entry = this.#sums.get(column) ?? {
                sum: typeof value === "bigint" ? 0n : [],
                count: 0,
            };
            const sum =
                typeof value === "bigint"
                    ? (entry.sum as bigint) + BigInt(sign) * value
                    : grow(entry.sum as readonly number[], sign * (value as number));
            this.#sums.set(column, { sum, count: entry.count + sign });
        }
    }
}

/** A group's measures as a page sends them: its rows, its measures, and the parts of its averages. */
export interface Result {
    /** The rows the group holds. */
    readonly rows: number;
    /** Whether the group holds no rows. */
    readonly isEmpty: boolean;
    /** Read the measures in their JSON form. */
    values(): Record<string, Scalar>;
    /** Read each average's sum and count of present values. */
    parts(): Record<string, Part>;
}

/** A group's measures as one SQL count read them, at the log sequence it read. */
export interface Measurement {
    /** The log sequence the count read at. */
    readonly sequence: number;
    /** The rows the group holds. */
    readonly rows: number;
    /** The sums of the added-up columns, by property. */
    readonly sums: Readonly<Record<string, Sum>>;
    /** The extreme measures' values, by measure name. */
    readonly extremes: Readonly<Record<string, unknown>>;
}

/** One column's sum over the group's rows, and how many of them hold a value. */
interface Sum {
    /** The sum of a bigint column, or the nonoverlapping partials whose exact sum a number column's values add up to. */
    readonly sum: bigint | readonly number[];
    /** The rows holding a value. */
    readonly count: number;
}

/** An average's parts: the sum of the group's present values in their JSON form, and how many there are. */
export interface Part {
    /** The sum. */
    readonly sum: Scalar;
    /** The present values. */
    readonly count: number;
}

/** Select a computed value, a number as a number. */
function valueOf(node: Node, name: string, visibility: Visibility | undefined): SQL {
    const value = sql`${node.sql(name, visibility)}`;

    return node.kindOf(name) === "text" ? value.mapWith(String) : value.mapWith(Number);
}

/** Select a measure's extreme, a computed number as a number. */
function extremeOf(node: Node, measure: Measure, visibility: Visibility | undefined): SQL {
    const extreme = (measure.function === "min" ? min : max)(node.sql(measure.column!, visibility));
    const computed = node.computed[measure.column!];

    return computed !== undefined && node.kindOf(measure.column!) !== "text"
        ? extreme.mapWith(Number)
        : extreme;
}

/** List the distinct columns sums and averages add up. */
export function summedColumns(measures: Readonly<Record<string, Measure>>): string[] {
    return [
        ...new Set(
            Object.values(measures)
                .filter((measure) => measure.function === "sum" || measure.function === "avg")
                .map((measure) => measure.column!),
        ),
    ];
}

/** Add a number to nonoverlapping partials exactly, as Shewchuk's expansion sum does. */
function grow(partials: readonly number[], value: number): number[] {
    // carry the value through each partial, keeping each rounding error as a smaller partial
    const next: number[] = [];
    let carried = value;
    for (const partial of partials) {
        const [larger, smaller] =
            Math.abs(carried) < Math.abs(partial) ? [partial, carried] : [carried, partial];
        const high = larger + smaller;
        const low = smaller - (high - larger);
        if (low !== 0) {
            next.push(low);
        }
        carried = high;
    }
    next.push(carried);

    return next;
}

/** Round the exact sum of partials to the nearest number, zero unsigned, and keep a bigint sum as it is. */
function total(sum: bigint | readonly number[]): number | bigint {
    // keep bigint sums, and add the partials from the largest down until a rounding error remains
    if (typeof sum === "bigint") {
        return sum;
    }
    let index = sum.length - 1;
    let high = sum[index] ?? 0;
    let low = 0;
    while (index > 0) {
        index -= 1;
        const next = sum[index]!;
        const rounded = high + next;
        low = next - (rounded - high);
        high = rounded;
        if (low !== 0) {
            break;
        }
    }

    // round half to even across the remaining partials, as the exact sum would
    if (index > 0 && ((low < 0 && sum[index - 1]! < 0) || (low > 0 && sum[index - 1]! > 0))) {
        const doubled = low * 2;
        const rounded = high + doubled;
        if (doubled === rounded - high) {
            high = rounded;
        }
    }

    // write an exact zero unsigned
    return high === 0 ? 0 : high;
}
