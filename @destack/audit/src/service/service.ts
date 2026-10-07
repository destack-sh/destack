import { schema } from "@destack/schema";
import { defineProcedure, defineService, eventIterator } from "@destack/service";
import { AuditPage, AuditPrune, AuditQuery, AuditRecord } from "./query.ts";
import type {} from "@destack/package/import-meta";

/** A procedure that records its call and checks the history permission in its handler. */
const procedure = defineProcedure({ authentication: "identity", permission: null, audit: false });

/** The audit service definition. */
export const auditService = defineService("audit", {
    list: procedure
        .route({ method: "POST", path: "/audit/list" })
        .input(AuditQuery)
        .output(AuditPage),
    export: procedure
        .route({ method: "POST", path: "/audit/export" })
        .input(AuditQuery)
        .output(eventIterator(AuditRecord)),
    prune: procedure
        .route({ method: "POST", path: "/audit/prune" })
        .input(AuditPrune)
        .output(schema.object({ calls: schema.number().int().nonnegative() })),
});
