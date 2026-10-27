import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The observability package's failures, each reason with the call error code it answers. */
export const ObservabilityError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    FINGERPRINT_INVALID: { code: "BAD_REQUEST" },
    FRAMES_INVALID: { code: "BAD_REQUEST" },
    SESSION_STATUS_INVALID: { code: "BAD_REQUEST" },
    SPAN_TIMES_INVALID: { code: "BAD_REQUEST" },
    ACTION_NAME_MISSING: { code: "BAD_REQUEST" },
    CALL_GROUP_MISSING: { code: "BAD_REQUEST" },
    DISTRIBUTION_MISSING: { code: "BAD_REQUEST" },
    LOG_EVENT_MISSING: { code: "BAD_REQUEST" },
    LOG_TIME_MISSING: { code: "BAD_REQUEST" },
    MEASURE_MISSING: { code: "BAD_REQUEST" },
    POINT_VALUE_MISSING: { code: "BAD_REQUEST" },
    SESSION_STATUS_MISSING: { code: "BAD_REQUEST" },
    VISIT_ROUTE_MISSING: { code: "BAD_REQUEST" },
    EVENTS_TYPE_UNKNOWN: { code: "BAD_REQUEST" },
    ISSUE_PENDING_UNSUPPORTED: { code: "BAD_REQUEST" },
    METRIC_UNSUPPORTED: { code: "BAD_REQUEST" },
    TEMPORALITY_UNSUPPORTED: { code: "BAD_REQUEST" },
    REVISION_UNKNOWN: { code: "BAD_REQUEST" },
    UNTIL_PASSED: { code: "BAD_REQUEST" },

    // forbidden
    EMITTER_ATTRIBUTE_RESERVED: { code: "FORBIDDEN" },
    PERSON_UNVERIFIED: { code: "FORBIDDEN" },

    // precondition failed
    RESPOND_CAPABILITY_MISSING: { code: "PRECONDITION_FAILED" },
});

/** A stable cause of observability failures. */
export type ObservabilityReason = ReasonOf<typeof ObservabilityError>;
