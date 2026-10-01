import { defineSchema, schema } from "@destack/schema";

/** The runtime selected by the compiler adapter. */
export const Runtime = defineSchema(schema.enum(["browser", "bun", "workerd"]));
/** The runtime selected by the compiler adapter. */
export type Runtime = schema.Infer<typeof Runtime>;

/** A runtime that hosts execute workloads in. */
export const ServerRuntime = defineSchema(Runtime.exclude(["browser"]));
/** A runtime that hosts execute workloads in. */
export type ServerRuntime = schema.Infer<typeof ServerRuntime>;
