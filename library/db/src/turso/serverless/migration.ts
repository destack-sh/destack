import { migrate as migrateDatabase } from "drizzle-orm/tursodatabase-serverless/migrator";
import type { TursoDatabaseServerlessDatabase } from "drizzle-orm/tursodatabase-serverless";
import type { AnyRelations } from "drizzle-orm/relations";
import type { MigrationConfig } from "drizzle-orm/migrator";

/** Apply generated SQL migrations to a remote Turso database. */
export async function migrate<T extends AnyRelations>(
    database: TursoDatabaseServerlessDatabase<T>,
    options: MigrationConfig,
): Promise<void> {
    const result = await migrateDatabase(database, options);
    if (result !== undefined) throw new Error(`Database migration failed: ${result.exitCode}.`);
}
