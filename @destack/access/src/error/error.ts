import type { ReportableError, ServiceErrorCode, ServiceErrorReport } from "@destack/error";
import type { StepUp } from "../context/elevation.ts";

/** The service error code of each access failure. */
const SERVICE_CODES = {
    INVALID_DECLARATION: "INTERNAL_SERVER_ERROR",
    INVALID_CONTEXT: "FORBIDDEN",
    NOT_FOUND: "NOT_FOUND",
    FORBIDDEN: "FORBIDDEN",
    INSUFFICIENT_AUTHENTICATION: "INSUFFICIENT_AUTHENTICATION",
    CONFLICT: "CONFLICT",
    STALE: "SERVICE_UNAVAILABLE",
} as const satisfies Readonly<Record<string, ServiceErrorCode>>;

/** A failure code of access decisions and declarations. */
export type AccessErrorCode = keyof typeof SERVICE_CODES;

/** Reject an invalid declaration, unavailable record, or unauthorized operation. */
export class AccessError extends Error implements ReportableError {
    /** The stable failure classification. */
    readonly code: AccessErrorCode;
    /** The authentication that would admit the caller, for insufficient authentication. */
    readonly stepUp?: StepUp;

    /** Retain the error classification and cause, and the authentication that would admit the caller. */
    constructor(
        code: AccessErrorCode,
        message: string,
        options?: ErrorOptions & { readonly stepUp?: StepUp },
    ) {
        // classify the error
        super(message, options);
        this.name = "AccessError";
        this.code = code;

        // keep the challenge a refusal names
        if (options?.stepUp !== undefined) {
            this.stepUp = options.stepUp;
        }
    }

    /** Convert the failure to the service error a caller receives, with the step-up it challenges for. */
    toServiceError(): ServiceErrorReport {
        const code = SERVICE_CODES[this.code];

        return this.stepUp === undefined
            ? { code, message: this.message }
            : { code, message: this.message, data: this.stepUp };
    }
}
