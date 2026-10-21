import type { CallErrorCode, Failure, DomainError } from "@destack/error";

/** The call error code per event failure: bad cursor, other contents, racing compaction. */
const CALL_CODES = {
    INVALID_CURSOR: "BAD_REQUEST",
    CONFLICT: "CONFLICT",
    CONCURRENT_UPDATE: "SERVICE_UNAVAILABLE",
} as const satisfies Readonly<Record<string, CallErrorCode>>;

/** A failure code of event reads and appends. */
export type EventErrorCode = keyof typeof CALL_CODES;

/** A read or append the event store refuses. */
export class EventError extends Error implements DomainError {
    /** The error classification. */
    readonly code: EventErrorCode;

    /** Create the error with its code and cause. */
    constructor(code: EventErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "EventError";
        this.code = code;
    }

    /** Convert the failure to the call error a caller receives. */
    toCallError(): Failure {
        return { code: CALL_CODES[this.code], message: this.message };
    }
}
