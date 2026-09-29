import { defineSchema, schema } from "@destack/schema";

/** An account's handle, a DNS label. */
export const AccountHandle = defineSchema(schema.string().regex(/^(?!-)[a-z0-9-]{1,63}(?<!-)$/));
/** An account's handle. */
export type AccountHandle = schema.Infer<typeof AccountHandle>;

/** Whether a handle may be taken. */
export const HandleAvailability = defineSchema(schema.enum(["available", "taken", "invalid"]));
/** Whether a handle may be taken. */
export type HandleAvailability = schema.Infer<typeof HandleAvailability>;
