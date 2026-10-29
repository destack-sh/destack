import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The workflow package's failures, each reason with the call error code it answers. */
export const WorkflowError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    BODY_INVALID: { code: "BAD_REQUEST" },
    TIMING_INVALID: { code: "BAD_REQUEST" },
    EVENT_TYPE_MISSING: { code: "BAD_REQUEST" },

    // forbidden
    CHANGE_REQUEST_CLOSED: { code: "FORBIDDEN" },

    // not found
    PATH_MISMATCH: { code: "NOT_FOUND" },
    STEP_NOT_FOUND: { code: "NOT_FOUND" },
    TRIGGER_NOT_FOUND: { code: "NOT_FOUND" },

    // conflict
    RUN_STARTED: { code: "CONFLICT" },

    // gone
    CHANGES_COMPACTED: { code: "GONE" },

    // unprocessable content
    STEP_DUPLICATE: { code: "UNPROCESSABLE_CONTENT" },
    RUN_NONDETERMINISTIC: { code: "UNPROCESSABLE_CONTENT" },

    // client closed request
    RUN_CANCELLED: { code: "CLIENT_CLOSED_REQUEST" },

    // service unavailable
    TRIGGER_PAUSED: { code: "SERVICE_UNAVAILABLE" },
    TRIGGER_UNRECORDED: { code: "SERVICE_UNAVAILABLE" },

    // timeout
    RUN_TIMED_OUT: { code: "TIMEOUT" },
});

/** A stable cause of workflow failures. */
export type WorkflowReason = ReasonOf<typeof WorkflowError>;
