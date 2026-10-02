import { Column, type ColumnValue } from "../table/column.ts";
import { type Aliased, isSQLWrapper, Parameter, SQL, sql, type SQLWrapper } from "./sql.ts";

/**
 * The most predicates one flat chain joins.
 *
 * 90 levels keep compound keys far below SQLite's expression depth limit of 1000.
 */
export const CHAIN_TERMS = 90;

/** A typed operand: a column, a fragment or a named fragment. */
type Typed<Value> = Column<Extract<Value, ColumnValue>> | SQL<Value> | Aliased<Value>;

/** A comparison of a typed operand with a value or another fragment. */
export type Comparison = <Value>(
    left: Typed<Value>,
    right: NoInfer<Value> | SQLWrapper,
) => SQL<boolean>;

/** Compare equal values. */
export const eq: Comparison = (left, right) => compare(left, "=", right);
/** Compare unequal values. */
export const ne: Comparison = (left, right) => compare(left, "<>", right);
/** Compare a greater value. */
export const gt: Comparison = (left, right) => compare(left, ">", right);
/** Compare a greater or equal value. */
export const gte: Comparison = (left, right) => compare(left, ">=", right);
/** Compare a smaller value. */
export const lt: Comparison = (left, right) => compare(left, "<", right);
/** Compare a smaller or equal value. */
export const lte: Comparison = (left, right) => compare(left, "<=", right);

/** Require every supplied predicate. */
export function and(first: SQLWrapper, ...rest: (SQLWrapper | undefined)[]): SQL<boolean>;
/** Combine the present predicates with AND. */
export function and(...conditions: (SQLWrapper | undefined)[]): SQL<boolean> | undefined;
/** Combine the present predicates with AND, whose signatures above keep a required first predicate present. */
export function and(...conditions: (SQLWrapper | undefined)[]): SQL<boolean> | undefined {
    const present = conditions.filter((condition) => condition !== undefined);

    return present.length === 0 ? undefined : combine(present, "AND");
}

/** Require at least one supplied predicate. */
export function or(first: SQLWrapper, ...rest: (SQLWrapper | undefined)[]): SQL<boolean>;
/** Combine the present predicates with OR. */
export function or(...conditions: (SQLWrapper | undefined)[]): SQL<boolean> | undefined;
/** Combine the present predicates with OR, whose signatures above keep a required first predicate present. */
export function or(...conditions: (SQLWrapper | undefined)[]): SQL<boolean> | undefined {
    const present = conditions.filter((condition) => condition !== undefined);

    return present.length === 0 ? undefined : combine(present, "OR");
}

/** Negate a predicate. */
export function not(condition: SQLWrapper): SQL<boolean> {
    return sql<boolean>`NOT ${condition}`;
}

/** Match a typed value in a list or subquery. */
export function inArray<Value>(
    value: Typed<Value>,
    candidates: readonly NoInfer<Value>[] | SQLWrapper,
): SQL<boolean>;
/** Match a value in a list or subquery. */
export function inArray(
    value: SQLWrapper,
    candidates: readonly unknown[] | SQLWrapper,
): SQL<boolean>;
/** Match a value in a list or subquery, bound through its column. */
export function inArray(
    value: SQLWrapper,
    candidates: readonly unknown[] | SQLWrapper,
): SQL<boolean> {
    return membership(value, "IN", candidates, sql`false`);
}

/** Match a typed value outside a list or subquery. */
export function notInArray<Value>(
    value: Typed<Value>,
    candidates: readonly NoInfer<Value>[] | SQLWrapper,
): SQL<boolean>;
/** Match a value outside a list or subquery. */
export function notInArray(
    value: SQLWrapper,
    candidates: readonly unknown[] | SQLWrapper,
): SQL<boolean>;
/** Match a value outside a list or subquery, bound through its column. */
export function notInArray(
    value: SQLWrapper,
    candidates: readonly unknown[] | SQLWrapper,
): SQL<boolean> {
    return membership(value, "NOT IN", candidates, sql`true`);
}

/** Match a missing value. */
export function isNull(value: SQLWrapper): SQL<boolean> {
    return sql<boolean>`${value} IS NULL`;
}

/** Match a present value. */
export function isNotNull(value: SQLWrapper): SQL<boolean> {
    return sql<boolean>`${value} IS NOT NULL`;
}

/** Match text by a LIKE pattern. */
export function like(value: Typed<string>, pattern: string | SQLWrapper): SQL<boolean> {
    return sql<boolean>`${value} LIKE ${bind(value, pattern)}`;
}

/** Match text outside a LIKE pattern. */
export function notLike(value: Typed<string>, pattern: string | SQLWrapper): SQL<boolean> {
    return sql<boolean>`${value} NOT LIKE ${bind(value, pattern)}`;
}

/** Match a value within inclusive bounds. */
export function between<Value>(
    value: Typed<Value>,
    lower: NoInfer<Value> | SQLWrapper,
    upper: NoInfer<Value> | SQLWrapper,
): SQL<boolean> {
    return sql<boolean>`${value} BETWEEN ${bind(value, lower)} AND ${bind(value, upper)}`;
}

/** Match a value outside inclusive bounds. */
export function notBetween<Value>(
    value: Typed<Value>,
    lower: NoInfer<Value> | SQLWrapper,
    upper: NoInfer<Value> | SQLWrapper,
): SQL<boolean> {
    return sql<boolean>`${value} NOT BETWEEN ${bind(value, lower)} AND ${bind(value, upper)}`;
}

/** Match when a subquery has rows. */
export function exists(query: SQLWrapper): SQL<boolean> {
    return sql<boolean>`EXISTS (${query})`;
}

/** Match when a subquery has no rows. */
export function notExists(query: SQLWrapper): SQL<boolean> {
    return sql<boolean>`NOT EXISTS (${query})`;
}

/** Order ascending. */
export function asc(value: SQLWrapper): SQL {
    return sql`${value} ASC`;
}

/** Order descending. */
export function desc(value: SQLWrapper): SQL {
    return sql`${value} DESC`;
}

/** Count rows, or the present values of an expression. */
export function count(value?: SQLWrapper): SQL<number> {
    return (value === undefined ? sql`count(*)` : sql`count(${value})`).mapWith(countOf);
}

/** Count distinct present values. */
export function countDistinct(value: SQLWrapper): SQL<number> {
    return sql`count(DISTINCT ${value})`.mapWith(countOf);
}

/** Sum numbers, as exact decimal text, missing over no rows. */
export function sum(value: SQLWrapper): SQL<string | null> {
    return sql`sum(${value})`.mapWith(nullableText);
}

/** Average numbers, as decimal text, missing over no rows. */
export function avg(value: SQLWrapper): SQL<string | null> {
    return sql`avg(${value})`.mapWith(nullableText);
}

/** Return the maximum column value, decoded by the column. */
export function max<Value extends ColumnValue>(value: Column<Value>): SQL<Value | null>;
/** Return the maximum expression value. */
export function max(value: SQLWrapper): SQL<string | null>;
/** Return the maximum value, whose signatures above type it by its operand. */
export function max(value: SQLWrapper): SQL {
    return extreme("max", value);
}

/** Return the minimum column value, decoded by the column. */
export function min<Value extends ColumnValue>(value: Column<Value>): SQL<Value | null>;
/** Return the minimum expression value. */
export function min(value: SQLWrapper): SQL<string | null>;
/** Return the minimum value, whose signatures above type it by its operand. */
export function min(value: SQLWrapper): SQL {
    return extreme("min", value);
}

/** Join predicates with one operator in nested flat chains. */
export function combine(predicates: readonly SQLWrapper[], operator: "AND" | "OR"): SQL<boolean> {
    // join short lists flat and longer ones as chains of chains
    const joiner = sql.raw(` ${operator} `);
    if (predicates.length <= CHAIN_TERMS) {
        return sql<boolean>`(${sql.join(predicates, joiner)})`;
    }
    const chains: SQL<boolean>[] = [];
    for (let start = 0; start < predicates.length; start += CHAIN_TERMS) {
        chains.push(combine(predicates.slice(start, start + CHAIN_TERMS), operator));
    }

    return combine(chains, operator);
}

/** Compare an operand with a value bound through its column, or with a fragment. */
function compare(left: SQLWrapper, operator: string, right: unknown): SQL<boolean> {
    return sql<boolean>`${left} ${sql.raw(operator)} ${bind(left, right)}`;
}

/** Bind a value through the operand's column, keeping fragments as they are. */
function bind(operand: SQLWrapper, value: unknown): SQLWrapper {
    // keep fragments, bind values through the operand's column
    if (isSQLWrapper(value)) {
        return value;
    }

    return new SQL([new Parameter(value, Column.is(operand) ? operand : undefined)]);
}

/** Match a value against a list or subquery, deciding an empty list without SQL. */
function membership(
    value: SQLWrapper,
    operator: "IN" | "NOT IN",
    candidates: readonly unknown[] | SQLWrapper,
    empty: SQL,
): SQL<boolean> {
    // decide an empty list, and bind each listed value through the operand's column
    if (isSQLWrapper(candidates)) {
        return sql<boolean>`${value} ${sql.raw(operator)} (${candidates})`;
    } else if (candidates.length === 0) {
        return sql<boolean>`${empty}`;
    }
    const bound = candidates.map((candidate) => bind(value, candidate));

    return sql<boolean>`${value} ${sql.raw(operator)} (${sql.join(bound, sql.raw(", "))})`;
}

/** Read the extreme of an operand, decoded by its column when it is one and as text otherwise. */
function extreme(name: "max" | "min", value: SQLWrapper): SQL {
    const aggregate = sql`${sql.raw(name)}(${value})`;

    return Column.is(value) ? aggregate.mapWith(value) : aggregate.mapWith(nullableText);
}

/** Read a count's driver value: a number, an exact integer or PostgreSQL's decimal text. */
function countOf(value: unknown): number {
    const counted = typeof value === "string" || typeof value === "bigint" ? Number(value) : value;
    if (typeof counted !== "number" || !Number.isSafeInteger(counted)) {
        throw new TypeError("a count returned a value that is no safe integer");
    }

    return counted;
}

/** Read an aggregate's driver value as text, missing as null. */
function nullableText(value: unknown): string | null {
    if (value === null) {
        return null;
    } else if (
        typeof value === "string" ||
        typeof value === "number" ||
        typeof value === "bigint"
    ) {
        return value.toString();
    }

    throw new TypeError("an aggregate returned a value that is no number");
}
