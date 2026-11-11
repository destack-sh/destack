import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The workflow package's failures, each reason with the call error code it answers. */
export const WorkflowError = defineErrorFamily({
    reasons: {
        // not found
        CHANGES_NOT_FOUND: {},
        PATH_NOT_FOUND: {},
        STEP_NOT_FOUND: {},

        // invalid
        BODY_INVALID: {},
        RUN_NONDETERMINISTIC_INVALID: {},
        STEP_INVALID: {},
        TIMING_INVALID: {},
        TRIGGER_INVALID: {},

        // required
        EVENT_TYPE_REQUIRED: {},

        // denied
        CHANGE_REQUEST_DENIED: {},

        // blocked
        RUN_BLOCKED: {},

        // unavailable
        TRIGGER_UNAVAILABLE: {},

        // unreachable
        RUN_UNREACHABLE: {},

        // cancelled
        RUN_CANCELLED: {},
    },
});

/** A stable cause of workflow failures. */
export type WorkflowReason = ReasonOf<typeof WorkflowError>;
