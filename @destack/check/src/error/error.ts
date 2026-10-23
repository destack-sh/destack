import type { DomainError, CallErrorCode, Failure } from "@destack/error";

/** The domain of check failures, written out: check runs outside the Destack build. */
const DOMAIN = "@destack/check";

/** The call error code of each check failure. */
const CALL_CODES = {
    TOOL_FAILED: "INTERNAL_SERVER_ERROR",
    INVALID_CONFIGURATION: "BAD_REQUEST",
    LANGUAGE_SERVICE_FAILED: "INTERNAL_SERVER_ERROR",
} as const satisfies Readonly<Record<string, CallErrorCode>>;

/** The stable causes of check failures. */
export type CheckReason = keyof typeof CALL_CODES;

/** A failed checker invocation or unsupported package. */
export class CheckError extends Error implements DomainError {
    /** The failure's stable cause within its package. */
    readonly reason: CheckReason;

    /** Identify the failed operation. */
    constructor(reason: CheckReason, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "CheckError";
        this.reason = reason;
    }

    /** Convert the failure to the call error a caller receives. */
    toCallError(): Failure {
        return {
            code: CALL_CODES[this.reason],
            reason: this.reason,
            domain: DOMAIN,
            message: this.message,
        };
    }
}
