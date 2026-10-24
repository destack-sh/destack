import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The setting package's failures, each reason with the call error code it answers. */
export const SettingError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    PLACEMENT_INVALID: { code: "BAD_REQUEST" },
    VALUE_INVALID: { code: "BAD_REQUEST" },

    // not found
    SETTING_UNDECLARED: { code: "NOT_FOUND" },
    SCOPE_UNKNOWN: { code: "NOT_FOUND" },

    // conflict
    VALUES_CONFLICTING: { code: "CONFLICT" },

    // precondition failed
    DEFAULT_MISSING: { code: "PRECONDITION_FAILED" },
    SETTING_INVALID: { code: "PRECONDITION_FAILED" },
    SETTING_UNSET: { code: "PRECONDITION_FAILED" },
});

/** A stable cause of setting failures. */
export type SettingReason = ReasonOf<typeof SettingError>;
