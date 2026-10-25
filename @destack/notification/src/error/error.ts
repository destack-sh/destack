import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The notification package's failures, each reason with the call error code it answers. */
export const NotificationError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    READ_IN_FUTURE: { code: "BAD_REQUEST" },
    SNOOZE_IN_PAST: { code: "BAD_REQUEST" },
    TEXT_UNEXPECTED: { code: "BAD_REQUEST" },
    ACTION_UNKNOWN: { code: "BAD_REQUEST" },
    NOTIFICATION_UNKNOWN: { code: "BAD_REQUEST" },

    // forbidden
    RECIPIENT_FORBIDDEN: { code: "FORBIDDEN" },

    // payload too large
    PAYLOAD_TOO_LARGE: { code: "PAYLOAD_TOO_LARGE" },

    // not found
    ACTION_TYPE_NOT_SERVED: { code: "NOT_FOUND" },
    INSTALLATION_NOT_FOUND: { code: "NOT_FOUND" },
});

/** A stable cause of notification failures. */
export type NotificationReason = ReasonOf<typeof NotificationError>;
