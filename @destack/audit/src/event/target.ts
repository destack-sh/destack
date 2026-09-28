import { defineSchema, schema } from "@destack/schema";

/** An affected object. */
export const AuditTarget = defineSchema(
    schema.object({
        /** The object type. */
        type: schema.string().min(1),
        /** The object identifier. */
        id: schema.string().min(1),
        /** The display name at the time of the event. */
        name: schema.string().optional(),
        /** The object version. */
        version: schema.string().optional(),
    }),
);
/** An affected object. */
export type AuditTarget = schema.Infer<typeof AuditTarget>;
