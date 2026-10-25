import { defineErrorFamily, type ReasonOf } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The vault package's failures, each reason with the call error code it answers. */
export const VaultError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    CURRENT_VERSION_REQUIRED: { code: "BAD_REQUEST" },
    EXPIRY_IN_PAST: { code: "BAD_REQUEST" },
    TEXT_REQUIRED: { code: "BAD_REQUEST" },
    VAULT_UNBOUND: { code: "BAD_REQUEST" },

    // not found
    CURRENT_VERSION_MISSING: { code: "NOT_FOUND" },
    SECRET_NOT_FOUND: { code: "NOT_FOUND" },
    VERSION_NOT_FOUND: { code: "NOT_FOUND" },

    // conflict
    SECRET_MISMATCH: { code: "CONFLICT" },
    VERSION_DESTROYED: { code: "CONFLICT" },
    VERSION_UNAVAILABLE: { code: "CONFLICT" },

    // payload too large
    VALUE_TOO_LARGE: { code: "PAYLOAD_TOO_LARGE" },

    // internal server error
    VERSION_MISMATCH: { code: "INTERNAL_SERVER_ERROR" },
});

/** A stable cause of vault failures. */
export type VaultReason = ReasonOf<typeof VaultError>;
