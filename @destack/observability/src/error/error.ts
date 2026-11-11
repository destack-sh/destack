import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The observability package's failures, each reason with the call error code it answers. */
export const ObservabilityError = defineErrorFamily({
    reasons: {
        // invalid
        EVENTS_TYPE_INVALID: {},
        FINGERPRINT_INVALID: {},
        FRAMES_INVALID: {},
        ISSUE_INVALID: {},
        METRIC_INVALID: {},
        REVISION_INVALID: {},
        SESSION_STATUS_INVALID: {},
        SPAN_TIMES_INVALID: {},
        TEMPORALITY_INVALID: {},
        UNTIL_INVALID: {},

        // required
        ACTION_NAME_REQUIRED: {},
        CALL_GROUP_REQUIRED: {},
        DISTRIBUTION_REQUIRED: {},
        LOG_EVENT_REQUIRED: {},
        LOG_TIME_REQUIRED: {},
        MEASURE_REQUIRED: {},
        POINT_VALUE_REQUIRED: {},
        SESSION_STATUS_REQUIRED: {},
        VISIT_ROUTE_REQUIRED: {},

        // denied
        PERSON_DENIED: {},

        // blocked
        RESPOND_CAPABILITY_BLOCKED: {},
    },
});

/** A stable cause of observability failures. */
export type ObservabilityReason = ReasonOf<typeof ObservabilityError>;
