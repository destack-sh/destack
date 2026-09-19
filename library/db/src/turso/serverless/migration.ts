import { readMigrations } from "../../migration/read.ts";
import type { DatabaseSchema } from "../../declare/schema.ts";
import { applyMigrations } from "../../sqlite/migration.ts";
import type { AnyRelations } from "drizzle-orm/relations";
import type { Database } from "./connection.ts";

/** Apply committed SQL and migration history in one write transaction. */
export async function migrate<Relations extends AnyRelations>(
    database: Database<Relations>,
    definition: DatabaseSchema,
): Promise<void> {
    const migrations = await readMigrations(definition);
    await database.transaction(async (transaction) => {
        await applyMigrations(transaction, migrations, definition.name);
    }, { behavior: "immediate" });
}
