import { defineSchema, schema } from "@destack/schema";

/** The supported storage and processing jurisdictions. */
export const RESIDENCIES = ["eu", "us"] as const;

/** The permitted jurisdiction for storage and processing. */
export const Residency = defineSchema(schema.enum(RESIDENCIES));
/** The permitted jurisdiction for storage and processing. */
export type Residency = schema.Infer<typeof Residency>;
