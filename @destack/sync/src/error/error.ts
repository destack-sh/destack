/** A copy, subscription or scope the source cannot serve. */
export class SyncError extends Error {
    /** The error classification. */
    readonly code:
        | "OVERLOADED"
        | "OVER_CAPACITY"
        | "STALE"
        | "INVALID_STREAM"
        | "INVALID_SCOPE"
        | "NOT_FOUND";

    /** Create the error with its code and cause. */
    constructor(code: SyncError["code"], message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "SyncError";
        this.code = code;
    }
}
