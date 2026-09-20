import type { DatabaseConnection } from "../database/connection.ts";
import type { DatabaseSchema } from "../schema/schema.ts";
import { assertNever } from "../error/error.ts";
import * as turso from "../turso/migration.ts";
import * as postgres from "../postgres/migration.ts";
import { readMigrations } from "./read.ts";

/** Apply a schema's committed history in one locked transaction. */
export async function migrate(
    database: DatabaseConnection,
    definition: Pick<DatabaseSchema, "name" | "migrations">,
): Promise<void> {
    // read SQL before reserving the connection and taking migration locks
    const migrations = await readMigrations(definition, database.connection.native.dialect);

    // read committed observes history changes made before the advisory lock was acquired
    await database.transaction(async (transaction) => {
        const connection = transaction.connection;
        if (connection.native.dialect === "sqlite") {
            await turso.applyMigrations(connection.native.database, migrations, definition.name);
        } else if (connection.native.dialect === "postgresql") {
            await postgres.applyMigrations(connection.native.database, migrations, definition.name);
        } else {
            assertNever(connection.native);
        }
    }, { isolationLevel: "read committed" });
}
