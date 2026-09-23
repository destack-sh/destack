import { describeTable, type Dialect } from "@destack/db";
import { TableDescription } from "@destack/db/inspect";
import { createPackageInspection } from "@destack/package/inspect";
import { ModuleGraph, SymbolReference } from "@destack/package/code";
import { schema } from "@destack/schema";
import { globalSchema } from "../global/schema/index.ts";
import { regionalSchema } from "../regional/schema/index.ts";
import { auditSchema } from "../audit/index.ts";

/** SQL dialects supported by the administrative models. */
const DIALECTS: readonly Dialect[] = ["sqlite", "postgresql"];

/** Model tables and their public entrypoints. */
const DATABASES = [
    { database: "global", entrypoint: "src/global/index.ts", tables: globalSchema.tables },
    { database: "regional", entrypoint: "src/regional/index.ts", tables: regionalSchema.tables },
    { database: "audit", entrypoint: "src/audit/index.ts", tables: auditSchema.tables },
] as const;

/** Describe each administrative model in both SQL dialects. */
export function inspect() {
    return Object.fromEntries(
        DATABASES.map(({ database, tables }) => [
            database,
            Object.values(tables).flatMap((table) =>
                DIALECTS.map((dialect) => describeTable(table, dialect)),
            ),
        ]),
    );
}

/** Describe table placement, SQL dialects, and exported symbols. */
export function inspectPackage(code: ModuleGraph) {
    // associate each physical description with its exported logical table
    const descriptions = DATABASES.flatMap(({ database, entrypoint, tables }) =>
        Object.entries(tables).flatMap(([name, table]) =>
            DIALECTS.map((dialect) => ({
                database,
                symbol: code.resolveExport(entrypoint, name),
                description: describeTable(table, dialect),
            })),
        ),
    );
    const definition = schema.object({
        tables: schema.array(
            schema.object({
                database: schema.enum(["global", "regional", "space", "audit"]),
                symbol: SymbolReference,
                description: TableDescription,
            }),
        ),
    });

    return createPackageInspection("@destack/model", code, definition, { tables: descriptions });
}
