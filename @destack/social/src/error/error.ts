import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The social package's failures, each reason with the call error code it answers. */
export const SocialError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    MENTIONS_INVALID: { code: "BAD_REQUEST" },
    FIRST_COMMENT_REQUIRED: { code: "BAD_REQUEST" },
    SELECTION_UNKNOWN: { code: "BAD_REQUEST" },
    TEXT_FIELD_UNKNOWN: { code: "BAD_REQUEST" },

    // conflict
    THREAD_OPEN: { code: "CONFLICT" },
    THREAD_RESOLVED: { code: "CONFLICT" },

    // payload too large
    LOCATION_TOO_LARGE: { code: "PAYLOAD_TOO_LARGE" },
});

/** A stable cause of social failures. */
export type SocialReason = ReasonOf<typeof SocialError>;
