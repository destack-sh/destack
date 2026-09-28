import { defineSchema, schema } from "@destack/schema";

/** The most values one statement binds, below SQLite's limit of 32,766. */
export const PARAMETER_BUDGET = 30_000;

/** The SQL dialect of a database. */
export const Dialect = defineSchema(schema.enum(["sqlite", "postgresql"]));

/** The SQL dialect of a database. */
export type Dialect = schema.Infer<typeof Dialect>;
