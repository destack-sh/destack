import type { DomainError, ServiceErrorCode, ServiceErrorReport } from "@destack/error";

/** The service error code per sync failure: capacity, unready copy, bad stream or scope, cycle. */
const SERVICE_CODES = {
    OVERLOADED: "SERVICE_UNAVAILABLE",
    OVER_CAPACITY: "UNPROCESSABLE_CONTENT",
    STALE: "SERVICE_UNAVAILABLE",
    INVALID_STREAM: "BAD_GATEWAY",
    INVALID_SCOPE: "UNPROCESSABLE_CONTENT",
    NOT_FOUND: "NOT_FOUND",
    CYCLE: "CONFLICT",
} as const satisfies Readonly<Record<string, ServiceErrorCode>>;

/** A failure code of copies, subscriptions and scopes. */
export type SyncErrorCode = keyof typeof SERVICE_CODES;

/** A copy, subscription or scope the source cannot serve. */
export class SyncError extends Error implements DomainError {
    /** The error classification. */
    readonly code: SyncErrorCode;

    /** Create the error with its code and cause. */
    constructor(code: SyncErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "SyncError";
        this.code = code;
    }

    /** Convert the failure to the service error a caller receives. */
    toServiceError(): ServiceErrorReport {
        return { code: SERVICE_CODES[this.code], message: this.message };
    }
}
