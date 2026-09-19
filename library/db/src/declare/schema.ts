import { ResourceName } from "@destack/resource";
import type { SQLiteTable } from "drizzle-orm/sqlite-core";

/** A named collection of tables and its committed migration history. */
export interface DatabaseSchema<
    Tables extends Record<string, SQLiteTable> = Record<string, SQLiteTable>,
> {
    /** The stable schema name, unique within its database. */
    readonly name: string;
    /** The tables managed by this schema. */
    readonly tables: Tables;
    /** The directory containing committed Drizzle migrations. */
    readonly migrations: URL;
}

/** Declare a schema without opening a database or applying migrations. */
export function defineDatabaseSchema<Tables extends Record<string, SQLiteTable>>(
    definition: DatabaseSchema<Tables>,
): DatabaseSchema<Tables> {
    ResourceName.parse(definition.name);

    return definition;
}
