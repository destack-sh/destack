import type { ReportableError, ServiceErrorCode, ServiceErrorReport } from "@destack/error";

/** The service error code of each audit failure. */
const SERVICE_CODES = {
    INVALID_EVENT: "BAD_REQUEST",
    CONFLICT: "CONFLICT",
    FORBIDDEN: "FORBIDDEN",
    NOT_FOUND: "NOT_FOUND",
    UNAVAILABLE: "SERVICE_UNAVAILABLE",
} as const satisfies Readonly<Record<string, ServiceErrorCode>>;

/** A failure code of audit history. */
export type AuditErrorCode = keyof typeof SERVICE_CODES;

/** An audit failure. */
export class AuditError extends Error implements ReportableError {
    /** The error classification. */
    readonly code: AuditErrorCode;

    /** Create the error with its code and cause. */
    constructor(code: AuditErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "AuditError";
        this.code = code;
    }

    /** Convert the failure to the service error a caller receives. */
    toServiceError(): ServiceErrorReport {
        return { code: SERVICE_CODES[this.code], message: this.message };
    }
}
