import type { DomainError, CallErrorCode, CallErrorReport } from "@destack/error";

/** The call error code per sync failure: capacity, unready replica, bad stream or scope, cycle. */
const CALL_CODES = {
    OVERLOADED: "SERVICE_UNAVAILABLE",
    OVER_CAPACITY: "UNPROCESSABLE_CONTENT",
    STALE: "SERVICE_UNAVAILABLE",
    INVALID_STREAM: "BAD_GATEWAY",
    INVALID_SCOPE: "UNPROCESSABLE_CONTENT",
    NOT_FOUND: "NOT_FOUND",
    CYCLE: "CONFLICT",
} as const satisfies Readonly<Record<string, CallErrorCode>>;

/** A failure code of replicas, subscriptions and scopes. */
export type SyncErrorCode = keyof typeof CALL_CODES;

/** A replica, subscription or scope the source cannot serve. */
export class SyncError extends Error implements DomainError {
    /** The error classification. */
    readonly code: SyncErrorCode;

    /** Create the error with its code and cause. */
    constructor(code: SyncErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "SyncError";
        this.code = code;
    }

    /** Convert the failure to the call error a caller receives. */
    toCallError(): CallErrorReport {
        return { code: CALL_CODES[this.code], message: this.message };
    }
}
