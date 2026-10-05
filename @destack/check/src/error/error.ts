import type { ReportableError, ServiceErrorCode, ServiceErrorReport } from "@destack/schema";

/** The service error code of each check failure: a bad selection, an unsupported package, or the tool's. */
const SERVICE_CODES = {
    TOOL: "INTERNAL_SERVER_ERROR",
    CONFIGURATION: "BAD_REQUEST",
    LANGUAGE: "UNPROCESSABLE_CONTENT",
} as const satisfies Readonly<Record<string, ServiceErrorCode>>;

/** A failure category of checks. */
export type CheckErrorCode = keyof typeof SERVICE_CODES;

/** A failed checker invocation or unsupported package. */
export class CheckError extends Error implements ReportableError {
    /** Failure category. */
    readonly code: CheckErrorCode;

    /** Identify the failed operation. */
    constructor(code: CheckErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "CheckError";
        this.code = code;
    }

    /** Convert the failure to the service error a caller receives. */
    toServiceError(): ServiceErrorReport {
        return { code: SERVICE_CODES[this.code], message: this.message };
    }
}
