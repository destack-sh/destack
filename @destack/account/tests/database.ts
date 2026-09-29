import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { directoryTables } from "@destack/directory";
import type { Table } from "@destack/db";
import { accountTables } from "../src/stack/index.ts";

/** The tables of a test global database: the accounts and every object server's own. */
export const globalTables: readonly Table[] = [...accountTables, ...directoryTables];

/** Open an isolated, migrated account database, PostgreSQL when configured. */
export function openAccountDatabase(): Promise<TestDatabase> {
    return TestDatabase.create(TEST_DIALECTS.at(-1)!, globalTables, { isMigrated: true });
}
