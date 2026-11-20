import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The universe package's failures, each reason with the call error code it answers. */
export const UniverseError = defineErrorFamily({
    reasons: {
        // unavailable
        DEFINITION_UNAVAILABLE: {},

        // broken
        ISSUER_BROKEN: {},
    },
});

/** A stable cause of universe failures. */
export type UniverseReason = ReasonOf<typeof UniverseError>;
