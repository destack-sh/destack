import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The sync package's failures, each reason with the call error code it answers. */
export const SyncError = defineErrorFamily({
    reasons: {
        // not found
        SCOPE_NOT_FOUND: {},

        // invalid
        PARAMETERS_INVALID: {},
        SCOPE_INVALID: {},

        // unauthenticated
        CREDENTIAL_UNAUTHENTICATED: {},
        FOLLOWER_UNAUTHENTICATED: {},

        // blocked
        FOLLOW_BLOCKED: {},
        POSITION_BLOCKED: {},

        // exhausted
        CAPACITY_EXHAUSTED: {},

        // unavailable
        FEED_UNAVAILABLE: {},
        REPLICA_UNAVAILABLE: {},

        // unreachable
        STREAM_UNREACHABLE: {},
    },
});

/** A stable cause of sync failures. */
export type SyncReason = ReasonOf<typeof SyncError>;
