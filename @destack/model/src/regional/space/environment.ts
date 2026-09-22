import { defineSchema, schema } from "@destack/schema";

/** The supported space purposes. */
export const ENVIRONMENTS = ["development", "preview", "production"] as const;

/** The purpose of a space. */
export const Environment = defineSchema(schema.enum(ENVIRONMENTS));
/** The purpose of a space. */
export type Environment = schema.Infer<typeof Environment>;
