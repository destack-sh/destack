import type { SQLWrapper } from "../sql/index.ts";
import type { ColumnDefinition } from "../table/column.ts";
import type { Condition } from "./condition.ts";
import type { Expression, Rollup } from "../expression/expression.ts";

/** The computed values of a row, by name. */
export type Computed = Readonly<Record<string, Expression>>;

/** The names a table's conditions, orders and computed values read beyond its columns. */
export interface Namespace {
    /** The values computed from each row, by name. */
    readonly computed: Computed;
    /** Render whether a related row meets a condition. */
    readonly exists?: (via: string, where: Condition | undefined) => SQLWrapper;
    /** Read a column of the related row: its definition and SQL value. */
    readonly lookup?: (
        via: string,
        column: string,
    ) => { readonly definition: ColumnDefinition; readonly value: SQLWrapper };
    /** Measure the related rows meeting a condition: the measured column and the SQL measure. */
    readonly rollup?: (
        measure: Rollup,
        via: string,
        column: string | undefined,
        where: Condition | undefined,
    ) => { readonly definition?: ColumnDefinition; readonly value: SQLWrapper };
}
