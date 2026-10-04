import {
    boolean,
    Column,
    integer,
    numeric,
    type ColumnDefinition,
    type ColumnValue,
} from "../table/column.ts";
import { type Alias, isSQLWrapper, Parameter, SQL, sql, type SQLWrapper } from "./sql.ts";

/**
 * The most predicates one flat chain joins.
 *
 * 90 levels keep compound keys far below SQLite's expression depth limit of 1000.
 */
export const CHAIN_TERMS = 90;

/** A typed operand: a column reading the value, a fragment or a named fragment. */
type Operand<Value> =
    | Column<ColumnDefinition & { fromJson(value: unknown): Value }>
    | SQL<Value>
    | Alias<Value>;

/** A comparison of a typed operand with a value or another fragment. */
export type Comparison = <Value>(
    left: Operand<Value>,
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
/**
 * Combine the present predicates with AND.
 *
 * @construct a required first predicate is present, so the combination is too.
 */
export function and(...conditions: (SQLWrapper | undefined)[]): SQL<boolean> | undefined {
    const present = conditions.filter((condition) => condition !== undefined);

    return present.length === 0 ? undefined : combine(present, "AND");
}

/** Require at least one supplied predicate. */
export function or(first: SQLWrapper, ...rest: (SQLWrapper | undefined)[]): SQL<boolean>;
/** Combine the present predicates with OR. */
export function or(...conditions: (SQLWrapper | undefined)[]): SQL<boolean> | undefined;
/**
 * Combine the present predicates with OR.
 *
 * @construct a required first predicate is present, so the combination is too.
 */
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
    value: Operand<Value>,
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
    value: Operand<Value>,
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
export function like(value: Operand<string>, pattern: string | SQLWrapper): SQL<boolean> {
    return sql<boolean>`${value} LIKE ${bind(value, pattern)}`;
}

/** Match text outside a LIKE pattern. */
export function notLike(value: Operand<string>, pattern: string | SQLWrapper): SQL<boolean> {
    return sql<boolean>`${value} NOT LIKE ${bind(value, pattern)}`;
}

/** Match a value within inclusive bounds. */
export function between<Value>(
    value: Operand<Value>,
    lower: NoInfer<Value> | SQLWrapper,
    upper: NoInfer<Value> | SQLWrapper,
): SQL<boolean> {
    return sql<boolean>`${value} BETWEEN ${bind(value, lower)} AND ${bind(value, upper)}`;
}

/** Match a value outside inclusive bounds. */
export function notBetween<Value>(
    value: Operand<Value>,
    lower: NoInfer<Value> | SQLWrapper,
    upper: NoInfer<Value> | SQLWrapper,
): SQL<boolean> {
    return sql<boolean>`${value} NOT BETWEEN ${bind(value, lower)} AND ${bind(value, upper)}`;
}

/** Match when a subquery has rows, selected as a boolean. */
export function exists(query: SQLWrapper): SQL<boolean> {
    return sql`EXISTS (${query})`.mapWith(boolean("exists").definition);
}

/** Match when a subquery has no rows, selected as a boolean. */
export function notExists(query: SQLWrapper): SQL<boolean> {
    return sql`NOT EXISTS (${query})`.mapWith(boolean("exists").definition);
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
    return (value === undefined ? sql`count(*)` : sql`count(${value})`).mapWith(
        integer("count").definition,
    );
}

/** Count distinct present values. */
export function countDistinct(value: SQLWrapper): SQL<number> {
    return sql`count(DISTINCT ${value})`.mapWith(integer("count").definition);
}

/** Sum numbers, as exact decimal text, missing over no rows. */
export function sum(value: SQLWrapper): SQL<string | null> {
    return sql`sum(${value})`.mapWith(numeric("aggregate").definition);
}

/** Average numbers, as decimal text, missing over no rows. */
export function avg(value: SQLWrapper): SQL<string | null> {
    return sql`avg(${value})`.mapWith(numeric("aggregate").definition);
}

/** Return the maximum column value, decoded by the column. */
export function max<Value extends ColumnValue>(
    value: Column<ColumnDefinition<Value>>,
): SQL<Value | null>;
/** Return the maximum expression value, decoded as the expression decodes. */
export function max<Value>(value: SQL<Value>): SQL<Value | null>;
/** Return the maximum of a column or expression, decoded as the operand decodes. */
export function max(value: SQLWrapper): SQL;
/**
 * Return the maximum value.
 *
 * @construct the maximum decodes as its operand does: a column by its definition, an expression by its decoder.
 */
export function max(value: SQLWrapper): SQL {
    return extreme("max", value);
}

/** Return the minimum column value, decoded by the column. */
export function min<Value extends ColumnValue>(
    value: Column<ColumnDefinition<Value>>,
): SQL<Value | null>;
/** Return the minimum expression value, decoded as the expression decodes. */
export function min<Value>(value: SQL<Value>): SQL<Value | null>;
/** Return the minimum of a column or expression, decoded as the operand decodes. */
export function min(value: SQLWrapper): SQL;
/**
 * Return the minimum value.
 *
 * @construct the minimum decodes as its operand does: a column by its definition, an expression by its decoder.
 */
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

    return Column.is(value)
        ? aggregate.mapWith(value.definition)
        : new SQL(aggregate.chunks, value.getSQL().decoder);
}
