import type { DomainError, CallErrorCode, CallErrorReport } from "@destack/error";

/** The call error code of each stack failure. */
const CALL_CODES = {
    INVALID_DEFINITION: "BAD_REQUEST",
    UNSERVED: "PRECONDITION_FAILED",
    NOT_READY: "SERVICE_UNAVAILABLE",
} as const satisfies Readonly<Record<string, CallErrorCode>>;

/** A failure code of a stack and the stack package. */
export type StackErrorCode = keyof typeof CALL_CODES;

/** An invalid stack, one this machine cannot serve, or a resource not ready yet. */
export class StackError extends Error implements DomainError {
    /** Stable failure code. */
    readonly code: StackErrorCode;

    /** Create a stack failure. */
    constructor(code: StackErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "StackError";
        this.code = code;
    }

    /** Convert the failure to the call error a caller receives. */
    toCallError(): CallErrorReport {
        return { code: CALL_CODES[this.code], message: this.message };
    }
}
