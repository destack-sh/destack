import {
    count,
    sql,
    sum,
    TABLE,
    type DatabaseConnection,
    type Dialect,
    max,
    min,
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
import type { Aggregate, Measure } from "../query/aggregate.ts";

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
    #count: number;
    /** The sums of the added-up columns, by property. */
    readonly #sums: Map<string, Sum>;
    /** The extreme values by measure name, null while the group has no value. */
    readonly #extremes: Map<string, ColumnValue>;
    /** Each extreme measure's values with their rows, absent once shed or for bytes and JSON. */
    #counts: Map<string, Map<ColumnValue, number>> | undefined;
    /** The log sequence the tally has changes up to. */
    sequence: number;
    /** The cached measures. */
    #values: Record<string, Scalar> | undefined;

    /** Start a tally from a count read at a log sequence. */
    constructor(node: Node, group: Record<string, Scalar>, measurement: Measurement) {
        // start with empty state
        this.#count = 0;
        this.#sums = new Map<string, Sum>();
        this.#extremes = new Map<string, ColumnValue>();

        // take the count's rows, sums and extremes
        const measures = Tally.#aggregate(node).values;
        this.#node = node;
        this.group = group;
        this.#measures = measures;
        this.#summed = Tally.#summedColumns(measures);
        this.sequence = measurement.sequence;
        this.#count = measurement.rows;
        this.#counts =
            measurement.counts === undefined
                ? undefined
                : new Map([...measurement.counts].map(([name, counts]) => [name, new Map(counts)]));
        for (const [column, summed] of Object.entries(measurement.sums)) {
            this.#sums.set(column, summed);
        }
        for (const [name, measure] of Object.entries(measures)) {
            if (measure.function === "min" || measure.function === "max") {
                this.#extremes.set(name, measurement.extremes[name] ?? null);
            }
        }
    }

    /** Measure an aggregate node's groups among a selection's rows, at one log sequence. */
    static async measure(
        database: DatabaseConnection,
        node: Node,
        selection: SQL,
        reading: TallyReading,
        visibility?: Visibility,
        admit?: (rows: readonly Row[]) => Promise<ReadonlySet<number>>,
    ): Promise<{ readonly sequence: number; readonly tallies: Tally[] }> {
        // tally real numbers and admitted rows in memory
        const aggregate = Tally.#aggregate(node);
        const grouping = node.grouping;
        const summed = Tally.#summedColumns(aggregate.values);
        if (admit !== undefined || summed.some((name) => node.columnKind(name) === "real")) {
            return Tally.#tally(database, node, selection, visibility, admit);
        }

        // select the group columns and each measure's terms
        const extremes = Object.entries(aggregate.values).flatMap(([name, measure]) =>
            measure.function === "min" || measure.function === "max"
                ? [[name, measure] as const]
                : [],
        );
        const fields = Tally.#measuredFields(node, grouping, summed, extremes, reading, visibility);

        // read the groups by select list position beside the log's head, by each extreme measure's value when counting
        const valued = grouping.length + 1 + summed.length * 2;
        const positions = [
            ...grouping.map((_, index) => index + 1),
            ...(reading === "values" ? extremes.map((_, index) => valued + index + 1) : []),
        ];
        const { sequence, rows } = await Tally.#headed(database, (reader) =>
            reader
                .select({ group: fields, head: headFields(reader.dialect) })
                .from(node.table)
                .where(selection)
                .groupBy(...positions.map((position) => sql.raw(String(position)))),
        );

        // fold each read row into its group's tally, counting values only when read by value
        const tallies = new Map<string, Tally>();
        for (const row of rows) {
            Tally.#fold(
                tallies,
                node,
                row,
                { sequence, reading },
                { grouping, summed },
                extremes,
                database.dialect,
            );
        }

        return { sequence, tallies: [...tallies.values()] };
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
        const measured = Object.values(Tally.#aggregate(node).values).flatMap((measure) =>
            measure.function === "count" ? [] : [measure.column],
        );
        const names = new Set(
            [...node.grouping, ...measured].flatMap((name) => node.columnsRead(name)),
        );
        const fields = {
            ...(admit === undefined ? {} : node.table[TABLE].logged),
            ...Object.fromEntries([...names].map((name) => [name, node.column(name)])),
            ...Object.fromEntries(
                Object.keys(node.extras).map((name) => [
                    name,
                    Tally.#selected(node, name, visibility),
                ]),
            ),
        };

        // read them beside the log's head
        const read = await Tally.#headed(database, (reader) =>
            reader
                .select({ group: fields, head: headFields(reader.dialect) })
                .from(node.table)
                .where(selection),
        );
        const sequence = read.sequence;
        const rows = read.rows.map((row) => Tally.#row(row));

        // tally each admitted row in its group
        const admitted = admit === undefined ? undefined : await admit(rows);
        const tallies = new Map<string, Tally>();
        for (const [index, row] of rows.entries()) {
            if (admitted !== undefined && !admitted.has(index)) {
                continue;
            }
            const group = node.rowGroup(row);
            const key = JSON.stringify(group);
            const tally = tallies.get(key) ?? Tally.empty(node, group, sequence);
            tallies.set(key, tally);
            tally.count(row, 1);
        }

        return { sequence, tallies: [...tallies.values()] };
    }

    /** Start an empty tally at a log sequence, counting each extreme measure's values from none. */
    static empty(node: Node, group: Record<string, Scalar>, sequence: number): Tally {
        const counts = new Map(
            Object.entries(Tally.#aggregate(node).values).flatMap(([name, measure]) =>
                measure.function === "min" || measure.function === "max"
                    ? [[name, new Map<ColumnValue, number>()] as const]
                    : [],
            ),
        );

        return new Tally(node, group, { sequence, rows: 0, sums: {}, extremes: {}, counts });
    }

    /** The distinct values the tally counts across its extreme measures. */
    get counted(): number {
        let counted = 0;
        for (const counts of this.#counts?.values() ?? []) {
            counted += counts.size;
        }

        return counted;
    }

    /** Stop counting the extreme measures' values, recounting a lost extreme from the database. */
    shed(): void {
        this.#counts = undefined;
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

        // keep each extreme from the counted values, or lose it without them
        let isExact = true;
        for (const [name, measure] of Object.entries(this.#measures)) {
            if (measure.function !== "min" && measure.function !== "max") {
                continue;
            }
            const value = row[measure.column] ?? null;
            if (value === null) {
                continue;
            }
            const counts = this.#countsOrShed(name, value);
            const extreme = this.#extremes.get(name) ?? null;
            const order = extreme === null ? undefined : Order.values(value, extreme);
            // count the value out, reading the next extreme from the counted values once it leaves
            if (sign < 0 && counts !== undefined) {
                const left = Tally.#uncount(counts, value);
                if (order === 0 && left === 0) {
                    this.#extremes.set(name, Tally.#extremeOf(counts, measure.function));
                }
            }
            // lose an extreme counted out without counted values
            else if (sign < 0) {
                isExact &&= order !== 0;
            }
            // count the value in, keeping a new extreme
            else {
                counts?.set(value, (counts.get(value) ?? 0) + 1);
                if (order === undefined || (measure.function === "min" ? order < 0 : order > 0)) {
                    this.#extremes.set(name, value);
                }
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
                            sum: this.#node.scalar(measure.column, Tally.#total(summed)),
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
                ? Number(Tally.#total(summed)) / summed.count
                : this.#node.scalar(measure.column, Tally.#total(summed));
        }

        // write an extreme in JSON form
        return this.#node.scalar(measure.column, this.#extremes.get(name) ?? null);
    }

    /** Read an extreme measure's counted values, shedding them all for bytes or JSON. */
    #countsOrShed(name: string, value: ColumnValue): Map<ColumnValue, number> | undefined {
        // count only values a map tells apart by value
        if (typeof value === "object") {
            this.#counts = undefined;
        }

        return this.#counts?.get(name);
    }

    /** Count a value out, answering the rows left with it. */
    static #uncount(counts: Map<ColumnValue, number>, value: ColumnValue): number {
        // refuse counting out a value the tally never counted in
        const left = (counts.get(value) ?? 0) - 1;
        if (left < 0) {
            const shown = typeof value === "bigint" ? `${value}n` : JSON.stringify(value);
            throw new TypeError(`a tally counted out ${shown}, which it never counted in`);
        }
        if (left === 0) {
            counts.delete(value);
        } else {
            counts.set(value, left);
        }

        return left;
    }

    /** Read the least or greatest counted value, null for none. */
    static #extremeOf(
        counts: ReadonlyMap<ColumnValue, number>,
        direction: "min" | "max",
    ): ColumnValue {
        let extreme: ColumnValue | null = null;
        for (const value of counts.keys()) {
            const order = extreme === null ? undefined : Order.values(value, extreme);
            if (order === undefined || (direction === "min" ? order < 0 : order > 0)) {
                extreme = value;
            }
        }

        return extreme;
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
                const partials = Tally.#grow(entry?.partials ?? [], sign * value);
                this.#sums.set(column, { partials, count: (entry?.count ?? 0) + sign });
            } else {
                throw new TypeError(`${column} sums values of a kind other than its sum`);
            }
        }
    }

    /** Select a computed value, a number as a number. */
    static #selected(node: Node, name: string, visibility: Visibility | undefined): SQL {
        const value = sql`${node.sql(name, visibility)}`;

        return node.columnKind(name) === "text"
            ? value.mapWith((selected) => SELECTED_TEXT.parse(selected))
            : value.mapWith((selected) => SELECTED_NUMBER.parse(selected));
    }

    /** Select an aggregate node's group columns and measure terms by select list name. */
    static #measuredFields(
        node: Node,
        grouping: readonly string[],
        summed: readonly string[],
        extremes: readonly (readonly [string, Extract<Measure, { readonly column: string }>])[],
        reading: TallyReading,
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
                    `v${index}`,
                    Tally.#measured(node, measure, reading, visibility),
                ]),
            ),
        };
    }

    /** Fold one selected row, a group's rows of one value per extreme measure, into its tally. */
    static #fold(
        tallies: Map<string, Tally>,
        node: Node,
        row: Readonly<Record<string, unknown>>,
        read: { readonly sequence: number; readonly reading: TallyReading },
        selected: { readonly grouping: readonly string[]; readonly summed: readonly string[] },
        extremes: readonly (readonly [string, Extract<Measure, { readonly column: string }>])[],
        dialect: Dialect,
    ): void {
        // find the group's tally, starting it empty
        const group = node.rowGroup(
            Object.fromEntries(
                selected.grouping.map((name, index) => [name, DriverValue.parse(row[`g${index}`])]),
            ),
        );
        const name = JSON.stringify(group);
        let tally = tallies.get(name);
        if (tally === undefined) {
            tally = Tally.empty(node, group, read.sequence);
            if (read.reading === "extremes") {
                tally.shed();
            }
            tallies.set(name, tally);
        }

        // add the rows and the sums, exact decimals or partials
        const rows = COUNT.parse(row["rows"]);
        tally.#count += rows;
        for (const [index, column] of selected.summed.entries()) {
            const text = SUM.parse(row[`s${index}`]) ?? "0";
            const present = COUNT.parse(row[`c${index}`]);
            const before = tally.#sums.get(column);
            tally.#sums.set(
                column,
                node.columnKind(column) === "bigint"
                    ? {
                          exact:
                              (before !== undefined && "exact" in before ? before.exact : 0n) +
                              BigInt(text),
                          count: (before?.count ?? 0) + present,
                      }
                    : {
                          partials: Tally.#grow(
                              before !== undefined && "partials" in before ? before.partials : [],
                              Number(text),
                          ),
                          count: (before?.count ?? 0) + present,
                      },
            );
        }

        // count each extreme measure's value and keep the extreme
        for (const [index, [measureName, measure]] of extremes.entries()) {
            const value = Tally.#decoded(node, measure.column, row[`v${index}`], dialect);
            if (value === null) {
                continue;
            }
            const counts = tally.#countsOrShed(measureName, value);
            counts?.set(value, (counts.get(value) ?? 0) + rows);
            const extreme = tally.#extremes.get(measureName) ?? null;
            const order = extreme === null ? undefined : Order.values(value, extreme);
            if (order === undefined || (measure.function === "min" ? order < 0 : order > 0)) {
                tally.#extremes.set(measureName, value);
            }
        }
    }

    /** Decode a selected measured value as the rows hold it: a column's by its column. */
    static #decoded(node: Node, name: string, value: unknown, dialect: Dialect): DriverValue {
        return node.extras[name] === undefined && value !== null
            ? DriverValue.parse(node.column(name).definition.decode(value, dialect))
            : DriverValue.parse(value);
    }

    /** Select a measure's column value, or its extreme, a number for numeric computed values. */
    static #measured(
        node: Node,
        measure: Extract<Measure, { readonly column: string }>,
        reading: TallyReading,
        visibility: Visibility | undefined,
    ): SQL {
        // select each value when counting values, else the group's extreme
        const operand = node.sql(measure.column, visibility);
        const value =
            reading === "values"
                ? sql`${operand}`
                : measure.function === "min"
                  ? min(operand.getSQL())
                  : max(operand.getSQL());
        const computed = node.extras[measure.column];

        return computed !== undefined && node.columnKind(measure.column) !== "text"
            ? value.mapWith((selected) => SELECTED_NUMBER.parse(selected))
            : value;
    }

    /** List the distinct summed columns. */
    static #summedColumns(measures: Readonly<Record<string, Measure>>): string[] {
        return [
            ...new Set(
                Object.values(measures).flatMap((measure) =>
                    measure.function === "sum" || measure.function === "avg"
                        ? [measure.column]
                        : [],
                ),
            ),
        ];
    }

    /** Add a number to nonoverlapping partials exactly, as Shewchuk's expansion sum does. */
    static #grow(partials: readonly number[], value: number): number[] {
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
    static #total(summed: Sum): number | bigint {
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
    static #aggregate(node: Node): Aggregate {
        if (node.aggregate === undefined) {
            throw new TypeError(`${node.name} aggregates no rows`);
        }

        return node.aggregate;
    }

    /** Read a selected row's values as column values. */
    static #row(row: Readonly<Record<string, unknown>>): Row {
        return Object.fromEntries(
            Object.entries(row).map(([name, value]) => [name, Tally.#columnValue(value)]),
        );
    }

    /** Read one selected value, decoded by its column or as a driver value. */
    static #columnValue(value: unknown): ColumnValue {
        return typeof value === "object" && value !== null && !(value instanceof Uint8Array)
            ? schema.json().parse(value)
            : DriverValue.parse(value);
    }

    /** Read rows with the log's head in one statement, or in one read transaction without rows. */
    static async #headed<Group>(
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
}

/** A group's measures as a page sends them. */
export interface Result {
    /** The rows of the group. */
    readonly rows: number;
    /** Whether the group has no rows. */
    readonly isEmpty: boolean;
    /** Read the measures in JSON form. */
    readonly values: () => Record<string, Scalar>;
    /** Read each average's sum and count of present values. */
    readonly averages: () => Record<string, Average>;
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
    /** Each extreme measure's values with their rows, absent when the read counted none. */
    readonly counts: ReadonlyMap<string, ReadonlyMap<ColumnValue, number>> | undefined;
}

/** How a grouped read measures extremes: each group's extreme, or each value's rows to count. */
export type TallyReading = "extremes" | "values";

/** One column's sum over a group, with its count of present values. */
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
