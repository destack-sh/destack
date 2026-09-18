import { defineSchema, schema } from "@destack/schema";
import { defineResource } from "@destack/resource";
import { TableDescription } from "./table.ts";

/** The SQL dialect and declared tables required by a database. */
export const DatabaseSpec = defineSchema(schema.object({
    /** The SQL dialect required by these declarations. */
    dialect: schema.enum(["sqlite", "postgresql", "mysql"]),
    /** The tables declared by this package. */
    tables: schema.array(TableDescription),
}));

/** A database resource declaration. */
export const DatabaseDescription = defineResource("database", 1, DatabaseSpec);
/** A database resource declaration. */
export type DatabaseDescription = schema.Infer<typeof DatabaseDescription>;
