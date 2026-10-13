import type { DomainError, CallErrorCode, CallErrorReport } from "@destack/error";

/** The call error code of each composition failure: every one is the caller's. */
const CALL_CODES = {
    INVALID_ADDRESS: "BAD_REQUEST",
    INVALID_HEADER: "BAD_REQUEST",
    INVALID_KEY: "BAD_REQUEST",
    INVALID_DATE: "BAD_REQUEST",
} as const satisfies Readonly<Record<string, CallErrorCode>>;

/** Failures composing a message. */
export type MimeErrorCode = keyof typeof CALL_CODES;

/** A message composition failure with a stable code. */
export class MimeError extends Error implements DomainError {
    /** The failure code. */
    readonly code: MimeErrorCode;

    /** Create a message composition failure. */
    constructor(code: MimeErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "MimeError";
        this.code = code;
    }

    /** Convert the failure to the call error a caller receives. */
    toCallError(): CallErrorReport {
        return { code: CALL_CODES[this.code], message: this.message };
    }
}
