import { defineErrorFamily, type ReasonOf } from "@destack/error";

/** The package owning the processes' failures, named here as configurations load it unbuilt. */
const OWNER = { name: "@destack/environment" };

/** The processes' failures, each reason with the call error code it answers. */
export const ProcessError = defineErrorFamily(
    OWNER,
    {
        // not found
        NOT_FOUND: { code: "NOT_FOUND" },

        // forbidden
        PERMISSION_DENIED: { code: "FORBIDDEN" },

        // internal server error
        PROCESS_FAILED: { code: "INTERNAL_SERVER_ERROR" },
    },
    { audience: "host" },
);

/** A stable cause of process failures. */
export type ProcessReason = ReasonOf<typeof ProcessError>;
