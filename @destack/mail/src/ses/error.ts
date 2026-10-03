/** Failures sending mail through SES. */
export type SesErrorCode =
    | "INVALID_OPTIONS"
    | "CONNECTION"
    | "THROTTLED"
    | "UNAVAILABLE"
    | "EXPIRED"
    | "UNAUTHORIZED"
    | "REJECTED"
    | "UNVERIFIED"
    | "SUSPENDED"
    | "INVALID_REQUEST"
    | "INVALID_RESPONSE";

/** The failures a later send of the same message may get past. */
const RETRYABLE: ReadonlySet<SesErrorCode> = new Set([
    "CONNECTION",
    "THROTTLED",
    "UNAVAILABLE",
    "EXPIRED",
]);

/** A failure sending mail through SES, with a stable code and the AWS error code. */
export class SesError extends Error {
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
}
