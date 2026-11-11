import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The registry package's failures, each reason with the call error code it answers. */
export const RegistryError = defineErrorFamily(import.meta.destack.package, {
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
});

/** A stable cause of registry failures. */
export type RegistryReason = ReasonOf<typeof RegistryError>;
