import {
    Key,
    TABLE,
    type DatabaseConnection,
    type Table,
    Condition,
    Namespace,
    Order,
    Predicate,
    Scalar,
    type Rollup,
    Expression,
    type Row,
    type ColumnValue,
    type Measure,
    type Path,
    type QueryOptions,
    type Relation,
    Relations,
} from "@destack/db";
import { canonicalize, aligned } from "@destack/schema";
import type { Query } from "../../query/query.ts";
import type { ConditionAudience } from "./audience.ts";

/** A level of a query the oracle evaluates: the root's rows, or a relation's rows under options. */
interface Level extends QueryOptions {
    /** The level's table. */
    readonly table: Table;
    /** How the level's rows join the rows above, absent for a root. */
    readonly on?: Path;
}

/** What a subscriber should have. */
export interface Contents {
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
): Promise<Contents> {
    // read every row of each table
    const read = new Map<Table, Row[]>();
    for (const table of new Set(
        Object.values(queries).flatMap((query) =>
            tablesOf(query, query.relations ?? new Relations()),
        ),
    )) {
        read.set(table, await database.select().from(table));
    }
    const tables = new Rows(read);

    // select each query's rows and groups
    const contents: Contents = { rows: new Map(), results: new Map() };
    for (const [name, query] of Object.entries(queries)) {
        select(
            name,
            query,
            query.relations ?? new Relations(),
            query.scopes,
            tables.all(query.table),
            undefined,
            tables,
            audience,
            contents,
        );
    }

    return contents;
}

/** Name a group of an aggregate query, as a copy names it. */
export function groupName(query: string, group: Readonly<Record<string, unknown>>): string {
    return JSON.stringify(query) + canonicalize(group);
}

/** Select the rows of a query or include and what they include. */
function select(
    name: string,
    query: Level,
    relations: Relations,
    scopes: Query["scopes"],
    candidates: readonly Row[],
    partition: { readonly name: string; readonly value: ColumnValue } | undefined,
    tables: Rows,
    audience: ConditionAudience,
    contents: Contents,
): Row[] {
    // keep the visible matching rows with their values
    const table = query.table;
    const scope = "scope";
    const computed = query.extras ?? {};
    const rows = candidates
        .map((row) => ({
            ...row,
            ...Object.fromEntries(
                Object.entries(computed).map(([property, expression]) => [
                    property,
                    Expression.evaluate(expression, row, {
                        lookup: (via, column) =>
                            relatedOf(
                                query,
                                relations,
                                via,
                                {},
                                row,
                                scopes,
                                tables,
                                audience,
                            )[0]?.[column] ?? null,
                        rollup: (measure, via, column, where) =>
                            rolledUp(
                                measure,
                                column,
                                relatedOf(
                                    query,
                                    relations,
                                    via,
                                    where,
                                    row,
                                    scopes,
                                    tables,
                                    audience,
                                ),
                            ),
                    }),
                ]),
            ),
        }))
        .filter(
            (row) =>
                inScopes(scopes, row[scope]) &&
                audience.isVisible(table, row) &&
                meets(query, relations, row, scopes, tables, audience),
        );

    // measure the groups of an aggregate
    if (query.aggregate !== undefined) {
        const grouping = query.aggregate.groupBy ?? [];
        const groups = new Map<string, Row[]>();
        for (const row of rows) {
            const values: (readonly [string, ColumnValue])[] = [
                ...(partition === undefined ? [] : [[partition.name, partition.value] as const]),
                ...grouping.map((column): [string, ColumnValue] => [
                    column,
                    json(table, column, row[column]),
                ]),
            ];
            const group = groupName(name, Object.fromEntries(values));
            groups.set(group, [...(groups.get(group) ?? []), row]);
        }
        for (const [group, members] of groups) {
            contents.results.set(
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

    // select the first rows with hidden columns concealed
    const order = Order.complete(Order.of(query.orderBy ?? {}), table);
    const selected = rows
        .toSorted((left, right) => Order.rows(order, left, right))
        .slice(0, query.limit);
    for (const row of selected) {
        const hidden = audience.hidden(table, row);
        contents.rows.set(Key.name(table, row), {
            ...table[TABLE].encode(loggedOf(table, row)),
            ...Object.fromEntries(hidden.map((column) => [column, null])),
        });
    }

    // measure each selected row's relations
    for (const { suffix, via, where, values } of measures(query)) {
        const relation = relationOf(query, relations, via, where);
        const path = relation.on;
        if (path.kind !== "key" && path.kind !== "junction") {
            throw new TypeError(`relation ${via} follows a ${path.kind} path`);
        }
        const [partitionColumn, column] =
            path.kind === "key" ? [path.column, path.parent] : [path.from.column, path.from.key];
        for (const row of selected) {
            const related = relatedOf(query, relations, via, where, row, scopes, tables, audience);
            if (related.length > 0) {
                contents.results.set(
                    groupName(`${name}?${via}${suffix}`, {
                        [partitionColumn]: json(table, column, row[column]),
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

    // select each include per selected row
    for (const [child, selection] of Object.entries(query.with ?? {})) {
        const include = includeOf(relations.get(table, child), selection === true ? {} : selection);
        for (const row of selected) {
            const reached = reach(include, table, row, scopes, tables, audience);
            const path = include.on;
            for (const join of reached.joins) {
                if (path.kind !== "junction") {
                    throw new TypeError(`include ${child} joins rows without a join table`);
                }
                const joins = path.table;
                contents.rows.set(
                    Key.name(joins, join),
                    joins[TABLE].encode(loggedOf(joins, join)),
                );
            }
            const members = select(
                `${name}.${child}`,
                include,
                relations,
                scopesOf(path, scopes),
                reached.rows,
                reached.partition,
                tables,
                audience,
                contents,
            );

            // select the chains of a tree include
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
                        contents.rows.set(Key.name(table, between), {
                            ...table[TABLE].encode(loggedOf(table, between)),
                            ...Object.fromEntries(
                                audience.hidden(table, between).map((column) => [column, null]),
                            ),
                        });
                    }
                }
            }
        }
    }

    return selected;
}

/** Read the visible rows strictly between a lower row and an upper one. */
function chain(
    table: Table,
    column: string,
    lower: Row,
    upper: Row,
    scopes: Query["scopes"],
    tables: Rows,
    audience: ConditionAudience,
): Row[] {
    // walk up, guarding against cycles
    const key = aligned(table[TABLE].key, 0);
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
                    inScopes(scopes, entry["scope"]) &&
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
    query: Level,
    relations: Relations,
    row: Row,
    scopes: Query["scopes"],
    tables: Rows,
    audience: ConditionAudience,
): boolean {
    // decide the columns and relations
    if (query.where === undefined) {
        return true;
    }
    const exists = (via: string, where: Condition) => {
        // reach the related rows in the relation's own scopes
        const include = relationOf(query, relations, via, where);
        const scope = "scope";
        const nested = scopesOf(include.on, scopes);

        return reach(include, query.table, row, scopes, tables, audience).rows.some(
            (related) =>
                inScopes(nested, related[scope]) &&
                audience.isVisible(include.table, related) &&
                meets(include, relations, related, nested, tables, audience),
        );
    };

    // decide the condition against the query's columns and computed values
    const fields = Namespace.fields(query.table, { extras: query.extras ?? {} });
    const match = Predicate.compile(Condition.resolve(query.where, fields), query.table);

    return match({ field: (name) => row[name], placeholder: () => null, exists }) === true;
}

/** Read a row's visible related rows that meet a condition. */
function relatedOf(
    query: Level,
    relations: Relations,
    via: string,
    where: Condition,
    row: Row,
    scopes: Query["scopes"],
    tables: Rows,
    audience: ConditionAudience,
): Row[] {
    // reach the related rows in the relation's own scopes
    const include = relationOf(query, relations, via, where);
    const scope = "scope";
    const nested = scopesOf(include.on, scopes);

    return reach(include, query.table, row, scopes, tables, audience).rows.filter(
        (related) =>
            inScopes(nested, related[scope]) &&
            audience.isVisible(include.table, related) &&
            meets(include, relations, related, nested, tables, audience),
    );
}

/** Read the scopes an include's rows live in: every scope for rows joined by their scope column. */
function scopesOf(path: Path, scopes: Query["scopes"]): Query["scopes"] {
    return path.kind === "key" && path.column === "scope" ? "every" : scopes;
}

/** Decide whether a scope is among some scopes. */
function inScopes(scopes: Query["scopes"], scope: unknown): boolean {
    return scopes === "every" || (typeof scope === "string" && scopes.includes(scope));
}

/** Measure rows by a rollup. */
function rolledUp(measure: Rollup, column: string | undefined, rows: readonly Row[]): ColumnValue {
    // count every row
    if (measure === "count") {
        return rows.length;
    } else if (column === undefined) {
        throw new TypeError(`rollup ${measure} has no column`);
    }

    // add up or take the extreme of the present values
    const values = rows
        .map((row) => row[column])
        .filter((value) => value !== null && value !== undefined);
    if (values.length === 0) {
        return null;
    } else if (measure === "sum") {
        return values.reduce<number>((total, value) => total + Number(value), 0);
    }
    const sorted = values.toSorted((left, right) => Order.missingFirst(left, right));

    return aligned(sorted, measure === "min" ? 0 : sorted.length - 1);
}

/** Read a level's relation under a condition as an include. */
function relationOf(
    query: Level,
    relations: Relations,
    via: string,
    where: Condition,
): Level & { readonly on: Path } {
    return includeOf(relations.get(query.table, via), { where });
}

/** Read a relation's rows under options as an include, meeting the relation's condition too. */
function includeOf(relation: Relation, options: QueryOptions): Level & { readonly on: Path } {
    const where =
        relation.where === undefined || options.where === undefined
            ? (relation.where ?? options.where)
            : { AND: [relation.where, options.where] };

    return {
        ...options,
        table: relation.table,
        on: relation.on,
        ...(where === undefined ? {} : { where }),
    };
}

/** List the measures of each relation and condition a query reads. */
function measures(query: Level): {
    readonly suffix: string;
    readonly via: string;
    readonly where: Condition;
    readonly values: Record<string, Measure>;
}[] {
    // count each relation and condition's rows
    const found = new Map<
        string,
        {
            suffix: string;
            via: string;
            where: Condition;
            values: Record<string, Measure>;
        }
    >();
    const use = (
        via: string,
        where: Condition,
        measure?: Exclude<Measure, { readonly function: "count" }>,
    ) => {
        // add the measures a use reads
        const suffix = canonicalize(where);
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
    const fields = Namespace.fields(query.table, { extras: query.extras ?? {} });
    for (const { via, where } of Predicate.relations(
        Condition.resolve(query.where ?? {}, fields),
    )) {
        use(via, where);
    }
    for (const expression of Object.values(query.extras ?? {})) {
        for (const { via, column } of Expression.lookups(expression)) {
            use(via, {}, { function: "min", column });
        }
        for (const rollup of rollupsOf(expression)) {
            // count the related rows or measure a column of them
            const column = rollup.column;
            if (rollup.function === "count") {
                use(rollup.via, rollup.where);
            } else if (column === undefined) {
                throw new TypeError(`rollup ${rollup.function} over ${rollup.via} has no column`);
            } else {
                use(rollup.via, rollup.where, { function: rollup.function, column });
            }
        }
    }

    return [...found.values()];
}

/** List the rollups an expression has. */
function rollupsOf(expression: Expression): Extract<Expression, { readonly kind: "rollup" }>[] {
    // take a rollup itself
    if (expression.kind === "rollup") {
        return [expression];
    }
    // read both operands of arithmetic
    else if (
        expression.kind === "add" ||
        expression.kind === "subtract" ||
        expression.kind === "multiply" ||
        expression.kind === "divide"
    ) {
        return [...rollupsOf(expression.left), ...rollupsOf(expression.right)];
    }
    // read each coalesced value
    else if (expression.kind === "coalesce") {
        return expression.values.flatMap(rollupsOf);
    }
    // find none in any other expression
    else {
        return [];
    }
}

/** Read the rows an include's path joins to one parent row. */
function reach(
    include: Level & { readonly on: Path },
    table: Table,
    parent: Row,
    scopes: Query["scopes"],
    tables: Rows,
    audience: ConditionAudience,
): {
    readonly rows: Row[];
    readonly joins: Row[];
    readonly partition: { readonly name: string; readonly value: ColumnValue };
} {
    // pass only through visible rows of the scopes
    const path = include.on;
    const passes = (through: Table, row: Row) =>
        inScopes(scopes, row["scope"]) && audience.isVisible(through, row);
    // join a column to the parent row's
    if (path.kind === "key") {
        return {
            rows: tables.where(include.table, path.column, parent[path.parent]),
            joins: [],
            partition: { name: path.column, value: json(table, path.parent, parent[path.parent]) },
        };
    }
    // join through the join rows naming the parent row
    else if (path.kind === "junction") {
        const joins = tables
            .where(path.table, path.from.column, parent[path.from.key])
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
                value: json(table, path.from.key, parent[path.from.key]),
            },
        };
    }

    // follow the parent column
    const key = Object.keys(table[TABLE].columns).find((column) =>
        table[TABLE].key.includes(column),
    );
    if (key === undefined) {
        throw new TypeError(`${table[TABLE].name} has no key column`);
    }
    const reached: Row[] = [];
    const seen = new Set<unknown>([parent[key]]);
    if (path.kind === "descendants") {
        let level = [parent];
        while (level.length > 0) {
            level = level.flatMap((holder) =>
                tables
                    .where(table, path.column, holder[key])
                    .filter((row) => passes(table, row) && !seen.has(row[key])),
            );
            for (const row of level) {
                seen.add(row[key]);
            }
            reached.push(...level);
        }
    } else {
        let current: Row | undefined = parent;
        while (current !== undefined) {
            const parentKey: ColumnValue | undefined = current[path.column];
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
            value: json(table, key, parent[key]),
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
    const column = measure.column;
    const values = rows
        .map((row) => row[column])
        .filter((value) => value !== null && value !== undefined);
    if (values.length === 0) {
        return null;
    }
    const sum = exactSum(values.map(Number));
    const direction = measure.function === "min" ? -1 : 1;
    const extreme = values.reduce((best, value) =>
        Math.sign(Order.missingFirst(value, best)) === direction ? value : best,
    );

    // sum, average or take the extreme
    if (measure.function === "sum") {
        return Scalar.parse(json(table, column, sum));
    } else if (measure.function === "avg") {
        return sum / values.length;
    } else {
        return Scalar.parse(json(table, column, extreme));
    }
}

/** Keep a row's logged columns for pages. */
function loggedOf(table: Table, row: Row): Row {
    return Object.fromEntries(
        Object.keys(table[TABLE].logged).flatMap((name) => {
            const value = row[name];

            return value === undefined ? [] : [[name, value]];
        }),
    );
}

/** Every row of the evaluated tables, with lazy column indexes. */
class Rows {
    /** The rows, by table. */
    readonly #rows: ReadonlyMap<Table, Row[]>;
    /** The rows by column value, by table and column. */
    readonly #indexes = new Map<Table, Map<string, Map<string, Row[]>>>();

    /** Keep every row of some tables. */
    constructor(rows: ReadonlyMap<Table, Row[]>) {
        this.#rows = rows;
    }

    /** Read every row of a table. */
    all(table: Table): Row[] {
        const rows = this.#rows.get(table);
        if (rows === undefined) {
            throw new TypeError(`${table[TABLE].name} was not read`);
        }

        return rows;
    }

    /** Read the rows of a table whose column has a value. */
    where(table: Table, column: string, value: ColumnValue | undefined): Row[] {
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

/** List the tables a query reads: its own, its includes' and the relations its conditions and values follow. */
function tablesOf(query: Level, relations: Relations): Table[] {
    return [
        query.table,
        ...(query.on?.kind === "junction" ? [query.on.table] : []),
        ...Object.entries(query.with ?? {}).flatMap(([child, selected]) =>
            tablesOf(
                includeOf(relations.get(query.table, child), selected === true ? {} : selected),
                relations,
            ),
        ),
        ...measures(query).flatMap(({ via, where }) =>
            tablesOf(relationOf(query, relations, via, where), relations),
        ),
    ];
}

/** Write a column value in JSON form. */
function json(table: Table, column: string, value: ColumnValue | undefined): ColumnValue {
    // write a missing value as null and a computed one without negative zero
    const definition = table[TABLE].columns[column]?.definition;
    if (value === null || value === undefined) {
        return null;
    } else if (definition === undefined) {
        return value === 0 ? 0 : value;
    } else {
        return definition.toJson(value);
    }
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
