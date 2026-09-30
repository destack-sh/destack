import { defineSchema, schema } from "@destack/schema";

/** The runtime selected by the compiler adapter. */
export const Runtime = defineSchema(schema.enum(["browser", "bun", "workerd"]));
/** The runtime selected by the compiler adapter. */
export type Runtime = schema.Infer<typeof Runtime>;

/** The module variant a runtime loads: browser modules in the browser, server modules elsewhere. */
export const Target = defineSchema(schema.enum(["browser", "server"]));
/** The module variant a runtime loads. */
export type Target = schema.Infer<typeof Target>;

/** A runtime that hosts execute workloads in. */
export const ServerRuntime = defineSchema(Runtime.exclude(["browser"]));
/** A runtime that hosts execute workloads in. */
export type ServerRuntime = schema.Infer<typeof ServerRuntime>;
