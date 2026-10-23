import type { CallErrorCode, Failure, DomainError } from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The call error code per event failure: bad cursor, other contents, racing compaction. */
const CALL_CODES = {
    INVALID_CURSOR: "BAD_REQUEST",
    CONTENTS_DIFFER: "CONFLICT",
    CONCURRENT_UPDATE: "SERVICE_UNAVAILABLE",
} as const satisfies Readonly<Record<string, CallErrorCode>>;

/** A stable cause of failures of event reads and appends. */
export type EventReason = keyof typeof CALL_CODES;

/** A read or append the event store refuses. */
export class EventError extends Error implements DomainError {
    /** The failure's stable cause within its package. */
    readonly reason: EventReason;

    /** Create the error with its reason and cause. */
    constructor(reason: EventReason, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "EventError";
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
