import { createHash } from "node:crypto";
import { readdir, readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { join } from "node:path";
import type { Migration } from "./migration.ts";
import type { DatabaseSchema } from "../declare/schema.ts";
import { DatabaseError } from "../error/index.ts";

/** Read committed Drizzle SQL and checksums without opening a database. */
export async function readMigrations(definition: DatabaseSchema): Promise<Migration[]> {
    // order migration directories independently of filesystem enumeration
    const directory = fileURLToPath(definition.migrations);
    const entries = await readdir(directory, { withFileTypes: true });
    const names = entries.filter((entry) => entry.isDirectory()).map((entry) => entry.name).sort();
    const migrations: Migration[] = [];

    // read each file once and retain the exact SQL used to compute its checksum
    for (const name of names) {
        if (!/^\d{14}_.+$/.test(name)) {
            throw new DatabaseError(
                "INVALID_MIGRATION",
                `Invalid migration directory: ${name}.`,
            );
        }
        const source = await readFile(join(directory, name, "migration.sql"), "utf8");
        migrations.push({
            name,
            checksum: createHash("sha256").update(source).digest("hex"),
            statements: source.split("--> statement-breakpoint"),
        });
    }

    return migrations;
}
