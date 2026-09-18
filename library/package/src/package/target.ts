import { defineSchema, schema } from "@destack/schema";

/** A Destack execution target. */
export const Target = defineSchema(schema.enum(["browser", "worker", "host"]));

/** A Destack execution target. */
export type Target = schema.Infer<typeof Target>;
