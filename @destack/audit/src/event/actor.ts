import { defineSchema, schema } from "@destack/schema";
import { Subject } from "@destack/access";

/** The authenticated identity captured when an action occurs. */
export const AuditActor = defineSchema(
    schema.discriminatedUnion("type", [
        schema.object({
            /** A verified subject acted. */
            type: schema.literal("subject"),
            /** The principal that acted. */
            subject: Subject,
            /** The display name captured when recording. */
            name: schema.string().optional(),
        }),
        schema.object({
            /** The platform acted on its own. */
            type: schema.literal("system"),
            /** The component that acted. */
            name: schema.string().min(1),
        }),
        schema.object({
            /** An unauthenticated caller acted. */
            type: schema.literal("anonymous"),
        }),
    ]),
);
/** The identity captured when an action occurs. */
export type AuditActor = schema.Infer<typeof AuditActor>;
