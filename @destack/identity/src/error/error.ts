import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The identity package's failures, each reason with the call error code it answers. */
export const IdentityError = defineErrorFamily(import.meta.destack.package, {
    // invalid
    CIPHERTEXT_INVALID: {},
    KEY_INVALID: {},
    OPERATION_INVALID: {},
    PROOF_INVALID: {},

    // unauthenticated
    ALGORITHM_UNAUTHENTICATED: {},
    AUDIENCE_UNAUTHENTICATED: {},
    CLAIM_UNAUTHENTICATED: {},
    ISSUER_UNAUTHENTICATED: {},
    KEY_UNAUTHENTICATED: {},
    SIGNATURE_UNAUTHENTICATED: {},
    TOKEN_UNAUTHENTICATED: {},

    // unavailable
    KEY_SET_UNAVAILABLE: {},
    KMS_UNAVAILABLE: {},

    // broken
    DECRYPTION_BROKEN: {},
    KEY_BROKEN: {},
    KMS_BROKEN: {},
    ROOT_KEY_BROKEN: {},
});

/** A stable cause of identity failures. */
export type IdentityReason = ReasonOf<typeof IdentityError>;
