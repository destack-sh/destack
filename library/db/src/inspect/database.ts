import { defineSchema, schema } from "@destack/schema";
import { ResourceName } from "@destack/resource";
import { DatabaseSpec } from "../declare/index.ts";
import { TableDescription } from "./table.ts";

/** The inspected schema associated with a package database declaration. */
export const DatabaseDescription = defineSchema(schema.object({
    /** The package-local database declaration. */
    name: ResourceName,
    /** The schema description format version. */
    version: schema.literal(1),
    /** The SQL dialect used by these tables. */
    dialect: DatabaseSpec.shape.dialect,
    /** The declared tables, columns, indexes and relationships. */
    tables: schema.array(TableDescription),
}));
/** The inspected schema associated with a package database declaration. */
export type DatabaseDescription = schema.Infer<typeof DatabaseDescription>;
