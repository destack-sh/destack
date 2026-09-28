import * as drizzle from "drizzle-orm";
import { SQL, type SQLWrapper } from "drizzle-orm";
import { Column } from "../table/column.ts";

/** Return the maximum column value, decoded by the column. */
export function max<Value>(expression: Column<Value>): SQL<Value | null>;
/** Return the maximum expression value. */
export function max(expression: SQLWrapper): SQL<string | null>;
/** Return the maximum value. */
export function max(expression: SQLWrapper): SQL<unknown> {
    const aggregate = drizzle.max(expression);

    return expression instanceof Column ? aggregate.mapWith(expression) : aggregate;
}

/** Return the minimum column value, decoded by the column. */
export function min<Value>(expression: Column<Value>): SQL<Value | null>;
/** Return the minimum expression value. */
export function min(expression: SQLWrapper): SQL<string | null>;
/** Return the minimum value. */
export function min(expression: SQLWrapper): SQL<unknown> {
    const aggregate = drizzle.min(expression);

    return expression instanceof Column ? aggregate.mapWith(expression) : aggregate;
}
