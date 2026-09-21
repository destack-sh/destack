import { createClient, type ClientOptions } from "@destack/service/client";
import { auditService } from "../service/service.ts";

/** Connect to a local or regional audit service. */
export function createAuditClient(options: ClientOptions) {
    return createClient(auditService, options);
}
/** The typed remote audit client. */
export type AuditClient = ReturnType<typeof createAuditClient>;
