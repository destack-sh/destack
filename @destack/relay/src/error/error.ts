import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The relay package's failures, each reason with the call error code it answers. */
export const RelayError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    FORWARDED_HEAD_MISSING: { code: "BAD_REQUEST" },
    FORWARDED_HOST_MISSING: { code: "BAD_REQUEST" },
    HANDSHAKE_MISSING: { code: "BAD_REQUEST" },
    PROTOCOL_VIOLATION: { code: "BAD_REQUEST" },

    // unauthorized
    SUBPROTOCOL_INVALID: { code: "UNAUTHORIZED" },
    MACHINE_REVOKED: { code: "UNAUTHORIZED" },
    MACHINE_UNKEYED: { code: "UNAUTHORIZED" },
    MACHINE_UNKNOWN: { code: "UNAUTHORIZED" },

    // forbidden
    RESIDENCY_FOREIGN: { code: "FORBIDDEN" },
    TUNNEL_FOREIGN: { code: "FORBIDDEN" },
    MACHINE_REQUIRED: { code: "FORBIDDEN" },

    // not found
    NAME_UNRESOLVED: { code: "NOT_FOUND" },
    RENEWAL_ONLY: { code: "NOT_FOUND" },
    TUNNEL_CALL_UNKNOWN: { code: "NOT_FOUND" },

    // misdirected request
    RELAY_OTHER: { code: "MISDIRECTED_REQUEST" },

    // bad gateway
    ENDPOINT_UNREACHABLE: { code: "BAD_GATEWAY" },
    RENEWAL_REFUSED: { code: "BAD_GATEWAY" },

    // service unavailable
    CLIENT_CLOSED: { code: "SERVICE_UNAVAILABLE" },
    MACHINE_DISCONNECTED: { code: "SERVICE_UNAVAILABLE" },
    SESSION_CLOSED: { code: "SERVICE_UNAVAILABLE" },
    STREAMS_EXHAUSTED: { code: "SERVICE_UNAVAILABLE" },

    // gateway timeout
    HEAD_TIMEOUT: { code: "GATEWAY_TIMEOUT" },
});

/** A stable cause of relay failures. */
export type RelayReason = ReasonOf<typeof RelayError>;
