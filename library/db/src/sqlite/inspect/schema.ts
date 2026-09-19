import type { DatabaseSchema } from "../../declare/schema.ts";
import { DatabaseSchemaDescription } from "../../inspect/index.ts";
import { describeTable } from "./table.ts";

/** Describe a named schema without opening a connection. */
export function describeSchema(definition: DatabaseSchema): DatabaseSchemaDescription {
    const descriptions = Object.values(definition.tables).map(describeTable);
    const names = new Set<string>();

    // reject duplicate table names within the managed schema
    for (const table of descriptions) {
        if (names.has(table.name)) throw new TypeError(`Duplicate SQL table: ${table.name}.`);
        names.add(table.name);
    }

    return DatabaseSchemaDescription.parse({
        name: definition.name,
        version: 1,
        dialect: "sqlite",
        tables: descriptions,
    });
}
