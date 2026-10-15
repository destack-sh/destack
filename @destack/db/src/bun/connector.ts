import type { DatabaseConnector } from "../declare/database.ts";
import { bunSqliteConnector } from "./sqlite.ts";

/** The connectors opening databases on Bun: SQLite files. */
export const connectors: Readonly<Record<string, DatabaseConnector>> = {
    sqlite: bunSqliteConnector,
};
