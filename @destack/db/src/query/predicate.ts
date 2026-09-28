import * as drizzle from "drizzle-orm";
import { sql, type SQL, type SQLWrapper } from "drizzle-orm";
import type { Column } from "../table/column.ts";

/**
 * The most predicates one flat chain joins.
 *
 * 90 levels keep compound keys far below SQLite's expression depth limit of 1000.
 */
export const CHAIN_TERMS = 90;

/** A comparison of a typed field. */
export interface Comparison {
    /** Compare values by the field's type. */
    <Value>(
        left: Column<Value> | SQL<Value> | SQL.Aliased<Value>,
        right: NoInfer<Value> | SQLWrapper,
    ): SQL;
}

/** Compare equal values. */
export const eq: Comparison = drizzle.eq;
/** Compare unequal values. */
export const ne: Comparison = drizzle.ne;
/** Compare a greater value. */
export const gt: Comparison = drizzle.gt;
/** Compare a greater or equal value. */
export const gte: Comparison = drizzle.gte;
/** Compare a smaller value. */
export const lt: Comparison = drizzle.lt;
/** Compare a smaller or equal value. */
export const lte: Comparison = drizzle.lte;

/** Require every supplied predicate. */
export function and(first: SQLWrapper, ...rest: (SQLWrapper | undefined)[]): SQL;
/** Combine the present predicates with AND. */
export function and(...conditions: (SQLWrapper | undefined)[]): SQL | undefined;
/** Combine the present predicates with AND. */
export function and(...conditions: (SQLWrapper | undefined)[]): SQL | undefined {
    const present = conditions.filter((condition) => condition !== undefined);

    return present.length === 0 ? undefined : combine(present, "AND");
}

/** Require at least one supplied predicate. */
export function or(first: SQLWrapper, ...rest: (SQLWrapper | undefined)[]): SQL;
/** Combine the present predicates with OR. */
export function or(...conditions: (SQLWrapper | undefined)[]): SQL | undefined;
/** Combine the present predicates with OR. */
export function or(...conditions: (SQLWrapper | undefined)[]): SQL | undefined {
    const present = conditions.filter((condition) => condition !== undefined);

    return present.length === 0 ? undefined : combine(present, "OR");
}

/** Match a value in a list or subquery. */
export function inArray(value: SQLWrapper, candidates: readonly unknown[] | SQLWrapper): SQL {
    // match nothing for no candidates
    if (Array.isArray(candidates) && candidates.length === 0) {
        return sql`false`;
    }

    return drizzle.inArray(value as drizzle.Column, candidates as unknown[] | SQLWrapper);
}

/** Join predicates with one operator in nested flat chains. */
export function combine(predicates: readonly SQLWrapper[], operator: "AND" | "OR"): SQL {
    // join short lists flat and longer ones as chains of chains
    const joiner = sql.raw(` ${operator} `);
    if (predicates.length <= CHAIN_TERMS) {
        return sql`(${sql.join([...predicates], joiner)})`;
    }
    const chains: SQL[] = [];
    for (let start = 0; start < predicates.length; start += CHAIN_TERMS) {
        chains.push(combine(predicates.slice(start, start + CHAIN_TERMS), operator));
    }

    return combine(chains, operator);
}
