import { describeTable } from "@destack/db";
import { TableDescription } from "@destack/db/inspect";
import { Inspector } from "@destack/package/inspect";
import { tables } from "./tables.ts";

/** Describe all exported model tables without opening a database. */
export function inspect() {
    return Object.values(tables).map(describeTable);
}

/** Describe exported tables in the built package manifest. */
export const inspector = new Inspector(1, { tables: TableDescription }, (context) => ({
    tables: Object.entries(tables).map(([name, table]) => ({
        declaration: context.resolve("src/index.ts", name),
        description: describeTable(table),
    })),
}));
