import { type SQL, sql } from "drizzle-orm";
import { MigrationRecord } from "../../inspect/migration.ts";
import type { DatabaseSchema } from "../../schema/schema.ts";
import { MigrationHistory } from "../../migration/history.ts";

/** Read one schema's applied migrations, or an empty history before initialization. */
export async function inspectMigrations(
    database: {
        /** Execute a query using the connection's existing transaction. */
        all(query: SQL): unknown[] | PromiseLike<unknown[]>;
    },
    definition: DatabaseSchema,
): Promise<MigrationRecord[]> {
    // identify an initialized history without creating database objects
    const history = new MigrationHistory([], definition.name);
    const tables = await database.all(sql`
        SELECT name FROM sqlite_schema WHERE type = 'table' AND name = ${history.name}
    `);
    if (tables.length === 0) {
        return [];
    }

    // expose applied history through the same description on every driver
    const records = await database.all(sql`
        SELECT name, checksum, applied_at AS appliedAt
        FROM ${history.table} ORDER BY id
    `);

    return records.map((record) => MigrationRecord.parse(record));
}
