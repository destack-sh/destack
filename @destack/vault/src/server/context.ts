import { AuditRecorder, AuditActor } from "@destack/audit";
import { AuditOutbox } from "@destack/audit/outbox";
import type { DatabaseConnection } from "@destack/db";
import type { ServiceContext } from "@destack/service/server";
import type { space } from "@destack/model/regional";
import { vaultPackage } from "../audit/index.ts";
import { sameSubject } from "@destack/access";
import { trace, context, isSpanContextValid } from "@destack/telemetry";

import type { Secret } from "../secret/index.ts";
import type { vaultService } from "../service/index.ts";
import type { Caller } from "@destack/service/authentication";

/** Exact operation names derived from the public service router. */
export type VaultOperation = {
    [
        Domain in keyof typeof vaultService
    ]: `${Domain}.${keyof (typeof vaultService)[Domain] & string}`;
}[keyof typeof vaultService];

/** Authenticated caller and durable audit recorder established by the receiving host. */
export interface VaultContext {
    /** Shared, audience-qualified authentication result. */
    caller: Caller;
    /** Durable recorder bound to the verified caller and this database. */
    audit: AuditRecorder<DatabaseConnection>;
}
/** The persisted target checked against regional grants and deployment bindings. */
export interface VaultAccess {
    /** Exact operation, such as version.read or secret.create. */
    operation: VaultOperation;
    /** Selected space. */
    spaceId: Secret["spaceId"];
    /** Containing vault, derived from persisted metadata for secret operations. */
    vaultId?: Secret["vaultId"];
    /** Secret identity, absent for collection operations. */
    secretId?: Secret["id"];
    /** Exact version when applicable. */
    version?: number;
}

/** Record verified identity and delegation without retaining credentials or secret values. */
export function vaultAudit(
    request: ServiceContext,
    database: DatabaseConnection,
    selected?: Pick<typeof space.$inferSelect, "id" | "accountId">,
): AuditRecorder<DatabaseConnection> {
    // exclude identities whose authentication failed
    const authentication =
        request.authenticationError === undefined ? request.caller?.authentication : undefined;
    const subject = authentication?.actor ?? authentication?.subject;
    const actor = subject
        ? AuditActor.parse({ type: subject.kind, id: subject.id })
        : { type: "anonymous" as const };
    const delegation = (authentication?.delegations ?? []).map(({ subject }) =>
        AuditActor.parse({ type: subject.kind, id: subject.id }),
    );

    // associate the acting deployment and trace with both procedure and domain records
    const deploymentId = subject
        ? authentication?.deployments?.find((entry) => sameSubject(entry.subject, subject))?.id
        : undefined;
    const span = trace.getSpanContext(context.active());

    return new AuditRecorder(
        {
            actor,
            delegation,
            package: vaultPackage,
            service: "vault",
            spaceId: selected?.id,
            accountId: selected?.accountId,
            requestId: request.requestId,
            deploymentId,
            traceId: span && isSpanContextValid(span) ? span.traceId : undefined,
        },
        new AuditOutbox(database),
    );
}
