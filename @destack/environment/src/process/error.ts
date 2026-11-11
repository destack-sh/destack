import { defineErrorFamily, type ReasonOf } from "@destack/error";

/** The package owning the processes' failures, named here as configurations load it unbuilt. */
const OWNER = { name: "@destack/environment" };

/** The processes' failures, each reason with the call error code it answers. */
export const ProcessError = defineErrorFamily(
    OWNER,
    {
        // not found
        FILE_NOT_FOUND: {},
        PROCESS_NOT_FOUND: {},

        // denied
        PERMISSION_DENIED: {},

        // broken
        PROCESS_BROKEN: {},
    },
    { audience: "host" },
);

/** A stable cause of process failures. */
export type ProcessReason = ReasonOf<typeof ProcessError>;
