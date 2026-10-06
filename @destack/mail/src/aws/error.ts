import type { DomainError, ServiceErrorCode, ServiceErrorReport } from "@destack/error";

/** The service error code of each SES failure: a refused message, a throttle, SES's, or the host's. */
const SERVICE_CODES = {
    INVALID_OPTIONS: "INTERNAL_SERVER_ERROR",
    CONNECTION: "BAD_GATEWAY",
    THROTTLED: "TOO_MANY_REQUESTS",
    UNAVAILABLE: "SERVICE_UNAVAILABLE",
    EXPIRED: "SERVICE_UNAVAILABLE",
    UNAUTHORIZED: "INTERNAL_SERVER_ERROR",
    REJECTED: "UNPROCESSABLE_CONTENT",
    UNVERIFIED: "UNPROCESSABLE_CONTENT",
    SUSPENDED: "SERVICE_UNAVAILABLE",
    INVALID_REQUEST: "BAD_REQUEST",
    INVALID_RESPONSE: "BAD_GATEWAY",
} as const satisfies Readonly<Record<string, ServiceErrorCode>>;

/** Failures sending mail through SES. */
export type SesErrorCode = keyof typeof SERVICE_CODES;

/** The failures a later send of the same message may get past. */
const RETRYABLE: ReadonlySet<SesErrorCode> = new Set([
    "CONNECTION",
    "THROTTLED",
    "UNAVAILABLE",
    "EXPIRED",
]);

/** A failure sending mail through SES, with a stable code and the AWS error code. */
export class SesError extends Error implements DomainError {
    /** The failure code. */
    readonly code: SesErrorCode;
    /** The AWS error code, such as MessageRejected, absent for failures SES did not answer. */
    readonly awsCode: string | undefined;
    /** The HTTP status SES answered with, absent for failures SES did not answer. */
    readonly status: number | undefined;
    /** Whether sending the same message later may succeed. */
    readonly isRetryable: boolean;

    /** Create an SES failure. */
    constructor(
        code: SesErrorCode,
        message: string,
        answer?: { readonly awsCode: string | undefined; readonly status: number },
        options?: ErrorOptions,
    ) {
        // keep the code and SES's answer
        super(message, options);
        this.name = "SesError";
        this.code = code;
        this.awsCode = answer?.awsCode;
        this.status = answer?.status;
        this.isRetryable = RETRYABLE.has(code);
    }

    /** Convert the failure to the service error a caller receives. */
    toServiceError(): ServiceErrorReport {
        return { code: SERVICE_CODES[this.code], message: this.message };
    }
}
