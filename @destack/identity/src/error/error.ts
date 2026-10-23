import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The identity package's failures, each reason with the call error code it answers. */
export const IdentityError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    INVALID_OPERATION: { code: "BAD_REQUEST" },
    INVALID_PROOF: { code: "BAD_REQUEST" },
    INVALID_PUBLIC_KEY: { code: "BAD_REQUEST" },

    // internal server error
    DECRYPTION_FAILED: { code: "INTERNAL_SERVER_ERROR" },
    INVALID_ROOT_KEY: { code: "INTERNAL_SERVER_ERROR" },
    KEY_UNAVAILABLE: { code: "INTERNAL_SERVER_ERROR" },
});

/** A stable cause of identity failures. */
export type IdentityReason = ReasonOf<typeof IdentityError>;
