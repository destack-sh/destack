import { type SQL, sql } from "drizzle-orm";
import { MigrationRecord } from "../../inspect/migration.ts";
import type { DatabaseSchema } from "../../schema/schema.ts";
import { MigrationHistory } from "../../migration/history.ts";

/** Read applied migrations through the current PostgreSQL search path. */
export async function inspectMigrations(database: {
    /** Execute SQL using the caller's connection or transaction. */
    execute(query: SQL): PromiseLike<Iterable<unknown>>;
}, definition: DatabaseSchema): Promise<MigrationRecord[]> {
    // resolve the history table without creating database objects
    const history = new MigrationHistory([], definition.name);
    const tables = Array.from(
        await database.execute(sql`
        SELECT 1 FROM pg_class WHERE oid = to_regclass(quote_ident(${history.name})) AND relkind = 'r'
    `),
    );
    if (tables.length === 0) return [];

    const records = await database.execute(sql`
        SELECT name, checksum, applied_at AS "appliedAt"
        FROM ${history.table} ORDER BY id
    `);

    return Array.from(records, (record) => MigrationRecord.parse(record));
}
