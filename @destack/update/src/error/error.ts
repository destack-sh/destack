import type { DomainError, CallErrorCode, Failure } from "@destack/error";

/** The call error code of each update failure. */
const CALL_CODES = {
    BUSY: "CONFLICT",
    CLOSED: "SERVICE_UNAVAILABLE",
    RELEASE: "BAD_GATEWAY",
    REPOSITORY: "BAD_GATEWAY",
    DOWNLOAD: "BAD_GATEWAY",
    ACTIVATION: "INTERNAL_SERVER_ERROR",
    INSTALL: "INTERNAL_SERVER_ERROR",
} as const satisfies Readonly<Record<string, CallErrorCode>>;

/** Failures detected by Destack's update and installation logic. */
export type UpdateErrorCode = keyof typeof CALL_CODES;

/** A failure reported by the distribution updater. */
export class UpdateError extends Error implements DomainError {
    /** Stable failure category for native callers. */
    readonly code: UpdateErrorCode;

    /** Describe an update failure and retain its underlying cause. */
    constructor(code: UpdateErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "UpdateError";
        this.code = code;
    }

    /** Convert the failure to the call error a caller receives. */
    toCallError(): Failure {
        return { code: CALL_CODES[this.code], message: this.message };
    }
}
