import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The control package's failures, each reason with the call error code it answers. */
export const ControlError = defineErrorFamily({
    reasons: {
        // unavailable
        DEFINITION_UNAVAILABLE: {},

        // broken
        ISSUER_BROKEN: {},
    },
});

/** A stable cause of control failures. */
export type ControlReason = ReasonOf<typeof ControlError>;
