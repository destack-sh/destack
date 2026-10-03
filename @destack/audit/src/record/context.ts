import { defineSchema, schema } from "@destack/schema";
import { Package } from "@destack/package";
import { AuditCaller } from "./actor.ts";

/** The authority and origin of an executed call, as its host supplied them. */
export const AuditContext = defineSchema(
    schema.object({
        /** The caller: a verified subject with its delegates, the platform, or an anonymous caller. */
        caller: AuditCaller,
        /** The scope whose history receives the call. */
        scope: schema.string().min(1),
        /** The package serving the call. */
        package: Package,
        /** The service serving the call. */
        service: schema.string().min(1),
        /** The deployment executing the service. */
        deploymentId: schema.identifier("deployment").exactOptional(),
        /** The instance executing the service. */
        instanceId: schema.identifier("instance").exactOptional(),
        /** The host executing the service. */
        hostId: schema.identifier("host").exactOptional(),
        /** The device the caller authenticated on. */
        deviceId: schema.identifier("device").exactOptional(),
        /** The caller's session, without its credential. */
        sessionId: schema.identifier("session").exactOptional(),
        /** The caller's token, without its secret. */
        tokenId: schema.identifier("token").exactOptional(),
        /** The call that caused this one. */
        causeId: schema.identifier("call").exactOptional(),
        /** The trace the call belongs to, as 32 lowercase hexadecimal digits. */
        traceId: schema
            .string()
            .regex(/^[0-9a-f]{32}$/u)
            .exactOptional(),
        /** The caller's network address, as the host saw it. */
        address: schema.string().exactOptional(),
        /** The caller's user agent, as the host saw it. */
        userAgent: schema.string().exactOptional(),
    }),
);
/** The authority and origin of an executed call, as its host supplied them. */
export type AuditContext = schema.Infer<typeof AuditContext>;
