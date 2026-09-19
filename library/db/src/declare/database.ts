import { defineSchema, schema } from "@destack/schema";
import { defineResourceSchema, Resource } from "@destack/resource";
import type { SQLiteAsyncDatabase } from "drizzle-orm/sqlite-core";

/** The SQL dialect used by a database. */
export const DatabaseSpec = defineSchema(schema.object({
    /** The dialect used by queries and migrations. */
    dialect: schema.literal("sqlite"),
}));
/** The SQL dialect used by a database. */
export type DatabaseSpec = schema.Infer<typeof DatabaseSpec>;

/** A named database dependency. */
export const DatabaseDeclaration = defineResourceSchema("database", 1, DatabaseSpec);
/** A named database dependency. */
export type DatabaseDeclaration = schema.Infer<typeof DatabaseDeclaration>;

/** The SQLite query and transaction API shared by local and remote drivers. */
export type DatabaseConnection = SQLiteAsyncDatabase<"async", unknown>;

/** An inert database declaration with invocation-scoped connection access. */
export type Database = Resource<DatabaseConnection, DatabaseDeclaration>;

/** Declare a database dependency. */
export function defineDatabase(
    declaration: Omit<DatabaseDeclaration, "kind" | "version">,
): Database {
    const description = DatabaseDeclaration.parse({ ...declaration, kind: "database", version: 1 });

    return new Resource<DatabaseConnection, DatabaseDeclaration>(description);
}
