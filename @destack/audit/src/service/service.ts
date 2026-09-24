import { schema } from "@destack/schema";
import { identifier } from "@destack/schema/identifier";
import { defineProcedure, defineService, eventIterator } from "@destack/service";
import { AuditEntry, AuditAcknowledgement } from "../outbox/delivery.ts";
import { AuditPage, AuditQuery, AuditRecord, AuditScope } from "../history/query.ts";
import type {} from "@destack/package/import-meta";

/** The portable audit service definition. */
export const auditService = defineService("audit", {
    ingest: procedure("ingest")
        .route({ method: "POST", path: "/audit/events" })
        .input(AuditEntry)
        .output(AuditAcknowledgement),
    get: procedure("get")
        .route({ method: "POST", path: "/audit/get" })
        .input(schema.object({ scope: AuditScope, id: identifier("audit-event") }))
        .output(AuditRecord),
    list: procedure("list")
        .route({ method: "POST", path: "/audit/list" })
        .input(AuditQuery)
        .output(AuditPage),
    export: procedure("export")
        .route({ method: "POST", path: "/audit/export" })
        .input(AuditQuery)
        .output(eventIterator(AuditRecord)),
    prune: procedure("prune")
        .route({ method: "POST", path: "/audit/prune" })
        .input(
            schema.object({
                scope: AuditScope,
                before: schema.number().int().nonnegative(),
                limit: schema.number().int().min(1).max(1000),
            }),
        )
        .output(schema.object({ events: schema.number().int().nonnegative() })),
});

/** Declare distinct operation permissions; handlers record history access explicitly. */
function procedure(action: "ingest" | "get" | "list" | "export" | "prune") {
    return defineProcedure({
        authentication: "identity",
        permission: {
            packageId: import.meta.destack.package.id,
            type: "audit",
            name: action,
        },
        audit: false,
    });
}
