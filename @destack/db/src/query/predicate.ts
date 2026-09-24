import * as drizzle from "drizzle-orm";
import type { SQL, SQLWrapper } from "drizzle-orm";
import type { Column } from "../table/column.ts";

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
    return drizzle.and(...conditions);
}

/** Require at least one supplied predicate. */
export function or(first: SQLWrapper, ...rest: (SQLWrapper | undefined)[]): SQL;
/** Combine optional predicates when at least one is present. */
export function or(...conditions: (SQLWrapper | undefined)[]): SQL | undefined;
/** Combine the present predicates with OR. */
export function or(...conditions: (SQLWrapper | undefined)[]): SQL | undefined {
    return drizzle.or(...conditions);
}
