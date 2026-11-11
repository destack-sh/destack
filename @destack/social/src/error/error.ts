import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The social package's failures, each reason with the call error code it answers. */
export const SocialError = defineErrorFamily(import.meta.destack.package, {
    // invalid
    LOCATION_INVALID: {},
    MENTIONS_INVALID: {},
    SELECTION_INVALID: {},
    TEXT_FIELD_INVALID: {},

    // required
    FIRST_COMMENT_REQUIRED: {},

    // blocked
    THREAD_BLOCKED: {},
});

/** A stable cause of social failures. */
export type SocialReason = ReasonOf<typeof SocialError>;
