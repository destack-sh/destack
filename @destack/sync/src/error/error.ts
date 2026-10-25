import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The sync package's failures, each reason with the call error code it answers. */
export const SyncError = defineErrorFamily(import.meta.destack.package, {
    // not found
    NOT_FOUND: { code: "NOT_FOUND" },

    // conflict
    CYCLE: { code: "CONFLICT" },

    // precondition failed
    EXPIRED: { code: "PRECONDITION_FAILED" },

    // unprocessable content
    SCOPE_INVALID: { code: "UNPROCESSABLE_CONTENT" },
    OVER_CAPACITY: { code: "UNPROCESSABLE_CONTENT" },

    // bad gateway
    STREAM_INVALID: { code: "BAD_GATEWAY" },
    CREDENTIAL_LAPSED: { code: "UNAUTHORIZED" },

    // service unavailable
    OVERLOADED: { code: "SERVICE_UNAVAILABLE" },
    STALE: { code: "SERVICE_UNAVAILABLE" },
});

/** A stable cause of sync failures. */
export type SyncReason = ReasonOf<typeof SyncError>;
