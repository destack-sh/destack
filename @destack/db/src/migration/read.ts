import { createHash } from "node:crypto";
import { readdir, readFile } from "node:fs/promises";
import { fileURLToPath, pathToFileURL } from "node:url";
import { join } from "node:path";
import type { Migration } from "./migration.ts";
import type { DatabaseSchema } from "../schema/schema.ts";
import { DatabaseError } from "../error/index.ts";
import type { Dialect } from "../dialect/dialect.ts";

/** Read committed SQL and TypeScript migrations without executing them. */
export async function readMigrations(
    definition: Pick<DatabaseSchema, "migrations">,
    dialect: Dialect,
): Promise<Migration[]> {
    // order migration directories independently of filesystem enumeration
    const directory = join(fileURLToPath(definition.migrations), dialect);
    const entries = await readdir(directory, { withFileTypes: true });
    const names = entries
        .filter((entry) => entry.isDirectory())
        .map((entry) => entry.name)
        .sort();
    const migrations: Migration[] = [];

    // read each file once and retain the exact SQL used to compute its checksum
    for (const name of names) {
        if (!/^\d{14}_.+$/.test(name)) {
            throw new DatabaseError("INVALID_MIGRATION", `invalid migration directory: ${name}`);
        }
        const source = await readFile(join(directory, name, "migration.sql"), "utf8");
        let transformation: string | undefined;
        const path = join(directory, name, "migration.ts");
        try {
            transformation = await readFile(path, "utf8");
        } catch (error) {
            if (!(error instanceof Error && "code" in error && error.code === "ENOENT")) {
                throw error;
            }
        }

        // include executable transformations in the immutable migration checksum
        const checksum = createHash("sha256").update(source);
        if (transformation !== undefined) {
            checksum.update("\0migration.ts\0").update(transformation);
        }
        const digest = checksum.digest("hex");

        // load only pending transformations after the migration transaction has acquired its lock
        let apply: Migration["apply"];
        if (transformation !== undefined) {
            const url = pathToFileURL(path);
            url.searchParams.set("checksum", digest);
            apply = async (database) => {
                const module = await import(url.href);
                if (typeof module.migrate !== "function") {
                    throw new DatabaseError("INVALID_MIGRATION", `missing migrate export: ${name}`);
                }

                await module.migrate(database);
            };
        }
        migrations.push({
            name,
            checksum: digest,
            statements: source.split("--> statement-breakpoint"),
            apply,
        });
    }

    return migrations;
}
