export * from "./table.ts";
export {
    alias,
    check,
    foreignKey,
    getTableConfig,
    getViewConfig,
    index,
    primaryKey,
    sqliteView as view,
    unique,
    uniqueIndex,
} from "drizzle-orm/sqlite-core";
export type { SQLiteTable as Table } from "drizzle-orm/sqlite-core";
