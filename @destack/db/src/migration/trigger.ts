import type { Dialect } from "../dialect/dialect.ts";
import type { TableState } from "./state.ts";

/** The triggers a feature generates on a table, reinstalled around every plan. */
export interface Triggers {
    /** Create the table's triggers, with the log's relations in the database's namespace. */
    readonly install: (
        state: TableState,
        dialect: Dialect,
        namespace?: string,
    ) => readonly string[];
    /** Drop the table's triggers. */
    readonly remove: (state: TableState, dialect: Dialect) => readonly string[];
}
