import type { DomainError, CallErrorCode, Failure } from "@destack/error";
import type { BucketFile } from "../bucket/file.ts";
import type {} from "@destack/package/import-meta";

/** The call error code of each storage failure. */
const CALL_CODES = {
    INVALID_KEY: "BAD_REQUEST",
    INVALID_RANGE: "BAD_REQUEST",
    INVALID_CURSOR: "BAD_REQUEST",
    INVALID_LIMIT: "BAD_REQUEST",
    INVALID_PART: "BAD_REQUEST",
    INVALID_CHECKSUM: "BAD_REQUEST",
    INVALID_STORAGE_CLASS: "BAD_REQUEST",
    INVALID_CUSTOMER_KEY: "BAD_REQUEST",
    INCOMPLETE_BODY: "BAD_REQUEST",
    PRECONDITION_FAILED: "PRECONDITION_FAILED",
    NO_SUCH_KEY: "NOT_FOUND",
    NO_SUCH_UPLOAD: "NOT_FOUND",
    NO_SUCH_BUCKET: "NOT_FOUND",
    LOCKED: "FORBIDDEN",
    UNSUPPORTED: "NOT_IMPLEMENTED",
    FILE_NOT_CLONABLE: "UNPROCESSABLE_CONTENT",
    FENCED: "SERVICE_UNAVAILABLE",
    CLOSED: "SERVICE_UNAVAILABLE",
    BUSY: "SERVICE_UNAVAILABLE",
    WRITE_FAILED: "INTERNAL_SERVER_ERROR",
} as const satisfies Readonly<Record<string, CallErrorCode>>;

/** A stable cause of failures of file storage. */
export type BucketReason = keyof typeof CALL_CODES;

/** A storage failure with its reason. */
export class BucketError extends Error implements DomainError {
    /** The failure's stable cause within its package. */
    readonly reason: BucketReason;

    /** Create a storage failure. */
    constructor(reason: BucketReason, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "BucketError";
        this.reason = reason;
    }

    /** Report whether a value is a stable cause of file storage failures. */
    static isReason(value: string): value is BucketReason {
        return Object.hasOwn(CALL_CODES, value);
    }

    /** Refuse a read or write whose precondition the current file, or its absence, failed. */
    static preconditionFailed(current: BucketFile | null): BucketPreconditionError {
        return new BucketPreconditionError(current);
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

/** A read or write refused by its precondition, with the file it met. */
export class BucketPreconditionError extends BucketError {
    /** The file the precondition met, null where none exists. */
    readonly current: BucketFile | null;

    /** Create the refusal of a precondition the current file, or its absence, failed. */
    constructor(current: BucketFile | null) {
        super("PRECONDITION_FAILED", "at least one of the preconditions did not hold");
        this.name = "BucketPreconditionError";
        this.current = current;
    }
}
