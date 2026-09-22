import { defineSchema, schema } from "@destack/schema";

/** An affected object retained independently of its current existence. */
export const AuditTarget = defineSchema(
    schema.object({
        /** The object type, qualified by its defining package when application-defined. */
        type: schema.string().min(1),
        /** The stable object identifier. */
        id: schema.string().min(1),
        /** The historical display name. */
        name: schema.string().optional(),
        /** The accessed or changed object version. */
        version: schema.string().optional(),
    }),
);
/** An affected object. */
export type AuditTarget = schema.Infer<typeof AuditTarget>;
