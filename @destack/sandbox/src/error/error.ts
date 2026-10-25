import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The sandbox package's failures, each reason with the call error code it answers. */
export const SandboxError = defineErrorFamily(
    import.meta.destack.package,
    {
        // bad request
        GRACE_PERIOD_INVALID: { code: "BAD_REQUEST" },

        // internal server error
        START_FAILED: { code: "INTERNAL_SERVER_ERROR" },
        STOP_FAILED: { code: "INTERNAL_SERVER_ERROR" },

        // not implemented
        UNSUPPORTED: { code: "NOT_IMPLEMENTED" },
    },
    { audience: "host" },
);

/** A stable cause of sandbox failures. */
export type SandboxReason = ReasonOf<typeof SandboxError>;
