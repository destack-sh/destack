import type { DomainError, CallErrorCode, CallErrorReport } from "@destack/error";

/** The call error code of each identity failure. */
const CALL_CODES = {
    INVALID_PUBLIC_KEY: "BAD_REQUEST",
    INVALID_PROOF: "BAD_REQUEST",
    INVALID_OPERATION: "BAD_REQUEST",
    INVALID_ROOT_KEY: "INTERNAL_SERVER_ERROR",
    KEY_UNAVAILABLE: "INTERNAL_SERVER_ERROR",
    DECRYPTION_FAILED: "INTERNAL_SERVER_ERROR",
} as const satisfies Readonly<Record<string, CallErrorCode>>;

/** A failure code of identities and their keys. */
export type IdentityErrorCode = keyof typeof CALL_CODES;

/** A failure of an identity or its keys, without their bytes. */
export class IdentityError extends Error implements DomainError {
    /** The stable failure code. */
    readonly code: IdentityErrorCode;

    /** Report a failure without its cryptographic inputs. */
    constructor(code: IdentityErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "IdentityError";
        this.code = code;
    }

    /** Convert the failure to the call error a caller receives. */
    toCallError(): CallErrorReport {
        return { code: CALL_CODES[this.code], message: this.message };
    }
}
