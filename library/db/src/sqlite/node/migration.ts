import { migrate as migrateDatabase } from "drizzle-orm/node-sqlite/migrator";
import type { NodeSQLiteDatabase } from "drizzle-orm/node-sqlite";
import type { AnyRelations } from "drizzle-orm/relations";
import type { MigrationConfig } from "drizzle-orm/migrator";

/** Apply generated SQL migrations and fail when migration initialization fails. */
export function migrate<T extends AnyRelations>(
    database: NodeSQLiteDatabase<T>,
    options: MigrationConfig,
): void {
    const result = migrateDatabase(database, options);
    if (result !== undefined) throw new Error(`Database migration failed: ${result.exitCode}.`);
}
