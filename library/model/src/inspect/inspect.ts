import { describeTable } from "@destack/db";
import { TableDescription } from "@destack/db/inspect";
import { createPackageInspection } from "@destack/package/inspect";
import { ModuleGraph, SymbolReference } from "@destack/package/code";
import { schema } from "@destack/schema";
import * as globalSchema from "../global/schema/index.ts";
import * as regionalSchema from "../regional/schema/index.ts";
import { auditEvent } from "../audit/index.ts";

/** Describe each administrative database without opening a connection. */
export function inspect() {
    return {
        global: Object.values(globalSchema.tables).map(describeTable),
        regional: Object.values(regionalSchema.tables).map(describeTable),
        audit: [describeTable(auditEvent)],
    };
}

/** Describe database placement, tables, and their exported symbols. */
export function inspectPackage(code: ModuleGraph) {
    // associate table descriptions with their database entrypoints
    const descriptions = [
        ...Object.entries(globalSchema.tables).map(([name, table]) => ({
            database: "global" as const,
            symbol: code.resolveExport("src/global/index.ts", name),
            description: describeTable(table),
        })),
        ...Object.entries(regionalSchema.tables).map(([name, table]) => ({
            database: "regional" as const,
            symbol: code.resolveExport("src/regional/index.ts", name),
            description: describeTable(table),
        })),
        {
            database: "audit" as const,
            symbol: code.resolveExport("src/audit/index.ts", "auditEvent"),
            description: describeTable(auditEvent),
        },
    ];
    const definition = schema.object({
        tables: schema.array(schema.object({
            database: schema.enum(["global", "regional", "audit"]),
            symbol: SymbolReference,
            description: TableDescription,
        })),
    });

    return createPackageInspection("@destack/model", 2, code, definition, { tables: descriptions });
}
