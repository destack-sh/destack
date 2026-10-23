import type { DomainError, CallErrorCode, Failure } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The call error code per sync failure: capacity, readiness, stream, scope, cycle, expiry. */
const CALL_CODES = {
    OVERLOADED: "SERVICE_UNAVAILABLE",
    OVER_CAPACITY: "UNPROCESSABLE_CONTENT",
    STALE: "SERVICE_UNAVAILABLE",
    INVALID_STREAM: "BAD_GATEWAY",
    INVALID_SCOPE: "UNPROCESSABLE_CONTENT",
    NOT_FOUND: "NOT_FOUND",
    CYCLE: "CONFLICT",
    EXPIRED: "PRECONDITION_FAILED",
} as const satisfies Readonly<Record<string, CallErrorCode>>;

/** A stable cause of failures of replicas, subscriptions and scopes. */
export type SyncReason = keyof typeof CALL_CODES;

/** A replica, subscription, scope or request the source cannot serve. */
export class SyncError extends Error implements DomainError {
    /** The failure's stable cause within its package. */
    readonly reason: SyncReason;

    /** Create the error with its reason and cause. */
    constructor(reason: SyncReason, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "SyncError";
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
