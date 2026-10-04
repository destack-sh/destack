/** Failures reported by file storage. */
export type StorageErrorCode =
    | "INVALID_KEY"
    | "INVALID_RANGE"
    | "INVALID_CURSOR"
    | "INVALID_LIMIT"
    | "INVALID_PART"
    | "INVALID_CHECKSUM"
    | "INVALID_STORAGE_CLASS"
    | "INVALID_CUSTOMER_KEY"
    | "INCOMPLETE_BODY"
    | "WRITE_FAILED"
    | "NO_SUCH_KEY"
    | "NO_SUCH_UPLOAD"
    | "NO_SUCH_BUCKET"
    | "UNSUPPORTED"
    | "CLOSED"
    | "BUSY"
    | "FENCED";

/** A storage failure with a stable code. */
export class StorageError extends Error {
    /** The failure code. */
    readonly code: StorageErrorCode;

    /** Create a storage failure. */
    constructor(code: StorageErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "StorageError";
        this.code = code;
    }
}
