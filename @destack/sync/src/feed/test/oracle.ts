import { encodeRow, Key, TABLE, type DatabaseConnection, type Table } from "@destack/db";
import { Condition, type Rollup, Expression, Order, type Scalar } from "@destack/db/query";
import { canonicalize } from "@destack/schema/json";
import type { Include, Measure, Query, Relation } from "../../query/query.ts";
import type { Row } from "@destack/db";
import type { ConditionAudience } from "./audience.ts";

/** What a subscriber should hold. */
export interface Holding {
    /** The rows, by table and key. */
    readonly rows: Map<string, Record<string, unknown>>;
    /** The aggregate groups, by query name and group. */
    readonly results: Map<string, Record<string, Scalar>>;
}

/** Evaluate queries in memory over every row of their tables. */
export async function evaluate(
    database: DatabaseConnection,
    queries: Readonly<Record<string, Query>>,
    audience: ConditionAudience,
): Promise<Holding> {
    // read every row of each table
    const read = new Map<Table, Row[]>();
    for (const table of new Set(Object.values(queries).flatMap(tablesOf))) {
        read.set(table, (await database.select().from(table as never)) as Row[]);
    }
    const tables = new Rows(read);

    // hold each query's rows and groups
    const holding: Holding = { rows: new Map(), results: new Map() };
    for (const [name, query] of Object.entries(queries)) {
        hold(
            name,
            query,
            query.scopes,
            tables.all(query.table),
            undefined,
            tables,
            audience,
            holding,
        );
    }

    return holding;
}

/** Name a group of an aggregate query, as a copy names it. */
export function groupName(query: string, group: Readonly<Record<string, unknown>>): string {
    return JSON.stringify(query) + canonicalize(group);
}

/** Hold the rows of a query or include and what they include. */
function hold(
    name: string,
    query: Query | Include,
    scopes: readonly string[],
    candidates: readonly Row[],
    partition: { readonly name: string; readonly value: unknown } | undefined,
    tables: Rows,
    audience: ConditionAudience,
    holding: Holding,
): Row[] {
    // keep the visible matching rows with their values
    const table = query.table;
    const scope = "scope";
    const computed = query.compute ?? {};
    const rows = candidates
        .map((row) => ({
            ...row,
            ...Object.fromEntries(
                Object.entries(computed).map(([name, expression]) => [
                    name,
                    Expression.evaluate(expression, row, {
                        lookup: (via, column) =>
                            (relatedOf(query, via, undefined, row, scopes, tables, audience)[0]?.[
                                column
                            ] ?? null) as Scalar,
                        rollup: (measure, via, column, where) =>
                            rolledUp(
                                measure,
                                column,
                                relatedOf(query, via, where, row, scopes, tables, audience),
                            ),
                    }),
                ]),
            ),
        }))
        .filter(
            (row) =>
                scopes.includes(row[scope] as string) &&
                audience.isVisible(table, row) &&
                meets(query, row, scopes, tables, audience),
        );

    // measure the groups of an aggregate
    if (query.aggregate !== undefined) {
        const grouping = query.aggregate.groupBy ?? [];
        const groups = new Map<string, Row[]>();
        for (const row of rows) {
            const group = groupName(
                name,
                Object.fromEntries([
                    ...(partition === undefined ? [] : [[partition.name, partition.value]]),
                    ...grouping.map((column) => [column, json(table, column, row[column])]),
                ]),
            );
            groups.set(group, [...(groups.get(group) ?? []), row]);
        }
        for (const [group, members] of groups) {
            holding.results.set(
                group,
                Object.fromEntries(
                    Object.entries(query.aggregate.values).map(([measure, entry]) => [
                        measure,
                        measured(table, entry, members),
                    ]),
                ),
            );
        }

        return [];
    }

    // hold the first rows, concealing hidden columns
    const order = Order.complete(query.order ?? [], table);
    const held = [...rows]
        .sort((left, right) => Order.rows(order, left, right))
        .slice(0, query.limit);
    for (const row of held) {
        const hidden = audience.hidden(table, row);
        holding.rows.set(Key.name(table, row), {
            ...encodeRow(table, loggedOf(table, row)),
            ...Object.fromEntries(hidden.map((column) => [column, null])),
        });
    }

    // measure each held row's relations
    for (const { suffix, via, where, values } of measures(query)) {
        const relation = relationOf(query, via, where);
        const path = relation.on as Extract<Include["on"], { kind: "key" | "junction" }>;
        const [partition, column] =
            path.kind === "key" ? [path.column, path.parent] : [path.from.column, path.from.key];
        for (const row of held) {
            const related = relatedOf(query, via, where, row, scopes, tables, audience);
            if (related.length > 0) {
                holding.results.set(
                    groupName(`${name}?${via}${suffix}`, {
                        [partition]: json(table, column, row[column]),
                    }),
                    Object.fromEntries(
                        Object.entries(values).map(([measure, entry]) => [
                            measure,
                            measured(relation.table, entry, related),
                        ]),
                    ),
                );
            }
        }
    }

    // hold each include per held row
    for (const [child, include] of Object.entries(query.include ?? {})) {
        for (const row of held) {
            const reached = reach(include, table, row, scopes, tables, audience);
            for (const join of reached.joins) {
                const joins = include.on as { readonly table: Table };
                holding.rows.set(
                    Key.name(joins.table, join),
                    encodeRow(joins.table, loggedOf(joins.table, join)),
                );
            }
            const members = hold(
                `${name}.${child}`,
                include,
                scopes,
                reached.rows,
                reached.partition,
                tables,
                audience,
                holding,
            );

            // hold the chains of a tree include
            const path = include.on;
            if (
                (path.kind === "descendants" || path.kind === "ancestors") &&
                (include.limit !== undefined || include.where !== undefined)
            ) {
                for (const member of members) {
                    const [lower, upper] =
                        path.kind === "descendants" ? [member, row] : [row, member];
                    for (const between of chain(
                        table,
                        path.column,
                        lower,
                        upper,
                        scopes,
                        tables,
                        audience,
                    )) {
                        holding.rows.set(Key.name(table, between), {
                            ...encodeRow(table, loggedOf(table, between)),
                            ...Object.fromEntries(
                                audience.hidden(table, between).map((column) => [column, null]),
                            ),
                        });
                    }
                }
            }
        }
    }

    return held;
}

/** Read the visible rows strictly between a lower row and an upper one. */
function chain(
    table: Table,
    column: string,
    lower: Row,
    upper: Row,
    scopes: readonly string[],
    tables: Rows,
    audience: ConditionAudience,
): Row[] {
    // walk up, guarding against cycles
    const key = table[TABLE].key[0]!;
    const rows: Row[] = [];
    const seen = new Set<unknown>([lower[key]]);
    let current = lower;
    for (;;) {
        // stop at the upper row, a cycle or an impassable row
        const parentKey = current[column];
        if (parentKey === null || parentKey === undefined || seen.has(parentKey)) {
            return [];
        } else if (Order.values(parentKey, upper[key]) === 0) {
            return rows;
        }
        const parent = tables
            .where(table, key, parentKey)
            .find(
                (entry) =>
                    Order.values(entry[key], parentKey) === 0 &&
                    scopes.includes(entry.scope as string) &&
                    audience.isVisible(table, entry),
            );
        if (parent === undefined) {
            return [];
        }
        seen.add(parentKey);
        rows.push(parent);
        current = parent;
    }
}

/** Decide whether a row meets a query's condition. */
function meets(
    query: Query | Include,
    row: Row,
    scopes: readonly string[],
    tables: Rows,
    audience: ConditionAudience,
): boolean {
    // decide the columns and relations
    if (query.where === undefined) {
        return true;
    }
    const exists = (via: string, where: Condition | undefined) => {
        const include = relationOf(query, via, where);
        const scope = "scope";

        return reach(include, query.table, row, scopes, tables, audience).rows.some(
            (related) =>
                scopes.includes(related[scope] as string) &&
                audience.isVisible(include.table, related) &&
                meets(include, related, scopes, tables, audience),
        );
    };

    return (
        Condition.compile(
            query.where,
            query.table,
        )({
            column: (name) => row[name],
            parameter: () => null,
            exists,
        }) === true
    );
}

/** Read a row's visible related rows that meet a condition. */
function relatedOf(
    query: Query | Include,
    via: string,
    where: Condition | undefined,
    row: Row,
    scopes: readonly string[],
    tables: Rows,
    audience: ConditionAudience,
): Row[] {
    const include = relationOf(query, via, where);
    const scope = "scope";

    return reach(include, query.table, row, scopes, tables, audience).rows.filter(
        (related) =>
            scopes.includes(related[scope] as string) &&
            audience.isVisible(include.table, related) &&
            meets(include, related, scopes, tables, audience),
    );
}

/** Measure rows by a rollup. */
function rolledUp(measure: Rollup, column: string | undefined, rows: readonly Row[]): Scalar {
    if (measure === "count") {
        return rows.length;
    }
    const values = rows
        .map((row) => row[column!])
        .filter((value) => value !== null && value !== undefined);
    if (values.length === 0) {
        return null;
    } else if (measure === "sum") {
        return values.reduce<number>((total, value) => total + Number(value), 0);
    }
    const sorted = [...values].sort((left, right) => Order.values(left, right)!);

    return (measure === "min" ? sorted[0] : sorted.at(-1)) as Scalar;
}

/** Read a relation as an include. */
function relationOf(query: Query | Include, via: string, where: Condition | undefined): Include {
    const relation = query.relations![via]!;

    return {
        table: relation.table,
        on: relation.on,
        ...(where === undefined ? {} : { where }),
        ...(relation.relations === undefined ? {} : { relations: relation.relations }),
    };
}

/** List the measures of each relation and condition a query reads. */
function measures(query: Query | Include): {
    readonly suffix: string;
    readonly via: string;
    readonly where: Condition | undefined;
    readonly values: Record<string, Measure>;
}[] {
    // count each relation and condition's rows
    const found = new Map<
        string,
        {
            suffix: string;
            via: string;
            where: Condition | undefined;
            values: Record<string, Measure>;
        }
    >();
    const use = (via: string, where: Condition | undefined, measure?: Measure) => {
        // add the measures a use reads
        const suffix = where === undefined ? "" : canonicalize(where);
        const entry = found.get(`${via}${suffix}`) ?? {
            suffix,
            via,
            where,
            values: { count: { function: "count" } },
        };
        found.set(`${via}${suffix}`, entry);
        if (measure !== undefined) {
            entry.values[`${measure.function}(${measure.column})`] = measure;
        }
    };
    for (const { via, where } of Condition.relations(query.where ?? Condition.all())) {
        use(via, where);
    }
    for (const expression of Object.values(query.compute ?? {})) {
        for (const { via, column } of Expression.lookups(expression)) {
            use(via, undefined, { function: "min", column });
        }
        for (const rollup of rollupsOf(expression)) {
            use(
                rollup.via,
                rollup.where,
                rollup.function === "count"
                    ? undefined
                    : { function: rollup.function, column: rollup.column },
            );
        }
    }

    return [...found.values()];
}

/** List the rollups an expression holds. */
function rollupsOf(expression: Expression): Extract<Expression, { readonly kind: "rollup" }>[] {
    return expression.kind === "rollup"
        ? [expression]
        : expression.kind === "add" ||
            expression.kind === "subtract" ||
            expression.kind === "multiply" ||
            expression.kind === "divide"
          ? [...rollupsOf(expression.left), ...rollupsOf(expression.right)]
          : expression.kind === "coalesce"
            ? expression.values.flatMap(rollupsOf)
            : [];
}

/** Read the rows an include's path joins to one held row. */
function reach(
    include: Include,
    table: Table,
    held: Row,
    scopes: readonly string[],
    tables: Rows,
    audience: ConditionAudience,
): {
    readonly rows: Row[];
    readonly joins: Row[];
    readonly partition: { readonly name: string; readonly value: unknown };
} {
    // pass only through visible rows of the scopes
    const path = include.on;
    const passes = (through: Table, row: Row) =>
        scopes.includes(row.scope as string) && audience.isVisible(through, row);
    // join a column to the held row's
    if (path.kind === "key") {
        return {
            rows: tables.where(include.table, path.column, held[path.parent]),
            joins: [],
            partition: { name: path.column, value: json(table, path.parent, held[path.parent]) },
        };
    }
    // join through the join rows naming the held row
    else if (path.kind === "junction") {
        const joins = tables
            .where(path.table, path.from.column, held[path.from.key])
            .filter((join) => passes(path.table, join));
        const joined = new Map<string, Row>();
        for (const join of joins) {
            for (const row of tables.where(include.table, path.to.key, join[path.to.column])) {
                joined.set(Key.name(include.table, row), row);
            }
        }

        return {
            rows: [...joined.values()],
            joins,
            partition: {
                name: path.from.column,
                value: json(table, path.from.key, held[path.from.key]),
            },
        };
    }

    // follow the parent column
    const key = Object.keys(table[TABLE].columns).find((column) =>
        table[TABLE].key.includes(column),
    )!;
    const reached: Row[] = [];
    const seen = new Set<unknown>([held[key]]);
    if (path.kind === "descendants") {
        let level = [held];
        while (level.length > 0) {
            level = level.flatMap((parent) =>
                tables
                    .where(table, path.column, parent[key])
                    .filter((row) => passes(table, row) && !seen.has(row[key])),
            );
            for (const row of level) {
                seen.add(row[key]);
            }
            reached.push(...level);
        }
    } else {
        let current: Row | undefined = held;
        while (current !== undefined) {
            const parentKey: unknown = current[path.column];
            current = seen.has(parentKey)
                ? undefined
                : tables.where(table, key, parentKey).find((row) => passes(table, row));
            if (current !== undefined) {
                seen.add(current[key]);
                reached.push(current);
            }
        }
    }

    return {
        rows: reached,
        joins: [],
        partition: {
            name: path.kind === "descendants" ? "ancestor" : "descendant",
            value: json(table, key, held[key]),
        },
    };
}

/** Measure one group's rows in JSON form. */
function measured(table: Table, measure: Measure, rows: readonly Row[]): Scalar {
    // count rows
    if (measure.function === "count") {
        return rows.length;
    }

    // measure the present values
    const values = rows
        .map((row) => row[measure.column!])
        .filter((value) => value !== null && value !== undefined);
    if (values.length === 0) {
        return null;
    }
    const sum = exactSum(values.map(Number));
    const extreme = values.reduce((best, value) => {
        const order = Order.values(value, best)!;

        return measure.function === "min" ? (order < 0 ? value : best) : order > 0 ? value : best;
    });

    return measure.function === "sum"
        ? (json(table, measure.column!, sum) as Scalar)
        : measure.function === "avg"
          ? sum / values.length
          : (json(table, measure.column!, extreme) as Scalar);
}

/** Keep a row's logged columns, which pages carry. */
function loggedOf(table: Table, row: Row): Row {
    return Object.fromEntries(Object.keys(table[TABLE].logged).map((name) => [name, row[name]]));
}

/** Every row of the evaluated tables, with lazy column indexes. */
class Rows {
    /** The rows, by table. */
    readonly #rows: ReadonlyMap<Table, Row[]>;
    /** The rows by column value, by table and column. */
    readonly #indexes = new Map<Table, Map<string, Map<string, Row[]>>>();

    /** Hold every row of some tables. */
    constructor(rows: ReadonlyMap<Table, Row[]>) {
        this.#rows = rows;
    }

    /** Read every row of a table. */
    all(table: Table): Row[] {
        return this.#rows.get(table)!;
    }

    /** Read the rows of a table whose column holds a value. */
    where(table: Table, column: string, value: unknown): Row[] {
        // index the column once
        const byColumn = this.#indexes.get(table) ?? new Map<string, Map<string, Row[]>>();
        this.#indexes.set(table, byColumn);
        let index = byColumn.get(column);
        if (index === undefined) {
            index = new Map();
            for (const row of this.all(table)) {
                const name = JSON.stringify(json(table, column, row[column]));
                index.set(name, [...(index.get(name) ?? []), row]);
            }
            byColumn.set(column, index);
        }

        return value === null || value === undefined
            ? []
            : (index.get(JSON.stringify(json(table, column, value))) ?? []);
    }
}

/** List the tables a query reads. */
function tablesOf(query: Query | Include | Relation): Table[] {
    return [
        query.table,
        ...("on" in query && query.on.kind === "junction" ? [query.on.table] : []),
        ...Object.values("include" in query ? (query.include ?? {}) : {}).flatMap(tablesOf),
        ...Object.values(query.relations ?? {}).flatMap(tablesOf),
    ];
}

/** Write a column value in JSON form. */
function json(table: Table, column: string, value: unknown): unknown {
    return value === null || value === undefined
        ? null
        : table[TABLE].columns[column] === undefined
          ? value === 0
              ? 0
              : value
          : table[TABLE].columns[column].definition.toJson(value);
}

/** The smallest subnormal's exponent. */
const SCALE = 1074n;

/** Add numbers exactly and round the sum half to even once. */
function exactSum(values: readonly number[]): number {
    // add the values as scaled integers
    let scaled = 0n;
    for (const value of values) {
        const [mantissa, exponent] = decompose(value);
        scaled += mantissa << (exponent + SCALE);
    }
    const sign = scaled < 0n ? -1 : 1;
    const magnitude = scaled < 0n ? -scaled : scaled;

    // round to 53 significant bits, half to even
    const bits = magnitude.toString(2).length;
    const dropped = BigInt(Math.max(0, bits - 53));
    let kept = magnitude >> dropped;
    const remainder = magnitude - (kept << dropped);
    const half = dropped === 0n ? 0n : 1n << (dropped - 1n);
    if (dropped > 0n && (remainder > half || (remainder === half && (kept & 1n) === 1n))) {
        kept += 1n;
    }

    return sign * Number(kept) * 2 ** (Number(dropped) - Number(SCALE));
}

/** Split a finite number into an integer mantissa and a power of two. */
function decompose(value: number): [bigint, bigint] {
    // read the IEEE 754 fields
    const view = new DataView(new ArrayBuffer(8));
    view.setFloat64(0, value);
    const bits = view.getBigUint64(0);
    const sign = bits >> 63n === 1n ? -1n : 1n;
    const exponent = (bits >> 52n) & 0x7ffn;
    const fraction = bits & 0xfffffffffffffn;

    // read subnormals without the implicit bit
    return exponent === 0n
        ? [sign * fraction, -SCALE]
        : [sign * (fraction | (1n << 52n)), exponent - 1075n];
}
