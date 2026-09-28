import type { Dialect } from "../dialect/dialect.ts";
import type { TableState } from "./state.ts";

/** The triggers a feature generates on a table, reinstalled around every plan. */
export interface Triggers {
    /** Create the table's triggers. */
    install(state: TableState, dialect: Dialect): readonly string[];
    /** Drop the table's triggers. */
    remove(state: TableState, dialect: Dialect): readonly string[];
}
