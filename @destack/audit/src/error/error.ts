import { ServiceError } from "@destack/service/error";

/** The service error code of each audit failure. */
const SERVICE_CODES = {
    INVALID_EVENT: "BAD_REQUEST",
    CONFLICT: "CONFLICT",
    FORBIDDEN: "FORBIDDEN",
    NOT_FOUND: "NOT_FOUND",
    UNAVAILABLE: "UNAVAILABLE",
} as const;

/** An audit failure. */
export class AuditError extends Error {
    /** The error classification. */
    readonly code: keyof typeof SERVICE_CODES;

    /** Create the error with its code and cause. */
    constructor(code: AuditError["code"], message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "AuditError";
        this.code = code;
    }

    /** Convert the failure to the service error a caller receives. */
    toServiceError() {
        return new ServiceError(SERVICE_CODES[this.code], { message: this.message, cause: this });
    }
}
