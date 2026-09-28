import { schema } from "@destack/schema";
import { defineProcedure, defineService, eventIterator } from "@destack/service";
import { AuditBatch } from "../outbox/delivery.ts";
import { AuditPrune, AuditQuery, AuditRecord } from "../history/query.ts";
import type {} from "@destack/package/import-meta";

/** A procedure that records its call and checks the history permission in its handler. */
const procedure = defineProcedure({ authentication: "identity", permission: null, audit: false });

/** The audit service definition. */
export const auditService = defineService("audit", {
    ingest: procedure
        .route({ method: "POST", path: "/audit/events" })
        .input(AuditBatch)
        .output(schema.object({ events: schema.number().int().nonnegative() })),
    export: procedure
        .route({ method: "POST", path: "/audit/export" })
        .input(AuditQuery)
        .output(eventIterator(AuditRecord)),
    prune: procedure
        .route({ method: "POST", path: "/audit/prune" })
        .input(AuditPrune)
        .output(schema.object({ events: schema.number().int().nonnegative() })),
});
