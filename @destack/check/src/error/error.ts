import type { DomainError, CallErrorCode, Failure } from "@destack/error";

/** The call error code of each check failure. */
const CALL_CODES = {
    TOOL: "INTERNAL_SERVER_ERROR",
    CONFIGURATION: "BAD_REQUEST",
    LANGUAGE: "UNPROCESSABLE_CONTENT",
} as const satisfies Readonly<Record<string, CallErrorCode>>;

/** A failure category of checks. */
export type CheckErrorCode = keyof typeof CALL_CODES;

/** A failed checker invocation or unsupported package. */
export class CheckError extends Error implements DomainError {
    /** Failure category. */
    readonly code: CheckErrorCode;

    /** Identify the failed operation. */
    constructor(code: CheckErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "CheckError";
        this.code = code;
    }

    /** Convert the failure to the call error a caller receives. */
    toCallError(): Failure {
        return { code: CALL_CODES[this.code], message: this.message };
    }
}
