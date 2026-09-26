import { defineSchema, schema } from "@destack/schema";

/** The values one statement binds at most, below SQLite's limit of 32,766 parameters. */
export const PARAMETER_BUDGET = 30_000;

/** The SQL dialect a database runs: SQLite or PostgreSQL. */
export const Dialect = defineSchema(schema.enum(["sqlite", "postgresql"]));

/** The SQL dialect a database runs: SQLite or PostgreSQL. */
export type Dialect = schema.Infer<typeof Dialect>;
