import { migrate as migrateDatabase } from "drizzle-orm/tursodatabase/migrator";
import type { TursoDatabaseDatabase } from "drizzle-orm/tursodatabase";
import type { AnyRelations } from "drizzle-orm/relations";
import type { MigrationConfig } from "drizzle-orm/migrator";

/** Apply generated SQL migrations to the native Turso database. */
export async function migrate<T extends AnyRelations>(
    database: TursoDatabaseDatabase<T>,
    options: MigrationConfig,
): Promise<void> {
    const result = await migrateDatabase(database, options);
    if (result !== undefined) throw new Error(`Database migration failed: ${result.exitCode}.`);
}
