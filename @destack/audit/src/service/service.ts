import { principal, universe } from "@destack/access";
import { defineProcedure, defineService } from "@destack/service";
import { schema } from "@destack/schema";
import { AuditBatch } from "./batch.ts";
import type {} from "@destack/package/import-meta";

/** A procedure the principals delivering journals call, each admitted for its scopes through the machine's intake. */
const procedure = defineProcedure({
    authentication: "identity",
    permission: {
        principals: [universe, principal.machine, principal.space, principal.installation],
    },
    audit: false,
});

/** The audit service: ended calls another machine's journal delivers to the scopes this history keeps. */
export const auditService = defineService("audit", {
    /** Store a batch of ended calls another machine's journal delivers to the scopes this history keeps. */
    ingest: procedure
        .route({ method: "POST", path: "/audit/ingest" })
        .input(AuditBatch)
        .output(schema.object({ /** The stored count. */ stored: schema.int().min(0) })),
});
