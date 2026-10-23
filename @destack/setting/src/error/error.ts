import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The setting package's failures, each reason with the call error code it answers. */
export const SettingError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    INVALID_PLACEMENT: { code: "BAD_REQUEST" },
    INVALID_VALUE: { code: "BAD_REQUEST" },

    // not found
    UNDECLARED: { code: "NOT_FOUND" },
    UNKNOWN_SCOPE: { code: "NOT_FOUND" },

    // conflict
    CONFLICTING_VALUES: { code: "CONFLICT" },
});

/** A stable cause of setting failures. */
export type SettingReason = ReasonOf<typeof SettingError>;
