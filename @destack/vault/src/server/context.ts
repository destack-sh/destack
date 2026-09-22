import { AuditRecorder, AuditActor } from "@destack/audit";
import { AuditOutbox } from "@destack/audit/outbox";
import type { DatabaseConnection } from "@destack/db";
import type { ServiceContext } from "@destack/service/server";
import type { space } from "@destack/model/regional";
import { vaultPackage } from "../audit/index.ts";
import { sameSubject } from "@destack/access";
import { trace, context, isSpanContextValid } from "@destack/telemetry";

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
