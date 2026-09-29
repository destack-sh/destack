import { AuditRecorder } from "@destack/audit";
import { AuditOutbox } from "@destack/audit/outbox";
import type { DatabaseConnection } from "@destack/db";
import type { Caller } from "@destack/service/authentication";
import { identifier, schema } from "@destack/schema";
import type {} from "@destack/package/import-meta";

/** The package declaring the account service's audit actions. */
export const accountPackage = import.meta.destack.package;

/** The credential of a verified caller. */
const AuditedCredential = schema
    .object({ kind: schema.string(), id: schema.string() })
    .passthrough();

/** Create an audit recorder for a caller and scope. */
export function accountAudit(
    caller: Caller | null,
    database: DatabaseConnection,
    requestId: string,
    scope: string,
) {
    // read the session or token the caller presented
    const credential = caller === null ? undefined : AuditedCredential.parse(caller.credential);
    const isToken =
        credential?.kind === "personal-access-token" || credential?.kind === "service-token";

    return AuditRecorder.from(caller, new AuditOutbox(database), {
        package: accountPackage,
        service: "account",
        scope,
        requestId,
        sessionId:
            credential?.kind === "session" ? identifier("session").parse(credential.id) : undefined,
        tokenId: isToken ? identifier("token").parse(credential!.id) : undefined,
    });
}
