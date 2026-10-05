import * as schema from "../validate/index.ts";
import { defineSchema } from "../declare/schema.ts";

/** An instant in UTC epoch milliseconds, as Temporal's Instant counts it. */
export const Instant = defineSchema(schema.number().int().nonnegative());
/** An instant in UTC epoch milliseconds. */
export type Instant = schema.Infer<typeof Instant>;
