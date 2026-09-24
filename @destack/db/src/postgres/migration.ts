import { sql } from "drizzle-orm";
import type { PostgresJsDatabase } from "drizzle-orm/postgres-js";
import type { Migration } from "../migration/migration.ts";
import { DatabaseError } from "../error/index.ts";
import type { MigrationDescription } from "../inspect/migration.ts";
import { MigrationHistory } from "../migration/history.ts";
import type { DatabaseConnection } from "../database/connection.ts";

/** Apply committed SQL through an existing PostgreSQL transaction. */
export async function applyMigrations(
    transaction: Pick<PostgresJsDatabase, "execute">,
    migrations: readonly Migration[],
    name: string,
    database: DatabaseConnection,
): Promise<void> {
    // select the history table for this schema
    const history = new MigrationHistory(migrations, name);

    // serialize schema creation before the history table exists
    await transaction.execute(
        sql`SELECT pg_advisory_xact_lock(hashtextextended(${history.name}, 0))`,
    );
    await transaction.execute(history.create());
    const applied = Array.from(await transaction.execute<MigrationDescription>(history.select()));
    const pending = history.pending(applied);

    // commit each SQL statement and its history in the same transaction
    for (const [offset, migration] of pending.entries()) {
        try {
            for (const statement of migration.statements) {
                await transaction.execute(sql.raw(statement));
            }
            await migration.apply?.(database);
            await transaction.execute(history.insert(migration, applied.length + offset + 1));
        } catch (cause) {
            throw new DatabaseError("MIGRATION_FAILED", `Migration failed: ${migration.name}.`, {
                cause,
            });
        }
    }
}
