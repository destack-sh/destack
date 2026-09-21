import { defineSchema, schema } from "@destack/schema";

/** The runtime selected by the compiler adapter. */
export const Runtime = defineSchema(schema.enum(["browser", "bun", "workerd"]));
/** The runtime selected by the compiler adapter. */
export type Runtime = schema.Infer<typeof Runtime>;
