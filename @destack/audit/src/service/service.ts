import { schema } from "@destack/schema";
import { defineProcedure, defineService, eventIterator } from "@destack/service";
import { AuditEntry, AuditAcknowledgement } from "../outbox/delivery.ts";
import { AuditPrune, AuditQuery, AuditRecord } from "../history/query.ts";
import type {} from "@destack/package/import-meta";

/** An operation whose handler records the history access, then decides the history permission itself. */
const procedure = defineProcedure({ authentication: "identity", permission: null, audit: false });

/** The portable audit service definition. */
export const auditService = defineService("audit", {
    ingest: procedure
        .route({ method: "POST", path: "/audit/events" })
        .input(AuditEntry)
        .output(AuditAcknowledgement),
    export: procedure
        .route({ method: "POST", path: "/audit/export" })
        .input(AuditQuery)
        .output(eventIterator(AuditRecord)),
    prune: procedure
        .route({ method: "POST", path: "/audit/prune" })
        .input(AuditPrune)
        .output(schema.object({ events: schema.number().int().nonnegative() })),
});
