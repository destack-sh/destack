import type { SQLiteTable } from "drizzle-orm/sqlite-core";
import { DatabaseDescription } from "../../inspect/index.ts";
import { describeTable } from "./table.ts";

/** Describe a database declaration without opening a connection. */
export function describeDatabase(
    name: string,
    tables: Record<string, SQLiteTable>,
): DatabaseDescription {
    const descriptions = Object.values(tables).map(describeTable);
    const names = new Set<string>();

    // reject duplicate table names before building a resource description
    for (const table of descriptions) {
        if (names.has(table.name)) throw new TypeError(`Duplicate SQL table: ${table.name}.`);
        names.add(table.name);
    }

    return DatabaseDescription.parse({
        name,
        kind: "database",
        version: 1,
        spec: { dialect: "sqlite", tables: descriptions },
    });
}
