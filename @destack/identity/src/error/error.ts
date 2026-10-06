import type { DomainError, ServiceErrorCode, ServiceErrorReport } from "@destack/error";

/** The service error code of each identity failure: a caller's malformed key, proof or operation, or the host's keys. */
const SERVICE_CODES = {
    INVALID_PUBLIC_KEY: "BAD_REQUEST",
    INVALID_PROOF: "BAD_REQUEST",
    INVALID_OPERATION: "BAD_REQUEST",
    INVALID_ROOT_KEY: "INTERNAL_SERVER_ERROR",
    KEY_UNAVAILABLE: "INTERNAL_SERVER_ERROR",
    DECRYPTION_FAILED: "INTERNAL_SERVER_ERROR",
} as const satisfies Readonly<Record<string, ServiceErrorCode>>;

/** A failure code of identities and the keys keeping them. */
export type IdentityErrorCode = keyof typeof SERVICE_CODES;

/** A failure of an identity or the keys keeping it, without their bytes. */
export class IdentityError extends Error implements DomainError {
    /** The stable failure code. */
    readonly code: IdentityErrorCode;

    /** Report a failure without its cryptographic inputs. */
    constructor(code: IdentityErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "IdentityError";
        this.code = code;
    }

    /** Convert the failure to the service error a caller receives. */
    toServiceError(): ServiceErrorReport {
        return { code: SERVICE_CODES[this.code], message: this.message };
    }
}
