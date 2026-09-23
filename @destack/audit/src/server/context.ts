import { sameSubject, type Subject } from "@destack/access";
import type { Caller } from "@destack/service/authentication";
import type { ServiceContext } from "@destack/service/server";
import { context, trace, isSpanContextValid } from "@destack/telemetry";
import { AuditRecorder, type AuditWriter } from "../record/index.ts";
import type { AuditActor, AuditContext } from "../event/index.ts";

/** Host-selected event origin, independent of the authenticated caller. */
export type AuditOrigin = Omit<
    AuditContext,
    "actor" | "subject" | "delegation" | "deploymentId" | "traceId"
>;

/** Capture verified identity and active telemetry without reading credentials or databases. */
export function createAuditContext(caller: Caller | null, origin: AuditOrigin): AuditContext {
    // retain acting and represented identities independently
    const authentication = caller?.authentication;
    const subject = authentication?.actor ?? authentication?.subject;
    const actor = subject ? describeActor(subject) : { type: "anonymous" as const };
    const delegation = (authentication?.delegations ?? []).map((entry) =>
        describeActor(entry.subject),
    );

    // select only the deployment verified for the acting identity
    const deploymentId = subject
        ? authentication?.deployments?.find((entry) => sameSubject(entry.subject, subject))?.id
        : undefined;
    const span = trace.getSpanContext(context.active());

    return {
        ...origin,
        actor,
        subject: authentication ? describeActor(authentication.subject) : undefined,
        delegation,
        deploymentId,
        traceId: span && isSpanContextValid(span) ? span.traceId : undefined,
    };
}

/** Bind request attribution to a host-selected durable audit writer. */
export function createRecorder<Transaction>(
    request: ServiceContext,
    writer: AuditWriter<Transaction>,
    origin: AuditOrigin,
): AuditRecorder<Transaction> {
    // exclude rejected authentication while retaining denied authenticated callers
    const caller = request.authenticationError === undefined ? request.caller : null;
    const context = createAuditContext(caller, { ...origin, requestId: request.requestId });

    return new AuditRecorder(context, writer);
}

/** Preserve the complete identity established by authentication. */
function describeActor(subject: Subject): AuditActor {
    return { type: subject.kind, authority: subject.authority, id: subject.id };
}
