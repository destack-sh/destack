import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The tunnel package's failures, each reason with the call error code it answers. */
export const TunnelError = defineErrorFamily({
    reasons: {
        // invalid
        PROTOCOL_VIOLATION_INVALID: {},

        // unavailable
        CLIENT_UNAVAILABLE: {},
        SESSION_UNAVAILABLE: {},

        // unreachable
        METADATA_UNREACHABLE: {},
        RENEWAL_UNREACHABLE: {},
    },
});

/** A stable cause of tunnel failures. */
export type TunnelReason = ReasonOf<typeof TunnelError>;
