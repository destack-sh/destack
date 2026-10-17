import * as schema from "../validate/index.ts";

/** An instant in UTC epoch milliseconds, as Temporal's Instant counts it. */
export const Instant = schema.number().int().nonnegative();
/** An instant in UTC epoch milliseconds. */
export type Instant = schema.Infer<typeof Instant>;
