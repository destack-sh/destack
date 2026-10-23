import type { DomainError, CallErrorCode, Failure } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The call error code of each sandbox failure: an unsupported restriction, or the host's. */
const CALL_CODES = {
    UNSUPPORTED: "NOT_IMPLEMENTED",
    START_FAILED: "INTERNAL_SERVER_ERROR",
    STOP_FAILED: "INTERNAL_SERVER_ERROR",
} as const satisfies Readonly<Record<string, CallErrorCode>>;

/** The stable causes of sandbox failures. */
export type SandboxReason = keyof typeof CALL_CODES;

/** A process could not start or stop under the requested OS restrictions. */
export class SandboxError extends Error implements DomainError {
    /** The failure's stable cause within its package. */
    readonly reason: SandboxReason;

    /** Retain the operation and its underlying failure. */
    constructor(reason: SandboxReason, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "SandboxError";
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
