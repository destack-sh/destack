import { defineSchema, schema } from "@destack/schema";
import { Subject } from "@destack/access";

/** The authenticated identity captured when an action occurs. */
export const AuditActor = defineSchema(
    schema.discriminatedUnion("type", [
        schema.object({
            /** The verified identity category. */
            type: Subject.shape.kind,
            /** The authority assigning this identity. */
            authority: Subject.shape.authority,
            /** The immutable identifier within that authority. */
            id: Subject.shape.id,
            /** The display name captured when recording. */
            name: schema.string().optional(),
        }),
        schema.object({ type: schema.literal("system"), name: schema.string().min(1) }),
        schema.object({ type: schema.literal("anonymous") }),
    ]),
);
/** An authenticated identity. */
export type AuditActor = schema.Infer<typeof AuditActor>;

/** Encode the complete identity for indexed history selection. */
export function actorKey(actor: AuditActor): string {
    if ("id" in actor) {
        return JSON.stringify([actor.type, actor.authority, actor.id]);
    } else if (actor.type === "system") {
        return JSON.stringify([actor.type, actor.name]);
    } else {
        return JSON.stringify([actor.type]);
    }
}
