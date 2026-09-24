import * as drizzle from "drizzle-orm";
import { SQL, type SQLWrapper } from "drizzle-orm";
import { Column } from "../table/column.ts";

/** Return the maximum column value using its application decoder. */
export function max<Value>(expression: Column<Value>): SQL<Value | null>;
/** Return the maximum expression value using Drizzle conventions. */
export function max(expression: SQLWrapper): SQL<string | null>;
/** Return the maximum value, decoded by the column when one is given. */
export function max(expression: SQLWrapper): SQL<unknown> {
    const aggregate = drizzle.max(expression);

    return expression instanceof Column ? aggregate.mapWith(expression) : aggregate;
}

/** Return the minimum column value using its application decoder. */
export function min<Value>(expression: Column<Value>): SQL<Value | null>;
/** Return the minimum expression value using Drizzle conventions. */
export function min(expression: SQLWrapper): SQL<string | null>;
/** Return the minimum value, decoded by the column when one is given. */
export function min(expression: SQLWrapper): SQL<unknown> {
    const aggregate = drizzle.min(expression);

    return expression instanceof Column ? aggregate.mapWith(expression) : aggregate;
}
