import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The setting package's failures, each reason with the call error code it answers. */
export const SettingError = defineErrorFamily({
    reasons: {
        // not found
        SCOPE_NOT_FOUND: {},
        SETTING_NOT_FOUND: {},

        // invalid
        PLACEMENT_INVALID: {},
        VALUES_INVALID: {},
        VALUE_INVALID: {},

        // blocked
        DEFAULT_BLOCKED: {},
        SETTING_BLOCKED: {},
    },
});

/** A stable cause of setting failures. */
export type SettingReason = ReasonOf<typeof SettingError>;
