import { defineProcedure, defineService } from "@destack/service";
import { schema } from "@destack/schema";
import { AuditBatch } from "./batch.ts";
import type {} from "@destack/package/import-meta";

/** A procedure whose handler admits its caller through the host's intake. */
const procedure = defineProcedure({ authentication: "identity", permission: null, audit: false });

/** The audit service: ended calls another host's journal delivers to the scopes this history keeps. */
export const auditService = defineService("audit", {
    /** Store a batch of ended calls another host's journal delivers to the scopes this history keeps. */
    ingest: procedure
        .route({ method: "POST", path: "/audit/ingest" })
        .input(AuditBatch)
        .output(schema.object({ /** The stored count. */ stored: schema.int().min(0) })),
});
