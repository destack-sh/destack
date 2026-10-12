import { principal, universe } from "@destack/access";
import { defineProcedure, defineService } from "@destack/service";
import { schema } from "@destack/schema";
import { AuditBatch } from "./batch.ts";
import type {} from "@destack/package/import-meta";

/** A procedure the principals delivering journals call. */
const procedure = defineProcedure({
    authentication: "identity",
    permission: {
        principals: [universe, principal.machine, principal.space, principal.installation],
    },
    audit: false,
});

/** The audit service: ended calls another machine's journal delivers to this history's scopes. */
export const auditService = defineService("audit", {
    /** Store a batch of ended calls that another machine's journal delivers to this history. */
    ingest: procedure
        .route({ method: "POST", path: "/audit/ingest" })
        .input(AuditBatch)
        .output(schema.object({ /** The stored count. */ stored: schema.int().min(0) })),
});
