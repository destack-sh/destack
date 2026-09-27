import { defineSchema, schema } from "@destack/schema";
import { identifier } from "@destack/schema/identifier";
import { Package } from "@destack/package";
import { Subject } from "@destack/access";
import { AuditActor } from "./actor.ts";

/** Authority and origin supplied by the trusted host. */
export const AuditContext = defineSchema(
    schema.object({
        /** The authenticated actor performing the action. */
        actor: AuditActor,
        /** The represented identity when authenticated work acts on its behalf. */
        subject: Subject.optional(),
        /** Verified delegators, from the original initiator to the immediate delegator. */
        delegation: schema.array(AuditActor),
        /** The scope whose history receives this event: the scope of the object it is about, or global for platform events outside every scope. */
        scope: schema.string().min(1),
        /** The package producing the event. */
        package: Package,
        /** The service producing the event. */
        service: schema.string().min(1),
        /** The deployment executing the service. */
        deploymentId: identifier("deployment").optional(),
        /** The instance executing the service. */
        instanceId: identifier("instance").optional(),
        /** The host executing the service. */
        hostId: identifier("host").optional(),
        /** The device the caller authenticated on. */
        deviceId: identifier("device").optional(),
        /** The caller's session, without its credential. */
        sessionId: identifier("session").optional(),
        /** The caller's token, without its secret. */
        tokenId: identifier("token").optional(),
        /** The request the event belongs to. */
        requestId: schema.string().min(1).optional(),
        /** The operation the event belongs to. */
        operationId: schema.string().min(1).optional(),
        /** The event that caused this one. */
        causeId: identifier("audit-event").optional(),
        /** The trace the event belongs to, as 32 lowercase hexadecimal digits. */
        traceId: schema
            .string()
            .regex(/^[0-9a-f]{32}$/)
            .optional(),
        /** The caller's network address, as the host saw it. */
        address: schema.string().optional(),
        /** The caller's user agent, as the host saw it. */
        userAgent: schema.string().optional(),
    }),
);
/** Trusted event context. */
export type AuditContext = schema.Infer<typeof AuditContext>;
