import { defineSchema, schema } from "@destack/schema";
import { anyone, Delegate } from "@destack/access";
import { Subject } from "@destack/sync";

/** The principal that performed a call. */
const auditActor = defineSchema(
    schema.object({
        /** The principal that acted, anyone for an unauthenticated caller. */
        subject: Subject,
        /** The display name captured when recording. */
        name: schema.string().exactOptional(),
    }),
);
/** The principal that performed a call, and the key filters name it by. */
export const AuditActor = Object.assign(auditActor, {
    /** Encode an actor as the actor key a call's event is filtered by. */
    key(actor: AuditActor): string {
        return Subject.key(actor.subject);
    },
});
/** The principal that performed a call. */
export type AuditActor = schema.Infer<typeof auditActor>;

/** The schema of a recorded caller. */
const auditCaller = defineSchema(
    schema.object({
        /** The represented principal, anyone for an unauthenticated caller. */
        subject: Subject,
        /** The principals acting for the subject in order, the last one sending the call. */
        delegates: schema.array(Delegate).exactOptional(),
        /** The represented subject's display name captured when recording. */
        name: schema.string().exactOptional(),
    }),
);
/** The caller of a call as the history keeps it. */
export type AuditCaller = schema.Infer<typeof auditCaller>;

/** The caller of a call as the history keeps it, and the actor it derives. */
export const AuditCaller = Object.assign(auditCaller, {
    /** The caller every unauthenticated call records. */
    anyone: auditCaller.parse({ subject: anyone.reference("*", "*") }),

    /** Read who acted: the last delegate, else the represented subject. */
    actor(caller: AuditCaller): AuditActor {
        const sender = caller.delegates?.at(-1)?.subject;

        return sender === undefined
            ? {
                  subject: caller.subject,
                  ...(caller.name === undefined ? {} : { name: caller.name }),
              }
            : { subject: sender };
    },
});
