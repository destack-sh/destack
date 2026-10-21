import type { DomainError, CallErrorCode, Failure } from "@destack/error";

/** The call error code of each object failure: a declaration's failures are the host's. */
const CALL_CODES = {
    INVALID_DECLARATION: "INTERNAL_SERVER_ERROR",
    UNSUPPORTED_DECLARATION: "INTERNAL_SERVER_ERROR",
    CYCLIC_DECLARATION: "INTERNAL_SERVER_ERROR",
    INVALID_CALL: "BAD_REQUEST",
    CONFLICT: "CONFLICT",
    FORBIDDEN: "FORBIDDEN",
    UNAVAILABLE: "SERVICE_UNAVAILABLE",
} as const satisfies Readonly<Record<string, CallErrorCode>>;

/** Stable object failure codes. */
export type ObjectErrorCode = keyof typeof CALL_CODES;

/** An object failure with a stable code. */
export class ObjectError extends Error implements DomainError {
    /** The failure code. */
    readonly code: ObjectErrorCode;

    /** Create an object failure with its code and cause. */
    constructor(code: ObjectErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "ObjectError";
        this.code = code;
    }

    /** Convert the failure to the call error a caller receives. */
    toCallError(): Failure {
        return { code: CALL_CODES[this.code], message: this.message };
    }
}
