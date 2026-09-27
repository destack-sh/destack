import * as drizzle from "drizzle-orm";
import { sql, type SQL, type SQLWrapper } from "drizzle-orm";
import type { Column } from "../table/column.ts";

/**
 * The most predicates one flat chain joins.
 *
 * Turso refuses expressions nested deeper than 100, and a chain nests one level per predicate.
 */
export const CHAIN_TERMS = 90;

/** A comparison between a typed field and a value or SQL expression. */
export interface Comparison {
    /** Compare values using the field's application type. */
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
/** Combine optional predicates when at least one is present. */
export function and(...conditions: (SQLWrapper | undefined)[]): SQL | undefined;
/** Combine the present predicates with AND. */
export function and(...conditions: (SQLWrapper | undefined)[]): SQL | undefined {
    const present = conditions.filter((condition) => condition !== undefined);

    return present.length === 0 ? undefined : combine(present, "AND");
}

/** Require at least one supplied predicate. */
export function or(first: SQLWrapper, ...rest: (SQLWrapper | undefined)[]): SQL;
/** Combine optional predicates when at least one is present. */
export function or(...conditions: (SQLWrapper | undefined)[]): SQL | undefined;
/** Combine the present predicates with OR. */
export function or(...conditions: (SQLWrapper | undefined)[]): SQL | undefined {
    const present = conditions.filter((condition) => condition !== undefined);

    return present.length === 0 ? undefined : combine(present, "OR");
}

/**
 * Match a value equal to one of several, or to a row a subquery selects: a flat chain of equalities, which SQLite plans as index lookups, up to the chain's limit, and an IN list beyond.
 *
 * Turso plans nested chains as scans, so a list longer than a chain holds becomes one IN list instead.
 */
export function inArray(value: SQLWrapper, candidates: readonly unknown[] | SQLWrapper): SQL {
    // match the rows a subquery selects
    if (!Array.isArray(candidates)) {
        return drizzle.inArray(value as drizzle.Column, candidates as SQLWrapper);
    }
    // match nothing for no candidates
    else if (candidates.length === 0) {
        return sql`false`;
    }

    // chain the equalities flat within the chain's limit, and list the candidates beyond it
    return candidates.length <= CHAIN_TERMS
        ? combine(
              candidates.map((candidate) => drizzle.eq(value, candidate)),
              "OR",
          )
        : drizzle.inArray(value as drizzle.Column, candidates as unknown[]);
}

/**
 * Join predicates with one operator: a flat chain, which SQLite plans as index lookups, up to the chain's limit, and chains of chains beyond.
 *
 * Turso refuses a chain nested deeper than 100, and plans nested chains as scans, so callers read more than a chain holds in several statements.
 */
export function combine(predicates: readonly SQLWrapper[], operator: "AND" | "OR"): SQL {
    // join a chain flat, and longer lists as a chain of flat chains
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
