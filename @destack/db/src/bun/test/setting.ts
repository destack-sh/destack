import { defineTable } from "../../table/table.ts";
import { text } from "../../table/column.ts";

/** A logged setting without a scope column, which the channel test and its child process write. */
export const setting = defineTable(
    "setting",
    { name: text("name").primaryKey(), value: text("value").notNull() },
    { log: {} },
);
