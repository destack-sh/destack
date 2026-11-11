import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The registry package's failures, each reason with the call error code it answers. */
export const RegistryError = defineErrorFamily({
    reasons: {
        // not found
        PUBLISHER_NOT_FOUND: {},

        // exists
        RELEASE_EXISTS: {},

        // invalid
        MANIFEST_INVALID: {},
        ORIGIN_INVALID: {},
        TAG_INVALID: {},

        // blocked
        DEPENDENCY_BLOCKED: {},

        // unreachable
        REMOTE_UNREACHABLE: {},
    },
});

/** A stable cause of registry failures. */
export type RegistryReason = ReasonOf<typeof RegistryError>;
