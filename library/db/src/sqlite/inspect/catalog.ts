import { defineSchema, schema } from "@destack/schema";
import { type SQL, sql } from "drizzle-orm";

/** A persisted SQLite schema object. */
export const CatalogEntry = defineSchema(schema.object({
    /** The SQLite object type. */
    kind: schema.enum(["table", "index", "view", "trigger"]),
    /** The object's SQL name. */
    name: schema.string().min(1),
    /** The table or view associated with the object. */
    table: schema.string().min(1),
    /** The stored CREATE statement, absent for implicit indexes. */
    sql: schema.string().nullable(),
}));
/** A persisted SQLite schema object. */
export type CatalogEntry = schema.Infer<typeof CatalogEntry>;

/** The persisted schema returned by one consistent SQLite query. */
export const DatabaseSnapshot = defineSchema(schema.object({
    /** The SQL dialect of the inspected catalog. */
    dialect: schema.literal("sqlite"),
    /** Tables, indexes, views, and triggers in stable order. */
    objects: schema.array(CatalogEntry),
}));
/** A snapshot of the persisted database schema. */
export type DatabaseSnapshot = schema.Infer<typeof DatabaseSnapshot>;

/** Read the persisted schema through a database connection or transaction. */
export async function inspectDatabase(database: {
    /** Execute a query using the connection's existing transaction. */
    all(query: SQL): unknown[] | PromiseLike<unknown[]>;
}): Promise<DatabaseSnapshot> {
    // read the full catalog in one statement, including implicit and virtual objects
    const objects = await database.all(sql`
        SELECT type AS kind, name, tbl_name AS "table", sql
        FROM sqlite_schema
        ORDER BY type, name
    `);

    return DatabaseSnapshot.parse({ dialect: "sqlite", objects });
}
