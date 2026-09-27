import type { SQLWrapper } from "drizzle-orm";
import type { ColumnDefinition } from "../table/column.ts";
import type { Condition } from "./condition.ts";
import type { Expression, Rollup } from "./expression.ts";

/** Values computed from a row, by name, which orders and conditions read like columns. */
export type Computed = Readonly<Record<string, Expression>>;

/** The names a table's conditions, orders and computed values read beyond its columns. */
export interface Namespace {
    /** The values computed from each row, by name. */
    readonly computed: Computed;
    /** Render whether a related row meets a condition. */
    readonly exists?: (via: string, where: Condition | undefined) => SQLWrapper;
    /** Read a column of the one row a relation relates: its definition, and its value in SQL, missing without a related row. */
    readonly lookup?: (
        via: string,
        column: string,
    ) => { readonly definition: ColumnDefinition; readonly value: SQLWrapper };
    /** Measure the rows a relation relates that meet a condition: the measured column's definition, absent for a count, and the measure in SQL. */
    readonly rollup?: (
        measure: Rollup,
        via: string,
        column: string | undefined,
        where: Condition | undefined,
    ) => { readonly definition?: ColumnDefinition; readonly value: SQLWrapper };
}
