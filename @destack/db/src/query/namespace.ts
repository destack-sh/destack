import type { SQLWrapper } from "../sql/index.ts";
import type { ColumnDefinition } from "../table/column.ts";
import { TABLE, type Table } from "../table/table.ts";
import type { Condition } from "./condition.ts";
import type { Expression, Rollup } from "../expression/expression.ts";

/** The computed values of a row, by name. */
export type Extras = Readonly<Record<string, Expression>>;

/** The names a table's conditions, orders and computed values read beyond its columns. */
export interface Namespace {
    /** The values computed from each row, by name. */
    readonly extras: Extras;
    /** Render whether a related row meets a condition. */
    readonly exists?: (via: string, where: Condition) => SQLWrapper;
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
        where: Condition,
    ) => { readonly definition?: ColumnDefinition; readonly value: SQLWrapper };
}

/** The names a table's conditions read. */
export const Namespace = {
    /** List the fields a condition compares on a table's rows: its columns and computed values. */
    fields(table: Table, namespace: Namespace = { extras: {} }): ReadonlySet<string> {
        return new Set([...Object.keys(table[TABLE].columns), ...Object.keys(namespace.extras)]);
    },
};
