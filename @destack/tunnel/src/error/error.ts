import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The tunnel package's failures, each reason with the call error code it answers. */
export const TunnelError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    PROTOCOL_VIOLATION: { code: "BAD_REQUEST" },

    // bad gateway
    METADATA_INVALID: { code: "BAD_GATEWAY" },
    RENEWAL_REFUSED: { code: "BAD_GATEWAY" },

    // service unavailable
    CLIENT_CLOSED: { code: "SERVICE_UNAVAILABLE" },
    SESSION_CLOSED: { code: "SERVICE_UNAVAILABLE" },
});

/** A stable cause of tunnel failures. */
export type TunnelReason = ReasonOf<typeof TunnelError>;
