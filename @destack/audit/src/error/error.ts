/** An audit failure. */
export class AuditError extends Error {
    /** The error classification. */
    readonly code: "INVALID_EVENT" | "CONFLICT" | "FORBIDDEN" | "NOT_FOUND" | "UNAVAILABLE";

    /** Create the error with its code and cause. */
    constructor(code: AuditError["code"], message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "AuditError";
        this.code = code;
    }
}
