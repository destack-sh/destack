import { defineSchema, schema } from "@destack/schema";
import { Delegate } from "@destack/access";
import { Subject } from "@destack/sync";

/** The identity that performed a call: a subject, the platform, or an anonymous caller. */
const auditActor = defineSchema(
    schema.discriminatedUnion("type", [
        schema.object({
            /** A verified subject acted. */
            type: schema.literal("subject"),
            /** The principal that acted. */
            subject: Subject,
            /** The display name captured when recording. */
            name: schema.string().exactOptional(),
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
/** The identity that performed a call, and the key filters name it by. */
export const AuditActor = Object.assign(auditActor, {
    /** Encode an actor as the actor key a call's event is filtered by. */
    key(actor: AuditActor): string {
        // key a subject by its access key
        if (actor.type === "subject") {
            return Subject.key(actor.subject);
        }
        // key a system component by its name
        else if (actor.type === "system") {
            return JSON.stringify([actor.type, actor.name]);
        }
        // key an anonymous caller by its type
        else {
            return JSON.stringify([actor.type]);
        }
    },
});
/** The identity that performed a call. */
export type AuditActor = schema.Infer<typeof auditActor>;

/** The schema of a recorded caller. */
const auditCaller = defineSchema(
    schema.discriminatedUnion("type", [
        schema.object({
            /** A verified caller called. */
            type: schema.literal("subject"),
            /** The represented subject. */
            subject: Subject,
            /** The principals acting for the subject in order, the last one sending the call. */
            delegates: schema.array(Delegate).exactOptional(),
            /** The represented subject's display name captured when recording. */
            name: schema.string().exactOptional(),
        }),
        schema.object({
            /** The platform called on its own. */
            type: schema.literal("system"),
            /** The component that called. */
            name: schema.string().min(1),
        }),
        schema.object({
            /** An unauthenticated caller called. */
            type: schema.literal("anonymous"),
        }),
    ]),
);
/** The caller of a call as the history keeps it. */
export type AuditCaller = schema.Infer<typeof auditCaller>;

/** The caller of a call as the history keeps it, and the actor it derives. */
export const AuditCaller = Object.assign(auditCaller, {
    /** Read who acted: the last delegate, else the represented subject, the platform, or no one. */
    actor(caller: AuditCaller): AuditActor {
        // act as the sending delegate, or as the subject when it sent the call itself
        if (caller.type === "subject") {
            const sender = caller.delegates?.at(-1)?.subject;

            return sender === undefined
                ? {
                      type: "subject",
                      subject: caller.subject,
                      ...(caller.name === undefined ? {} : { name: caller.name }),
                  }
                : { type: "subject", subject: sender };
        }
        // act as the platform or anonymously
        else {
            return caller;
        }
    },
});
