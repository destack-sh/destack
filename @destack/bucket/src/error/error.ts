import type { DomainError, ServiceErrorCode, ServiceErrorReport } from "@destack/error";
import type { BucketFile } from "../bucket/file.ts";

/** The service error code of each storage failure: the request's own, a missing file, a fence a caller may retry after, or the host's. */
const SERVICE_CODES = {
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
    FENCED: "SERVICE_UNAVAILABLE",
    CLOSED: "SERVICE_UNAVAILABLE",
    BUSY: "SERVICE_UNAVAILABLE",
    WRITE_FAILED: "INTERNAL_SERVER_ERROR",
} as const satisfies Readonly<Record<string, ServiceErrorCode>>;

/** A failure code of file storage. */
export type BucketErrorCode = keyof typeof SERVICE_CODES;

/** A storage failure with a stable code. */
export class BucketError extends Error implements DomainError {
    /** The failure code. */
    readonly code: BucketErrorCode;

    /** Create a storage failure. */
    constructor(code: BucketErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "BucketError";
        this.code = code;
    }

    /** Refuse a read or write whose precondition the current file, or its absence, failed. */
    static preconditionFailed(current: BucketFile | null): BucketPreconditionError {
        return new BucketPreconditionError(current);
    }

    /** Convert the failure to the service error a caller receives. */
    toServiceError(): ServiceErrorReport {
        return { code: SERVICE_CODES[this.code], message: this.message };
    }
}

/** A read or write refused by its precondition, with the file it met, as S3 answers 412 Precondition Failed. */
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
