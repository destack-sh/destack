import type { DomainError, CallErrorCode, CallErrorReport } from "@destack/error";

/** The call error code of each sandbox failure: an unsupported restriction, or the host's. */
const CALL_CODES = {
    UNSUPPORTED: "NOT_IMPLEMENTED",
    START_FAILED: "INTERNAL_SERVER_ERROR",
    STOP_FAILED: "INTERNAL_SERVER_ERROR",
} as const satisfies Readonly<Record<string, CallErrorCode>>;

/** A failed sandbox operation. */
export type SandboxErrorCode = keyof typeof CALL_CODES;

/** A process could not start or stop under the requested OS restrictions. */
export class SandboxError extends Error implements DomainError {
    /** The failed sandbox operation. */
    readonly code: SandboxErrorCode;

    /** Retain the operation and its underlying failure. */
    constructor(code: SandboxErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "SandboxError";
        this.code = code;
    }

    /** Convert the failure to the call error a caller receives. */
    toCallError(): CallErrorReport {
        return { code: CALL_CODES[this.code], message: this.message };
    }
}
