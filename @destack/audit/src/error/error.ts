/** Fail an audit operation without changing the reported application outcome. */
export class AuditError extends Error {
    /** Stable error classification. */
    readonly code: "INVALID_EVENT" | "CONFLICT" | "FORBIDDEN" | "NOT_FOUND" | "UNAVAILABLE";

    /** Retain the failure and its original cause. */
    constructor(code: AuditError["code"], message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "AuditError";
        this.code = code;
    }
}
