import { defineSchema, schema } from "@destack/schema";
import { DeclarationName } from "@destack/package";
import { Dialect } from "../dialect/dialect.ts";
import { TableDescription } from "./table.ts";
import { TreeDescription } from "./tree.ts";

/** The tables managed by one named database schema. */
export const DatabaseSchemaDescription = defineSchema(
    schema.object({
        /** The stable schema name within its database. */
        name: DeclarationName,
        /** The schema description format version. */
        version: schema.literal(1),
        /** The SQL dialect used by these tables. */
        dialect: Dialect,
        /** The declared tables, columns, indexes and relationships. */
        tables: schema.array(TableDescription),
        /** Tree indexes maintained by the schema's committed migrations. */
        trees: schema.array(TreeDescription).optional(),
        /** Schema histories that must be prepared before this one. */
        dependencies: schema.array(DeclarationName).optional(),
    }),
);
/** The tables managed by one named database schema. */
export type DatabaseSchemaDescription = schema.Infer<typeof DatabaseSchemaDescription>;
