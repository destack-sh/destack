import { type SQL, sql } from "drizzle-orm";
import { DatabaseCatalog } from "../../inspect/catalog.ts";
export { DatabaseCatalog, DatabaseObject } from "../../inspect/catalog.ts";

/** Read persisted objects in user schemas through one catalog snapshot. */
export async function inspectDatabase(database: {
    /** Execute SQL using the caller's connection or transaction. */
    execute(query: SQL): PromiseLike<Iterable<unknown>>;
}): Promise<DatabaseCatalog> {
    // inspect relation and trigger definitions without including PostgreSQL's internal schemas
    const rows = await database.execute(sql`
        SELECT kind, schema, name, "table", sql FROM (
            SELECT CASE c.relkind
                WHEN 'i' THEN 'index' WHEN 'I' THEN 'index'
                WHEN 'v' THEN 'view' WHEN 'm' THEN 'materializedView'
                WHEN 'S' THEN 'sequence' ELSE 'table' END AS kind,
                n.nspname AS schema, c.relname AS name,
                COALESCE(t.relname, c.relname) AS "table",
                CASE WHEN c.relkind IN ('i', 'I') THEN pg_get_indexdef(c.oid)
                     WHEN c.relkind IN ('v', 'm') THEN pg_get_viewdef(c.oid, true)
                     ELSE NULL END AS sql
            FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
            LEFT JOIN pg_index i ON i.indexrelid = c.oid
            LEFT JOIN pg_class t ON t.oid = i.indrelid
            WHERE c.relkind IN ('r', 'p', 'i', 'I', 'v', 'm', 'S')
                AND n.nspname !~ '^pg_' AND n.nspname <> 'information_schema'
            UNION ALL
            SELECT 'trigger', n.nspname, t.tgname, c.relname, pg_get_triggerdef(t.oid)
            FROM pg_trigger t JOIN pg_class c ON c.oid = t.tgrelid
            JOIN pg_namespace n ON n.oid = c.relnamespace
            WHERE NOT t.tgisinternal
                AND n.nspname !~ '^pg_' AND n.nspname <> 'information_schema'
        ) objects ORDER BY kind, schema, name
    `);

    return DatabaseCatalog.parse({ dialect: "postgresql", objects: Array.from(rows) });
}
