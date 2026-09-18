import { describeTable } from "@destack/db";
import { TableDescription } from "@destack/db/inspect";
import { createInspection } from "@destack/package/inspect";
import { ModuleGraph, SymbolReference } from "@destack/package/code";
import { schema } from "@destack/schema";
import { tables } from "./tables.ts";

/** Describe all exported model tables without opening a database. */
export function inspect() {
    return Object.values(tables).map(describeTable);
}

/** Describe model tables and their exported symbols. */
export function inspectPackage(code: ModuleGraph) {
    // describe each table with its exported symbol
    const descriptions = {
        tables: Object.entries(tables).map(([name, table]) => ({
            symbol: code.resolveExport("src/index.ts", name),
            description: describeTable(table),
        })),
    };
    const definition = schema.object({
        tables: schema.array(schema.object({
            symbol: SymbolReference,
            description: TableDescription,
        })),
    });

    return createInspection(
        "@destack/model",
        1,
        code,
        definition,
        descriptions,
    );
}
