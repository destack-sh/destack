import type { ReportableError, ServiceErrorCode, ServiceErrorReport } from "@destack/schema";

/** The service error code of each composition failure: every one is the caller's. */
const SERVICE_CODES = {
    INVALID_ADDRESS: "BAD_REQUEST",
    INVALID_HEADER: "BAD_REQUEST",
    INVALID_KEY: "BAD_REQUEST",
    INVALID_DATE: "BAD_REQUEST",
} as const satisfies Readonly<Record<string, ServiceErrorCode>>;

/** Failures composing a message. */
export type MimeErrorCode = keyof typeof SERVICE_CODES;

/** A message composition failure with a stable code. */
export class MimeError extends Error implements ReportableError {
    /** The failure code. */
    readonly code: MimeErrorCode;

    /** Create a message composition failure. */
    constructor(code: MimeErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "MimeError";
        this.code = code;
    }

    /** Convert the failure to the service error a caller receives. */
    toServiceError(): ServiceErrorReport {
        return { code: SERVICE_CODES[this.code], message: this.message };
    }
}
