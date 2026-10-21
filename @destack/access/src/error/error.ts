import type { DomainError, CallErrorCode, Failure } from "@destack/error";
import type { StepUp } from "../context/elevation.ts";

/** The call error code of each access failure. */
const CALL_CODES = {
    INVALID_DECLARATION: "INTERNAL_SERVER_ERROR",
    INVALID_CONTEXT: "FORBIDDEN",
    NOT_FOUND: "NOT_FOUND",
    FORBIDDEN: "FORBIDDEN",
    UNAUTHORIZED: "UNAUTHORIZED",
    INSUFFICIENT_AUTHENTICATION: "INSUFFICIENT_AUTHENTICATION",
    CONFLICT: "CONFLICT",
    STALE: "SERVICE_UNAVAILABLE",
    INVALID_EXCHANGE: "BAD_REQUEST",
    INSECURE_KEYS: "PRECONDITION_FAILED",
    KEYS_UNAVAILABLE: "SERVICE_UNAVAILABLE",
} as const satisfies Readonly<Record<string, CallErrorCode>>;

/** A failure code of access decisions and declarations. */
export type AccessErrorCode = keyof typeof CALL_CODES;

/** Reject an invalid declaration, unavailable record, or unauthorized operation. */
export class AccessError extends Error implements DomainError {
    /** The stable failure classification. */
    readonly code: AccessErrorCode;
    /** The authentication that would allow the caller, for insufficient authentication. */
    readonly stepUp?: StepUp;

    /** Retain the classification, cause, and the authentication that would allow the caller. */
    constructor(
        code: AccessErrorCode,
        message: string,
        options?: ErrorOptions & { readonly stepUp?: StepUp },
    ) {
        // classify the error
        super(message, options);
        this.name = "AccessError";
        this.code = code;

        // take the refusal's challenge
        if (options?.stepUp !== undefined) {
            this.stepUp = options.stepUp;
        }
    }

    /** Convert the failure to the caller's call error, with its step-up challenge. */
    toCallError(): Failure {
        const code = CALL_CODES[this.code];

        return this.stepUp === undefined
            ? { code, message: this.message }
            : { code, message: this.message, data: this.stepUp };
    }
}
