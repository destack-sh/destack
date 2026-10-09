import { defineSchema, schema } from "@destack/schema";
import { Package } from "@destack/package";
import { AuditCaller } from "./actor.ts";

/** The authority and origin of an executed call, as its machine supplied them. */
export const AuditContext = defineSchema(
    schema.object({
        /** The caller: a verified principal with its delegates, or anyone for an unauthenticated caller. */
        caller: AuditCaller,
        /** The code that made the call on its principal's behalf, such as a controller's name. */
        component: schema.string().min(1).exactOptional(),
        /** The scope whose history receives the call. */
        scope: schema.string().min(1),
        /** The Loan the caller's authority came with: its identifier and the home space that signed it. */
        loan: schema
            .object({
                /** The Loan's identifier. */
                id: schema.string().min(1),
                /** The home space that signed it. */
                issuer: schema.string().min(1),
            })
            .exactOptional(),
        /** The package serving the call. */
        package: Package,
        /** The service serving the call. */
        service: schema.string().min(1),
        /** The installation executing the service, as the machine relaying its journal recorded it. */
        installationId: schema.identifier("installation").exactOptional(),
        /** The instance executing the service, as the machine relaying its journal recorded it. */
        instanceId: schema.identifier("instance").exactOptional(),
        /** The machine executing the service. */
        machineId: schema.identifier("machine").exactOptional(),
        /** The deployment the acting workload ran, as the caller's verified claims name it. */
        deploymentId: schema.identifier("deployment").exactOptional(),
        /** The client the caller authenticated through. */
        clientId: schema.identifier("client").exactOptional(),
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
        /** The caller's network address, as the machine saw it. */
        address: schema.sensitive(schema.string(), "personal").exactOptional(),
        /** The caller's user agent, as the machine saw it. */
        userAgent: schema.sensitive(schema.string(), "personal").exactOptional(),
    }),
);
/** The authority and origin of an executed call, as its machine supplied them. */
export type AuditContext = schema.Infer<typeof AuditContext>;
