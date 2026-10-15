import { defineSchema, schema } from "@destack/schema";
import { Subject } from "@destack/sync";

/** The principal that performed a call. */
export const AuditActor = defineSchema(
    schema.object({
        /** The principal that acted, the anonymous one for a caller proving no identity. */
        subject: Subject,
        /** The display name captured when recording. */
        name: schema.string().exactOptional(),
    }),
);
/** The principal that performed a call. */
export type AuditActor = schema.Infer<typeof AuditActor>;

/** The schema of a recorded caller. */
const auditCaller = defineSchema(
    schema.object({
        /** Who the call was for, the anonymous principal for a caller proving no identity. */
        subject: Subject,
        /** The principals acting for the subject in order, the last one sending the call. */
        actors: schema.array(Subject).exactOptional(),
        /** The represented subject's display name captured when recording. */
        name: schema.string().exactOptional(),
    }),
);
/** The caller of a call as the history keeps it. */
export type AuditCaller = schema.Infer<typeof auditCaller>;

/** The caller of a call as the history keeps it, and the actor it derives. */
export const AuditCaller = Object.assign(auditCaller, {
    /** Read who acted: the last actor, else the subject. */
    actor(caller: AuditCaller): AuditActor {
        const sender = caller.actors?.at(-1);

        return sender === undefined
            ? {
                  subject: caller.subject,
                  ...(caller.name === undefined ? {} : { name: caller.name }),
              }
            : { subject: sender };
    },
});
