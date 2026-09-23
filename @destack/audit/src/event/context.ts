import { defineSchema, schema } from "@destack/schema";
import { identifier } from "@destack/schema/identifier";
import { Package } from "@destack/package";
import { AuditActor } from "./actor.ts";

/** Authority and origin supplied by the trusted host. */
export const AuditContext = defineSchema(
    schema.object({
        /** The authenticated actor performing the action. */
        actor: AuditActor,
        /** The represented identity when authenticated work acts on its behalf. */
        subject: AuditActor.optional(),
        /** Verified delegators, from the original initiator to the immediate delegator. */
        delegation: schema.array(AuditActor),
        /** The account and space whose history receives this event. */
        accountId: identifier("account").optional(),
        spaceId: identifier("space").optional(),
        /** The software and service producing the event. */
        package: Package,
        service: schema.string().min(1),
        /** The execution and authentication references, without credentials. */
        deploymentId: identifier("deployment").optional(),
        instanceId: identifier("instance").optional(),
        hostId: identifier("host").optional(),
        deviceId: identifier("device").optional(),
        sessionId: identifier("session").optional(),
        serviceTokenId: identifier("service-token").optional(),
        /** Related work and telemetry. */
        requestId: schema.string().min(1).optional(),
        operationId: schema.string().min(1).optional(),
        causeId: identifier("audit-event").optional(),
        traceId: schema
            .string()
            .regex(/^[0-9a-f]{32}$/)
            .optional(),
        /** Transport metadata selected by the host. */
        address: schema.string().optional(),
        userAgent: schema.string().optional(),
    }),
);
/** Trusted event context. */
export type AuditContext = schema.Infer<typeof AuditContext>;
