import { defineSchema, schema } from "@destack/schema";

/** The SQL dialect used by physical schemas and migration histories. */
export const Dialect = defineSchema(schema.enum(["sqlite", "postgresql"]));

/** The SQL dialect used by physical schemas and migration histories. */
export type Dialect = schema.Infer<typeof Dialect>;
