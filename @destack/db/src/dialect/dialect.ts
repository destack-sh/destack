import { defineSchema, schema } from "@destack/schema";

/** The values one statement binds at most, below SQLITE_MAX_VARIABLE_NUMBER of 32,766, leaving room for a statement's other parameters. */
export const PARAMETER_BUDGET = 30_000;

/** The SQL dialect a database runs: SQLite or PostgreSQL. */
export const Dialect = defineSchema(schema.enum(["sqlite", "postgresql"]));

/** The SQL dialect a database runs: SQLite or PostgreSQL. */
export type Dialect = schema.Infer<typeof Dialect>;
