import { defineSchema, schema } from "@destack/schema";
import { identifier } from "@destack/schema/identifier";
import { Package } from "@destack/package";
import { AuditActor } from "./actor.ts";
import { Subject } from "@destack/sync";

/** The authority and origin of an executed call, as its host supplied them. */
export const AuditContext = defineSchema(
    schema.object({
        /** The authenticated actor performing the call. */
        actor: AuditActor,
        /** The subject the actor acts for. */
        subject: Subject.optional(),
        /** The delegators, from the initiator to the immediate delegator. */
        delegation: schema.array(AuditActor),
        /** The scope whose history receives the call. */
        scope: schema.string().min(1),
        /** The package serving the call. */
        package: Package,
        /** The service serving the call. */
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
        /** The call that caused this one. */
        causeId: identifier("call").optional(),
        /** The trace the call belongs to, as 32 lowercase hexadecimal digits. */
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
/** The authority and origin of an executed call, as its host supplied them. */
export type AuditContext = schema.Infer<typeof AuditContext>;
