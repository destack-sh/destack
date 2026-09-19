import { type SQL, sql } from "drizzle-orm";
import type { Migration } from "../migration/migration.ts";
import type { MigrationDescription } from "../inspect/migration.ts";
import { MigrationHistory } from "../migration/history.ts";
import { DatabaseError } from "../error/index.ts";

/** Apply committed SQL through an existing database transaction. */
export async function applyMigrations(
    transaction: {
        /** Execute one SQL statement. */
        run(statement: SQL): Promise<unknown>;
        /** Read the applied migration history. */
        all<Row>(statement: SQL): Promise<Row[]>;
    },
    migrations: readonly Migration[],
    name: string,
): Promise<void> {
    // compare history while holding the transaction's write lock
    const history = new MigrationHistory(migrations, name);
    await transaction.run(history.create());
    await transaction.run(history.lock());
    const applied = await transaction.all<MigrationDescription>(history.select());
    const pending = history.pending(applied);

    // apply SQL and history together
    for (const migration of pending) {
        try {
            for (const statement of migration.statements) await transaction.run(sql.raw(statement));
            await transaction.run(history.insert(migration));
        } catch (cause) {
            throw new DatabaseError("MIGRATION_FAILED", `Migration failed: ${migration.name}.`, {
                cause,
            });
        }
    }
}
