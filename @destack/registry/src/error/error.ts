import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The registry package's failures, each reason with the call error code it answers. */
export const RegistryError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    FOREIGN_MANIFEST: { code: "BAD_REQUEST" },
    ORIGIN_INVALID: { code: "BAD_REQUEST" },
    TAG_INVALID: { code: "BAD_REQUEST" },

    // not found
    PUBLISHER_MISSING: { code: "NOT_FOUND" },

    // conflict
    RELEASE_CONFLICT: { code: "CONFLICT" },

    // bad gateway
    REMOTE_FAILED: { code: "BAD_GATEWAY" },
});

/** A stable cause of registry failures. */
export type RegistryReason = ReasonOf<typeof RegistryError>;
