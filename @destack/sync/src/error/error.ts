/** A subscription the source cannot serve. */
export class SyncError extends Error {
    /** The error classification. */
    readonly code: "OVERLOADED" | "OVER_CAPACITY" | "STALE";

    /** Create the error with its code and cause. */
    constructor(code: SyncError["code"], message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "SyncError";
        this.code = code;
    }
}
