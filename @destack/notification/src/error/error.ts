import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The notification package's failures, each reason with the call error code it answers. */
export const NotificationError = defineErrorFamily({
    reasons: {
        // not found
        ACTION_TYPE_NOT_FOUND: {},

        // invalid
        ACTION_INVALID: {},
        NOTIFICATION_INVALID: {},
        PAYLOAD_INVALID: {},
        READ_IN_FUTURE_INVALID: {},
        SNOOZE_IN_PAST_INVALID: {},
        TEXT_INVALID: {},

        // denied
        RECIPIENT_DENIED: {},
    },
});

/** A stable cause of notification failures. */
export type NotificationReason = ReasonOf<typeof NotificationError>;
