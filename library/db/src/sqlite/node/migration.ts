import { readMigrations } from "../../migration/read.ts";
import type { DatabaseSchema } from "../../declare/schema.ts";
import { sql } from "drizzle-orm";
import { MigrationHistory } from "../../migration/history.ts";
import type { MigrationDescription } from "../../inspect/migration.ts";
import { DatabaseError } from "../../error/index.ts";
import type { NodeSQLiteDatabase } from "drizzle-orm/node-sqlite";
import type { AnyRelations } from "drizzle-orm/relations";

/** Apply committed migrations and their history in one write transaction. */
export async function migrate<T extends AnyRelations>(
    database: NodeSQLiteDatabase<T>,
    definition: DatabaseSchema,
): Promise<void> {
    const migrations = await readMigrations(definition);
    const history = new MigrationHistory(migrations, definition.name);
    database.transaction((transaction) => {
        // serialize history inspection with schema changes
        transaction.run(history.create());
        transaction.run(history.lock());
        const applied = transaction.all<MigrationDescription>(history.select());
        const pending = history.pending(applied);

        // commit SQL and history together, preserving unrelated tables
        for (const migration of pending) {
            try {
                for (const statement of migration.statements) transaction.run(sql.raw(statement));
                transaction.run(history.insert(migration));
            } catch (cause) {
                throw new DatabaseError(
                    "MIGRATION_FAILED",
                    `Migration failed: ${migration.name}.`,
                    {
                        cause,
                    },
                );
            }
        }
    }, { behavior: "immediate" });
}
