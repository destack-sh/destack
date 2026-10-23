import type { DomainError, CallErrorCode, Failure } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The call error code of each identity failure. */
const CALL_CODES = {
    INVALID_PUBLIC_KEY: "BAD_REQUEST",
    INVALID_PROOF: "BAD_REQUEST",
    INVALID_OPERATION: "BAD_REQUEST",
    INVALID_ROOT_KEY: "INTERNAL_SERVER_ERROR",
    KEY_UNAVAILABLE: "INTERNAL_SERVER_ERROR",
    DECRYPTION_FAILED: "INTERNAL_SERVER_ERROR",
} as const satisfies Readonly<Record<string, CallErrorCode>>;

/** A stable cause of failures of identities and their keys. */
export type IdentityReason = keyof typeof CALL_CODES;

/** A failure of an identity or its keys, without their bytes. */
export class IdentityError extends Error implements DomainError {
    /** The failure's stable cause within its package. */
    readonly reason: IdentityReason;

    /** Report a failure without its cryptographic inputs. */
    constructor(reason: IdentityReason, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "IdentityError";
        this.reason = reason;
    }

    /** Convert the failure to the call error a caller receives. */
    toCallError(): Failure {
        return {
            code: CALL_CODES[this.reason],
            reason: this.reason,
            domain: import.meta.destack.package.name,
            message: this.message,
        };
    }
}
