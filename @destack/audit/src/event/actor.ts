import { defineSchema, schema } from "@destack/schema";
import { identifier } from "@destack/schema/identifier";

/** The authenticated identity captured when an action occurs. */
export const AuditActor = defineSchema(
    schema.discriminatedUnion("type", [
        schema.object({
            type: schema.literal("user"),
            id: identifier("user"),
            name: schema.string().optional(),
        }),
        schema.object({
            type: schema.literal("service-account"),
            id: identifier("service-account"),
            name: schema.string().optional(),
        }),
        schema.object({ type: schema.literal("system"), name: schema.string().min(1) }),
        schema.object({ type: schema.literal("anonymous") }),
    ]),
);
/** An authenticated identity. */
export type AuditActor = schema.Infer<typeof AuditActor>;
