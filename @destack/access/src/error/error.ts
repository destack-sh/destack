/** Reject an invalid declaration, unavailable record, or unauthorized operation. */
export class AccessError extends Error {
    /** The stable failure classification. */
    readonly code:
        | "INVALID_DECLARATION"
        | "INVALID_CONTEXT"
        | "NOT_FOUND"
        | "FORBIDDEN"
        | "CONFLICT"
        | "STALE";

    /** Retain the error classification and cause. */
    constructor(code: AccessError["code"], message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "AccessError";
        this.code = code;
    }
}
