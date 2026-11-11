import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The vault package's failures, each reason with the call error code it answers. */
export const VaultError = defineErrorFamily({
    reasons: {
        // not found
        CURRENT_VERSION_NOT_FOUND: {},
        SECRET_NOT_FOUND: {},
        VERSION_NOT_FOUND: {},

        // invalid
        EXPIRY_IN_PAST_INVALID: {},
        VALUE_INVALID: {},

        // required
        CURRENT_VERSION_REQUIRED: {},
        VALUE_REQUIRED: {},

        // blocked
        SECRET_BLOCKED: {},
        VERSION_BLOCKED: {},

        // broken
        VERSION_BROKEN: {},
    },
});

/** A stable cause of vault failures. */
export type VaultReason = ReasonOf<typeof VaultError>;
