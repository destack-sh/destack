import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The relay package's failures, each reason with the call error code it answers. */
export const RelayError = defineErrorFamily({
    reasons: {
        // not found
        NAME_NOT_FOUND: {},
        RENEWAL_ONLY_NOT_FOUND: {},
        TUNNEL_CALL_NOT_FOUND: {},

        // required
        CLIENT_HEAD_REQUIRED: {},
        HANDSHAKE_REQUIRED: {},
        HOST_REQUIRED: {},

        // unauthenticated
        MACHINE_UNAUTHENTICATED: {},
        SUBPROTOCOL_UNAUTHENTICATED: {},

        // denied
        MACHINE_DENIED: {},
        RESIDENCY_DENIED: {},
        TUNNEL_DENIED: {},

        // moved
        TUNNEL_MOVED: {},

        // unavailable
        MACHINE_UNAVAILABLE: {},
        STREAMS_UNAVAILABLE: {},

        // unreachable
        ENDPOINT_UNREACHABLE: {},
        HEAD_UNREACHABLE: {},
        RENEWAL_UNREACHABLE: {},
    },
});

/** A stable cause of relay failures. */
export type RelayReason = ReasonOf<typeof RelayError>;
