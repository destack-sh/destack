import { defineSchema, schema } from "@destack/schema";

/** A package's source language. */
export const Language = defineSchema(schema.enum(["typescript", "typescript++"]));

/** A package's source language. */
export type Language = schema.Infer<typeof Language>;
