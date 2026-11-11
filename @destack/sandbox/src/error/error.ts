import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The sandbox package's failures, each reason with the call error code it answers. */
export const SandboxError = defineErrorFamily({
    reasons: {
        // not found
        TOOL_NOT_FOUND: {},

        // invalid
        GRACE_PERIOD_INVALID: {},

        // unsupported
        HOST_UNSUPPORTED: {},

        // broken
        START_BROKEN: {},
        STOP_BROKEN: {},
    },
    audience: "host",
});

/** A stable cause of sandbox failures. */
export type SandboxReason = ReasonOf<typeof SandboxError>;
