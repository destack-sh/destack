/** Reject a subscription the source cannot serve: now, while overloaded, or at all, once it holds too much. */
export class SyncError extends Error {
    /** The stable failure classification. */
    readonly code: "OVERLOADED" | "OVER_CAPACITY" | "STALE";

    /** Retain the error classification and cause. */
    constructor(code: SyncError["code"], message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "SyncError";
        this.code = code;
    }
}
