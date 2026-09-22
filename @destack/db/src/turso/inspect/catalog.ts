import { type SQL, sql } from "drizzle-orm";
import { DatabaseCatalog } from "../../inspect/catalog.ts";
export { DatabaseCatalog, DatabaseObject } from "../../inspect/catalog.ts";

/** Read the persisted schema through a database connection or transaction. */
export async function inspectDatabase(database: {
    /** Execute a query using the connection's existing transaction. */
    all(query: SQL): unknown[] | PromiseLike<unknown[]>;
}): Promise<DatabaseCatalog> {
    // read the full catalog in one statement, including implicit and virtual objects
    const objects = await database.all(sql`
        SELECT type AS kind, name, tbl_name AS "table", sql
        FROM sqlite_schema
        ORDER BY type, name
    `);

    return DatabaseCatalog.parse({ dialect: "sqlite", objects });
}
