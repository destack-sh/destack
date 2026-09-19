import type { Migration } from "./migration.ts";
import type { MigrationDescription } from "../inspect/migration.ts";

/** Describe committed SQL without executing it. */
export function describeMigration(migration: Migration): MigrationDescription {
    return {
        name: migration.name,
        checksum: migration.checksum,
    };
}
