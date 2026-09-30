import { defineSchema, schema } from "@destack/schema";

/** Where a database lives, widest first: once in the universe, once per region, or within one zone. */
export const DatabaseTier = defineSchema(schema.enum(["global", "regional", "zonal"]));
/** Where a database lives. */
export type DatabaseTier = schema.Infer<typeof DatabaseTier>;
