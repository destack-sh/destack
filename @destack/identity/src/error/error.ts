import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The identity package's failures, each reason with the call error code it answers. */
export const IdentityError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    OPERATION_INVALID: { code: "BAD_REQUEST" },
    PROOF_INVALID: { code: "BAD_REQUEST" },
    KEY_INVALID: { code: "BAD_REQUEST" },
    CIPHERTEXT_MALFORMED: { code: "BAD_REQUEST" },

    // unauthorized
    TOKEN_MALFORMED: { code: "UNAUTHORIZED" },
    ALGORITHM_REFUSED: { code: "UNAUTHORIZED" },
    KEY_NOT_FOUND: { code: "UNAUTHORIZED" },
    SIGNATURE_INVALID: { code: "UNAUTHORIZED" },
    TOKEN_EXPIRED: { code: "UNAUTHORIZED" },
    TOKEN_NOT_YET_VALID: { code: "UNAUTHORIZED" },
    ISSUER_REFUSED: { code: "UNAUTHORIZED" },
    AUDIENCE_REFUSED: { code: "UNAUTHORIZED" },
    CLAIM_INVALID: { code: "UNAUTHORIZED" },

    // internal server error
    DECRYPTION_FAILED: { code: "INTERNAL_SERVER_ERROR" },
    ROOT_KEY_INVALID: { code: "INTERNAL_SERVER_ERROR" },
    KEY_UNAVAILABLE: { code: "INTERNAL_SERVER_ERROR" },
    KEY_SET_UNAVAILABLE: { code: "SERVICE_UNAVAILABLE" },
});

/** A stable cause of identity failures. */
export type IdentityReason = ReasonOf<typeof IdentityError>;
