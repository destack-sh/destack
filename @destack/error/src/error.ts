import type { JsonValue } from "@destack/schema";

/** The HTTP status of each call error code, from RFC 9110, 6585 and 8470 or from Destack. */
export const CALL_ERROR_STATUSES = {
    BAD_REQUEST: 400,
    UNAUTHORIZED: 401,
    INSUFFICIENT_AUTHENTICATION: 401,
    QUOTA_EXCEEDED: 402,
    FORBIDDEN: 403,
    INSUFFICIENT_GRANT: 403,
    APPROVAL_REQUIRED: 403,
    NOT_FOUND: 404,
    METHOD_NOT_SUPPORTED: 405,
    NOT_ACCEPTABLE: 406,
    TIMEOUT: 408,
    CONFLICT: 409,
    MANAGED: 409,
    GONE: 410,
    STALE_EPOCH: 410,
    PRECONDITION_FAILED: 412,
    PAYLOAD_TOO_LARGE: 413,
    UNSUPPORTED_MEDIA_TYPE: 415,
    MISDIRECTED_REQUEST: 421,
    UNPROCESSABLE_CONTENT: 422,
    TOO_MANY_REQUESTS: 429,
    CLIENT_CLOSED_REQUEST: 499,
    INTERNAL_SERVER_ERROR: 500,
    NOT_IMPLEMENTED: 501,
    BAD_GATEWAY: 502,
    SERVICE_UNAVAILABLE: 503,
    GATEWAY_TIMEOUT: 504,
} as const;

/** A call error code. */
export type CallErrorCode = keyof typeof CALL_ERROR_STATUSES;

/** The call error a caller receives for a failure, as plain values. */
export interface CallErrorReport {
    /** The call error code. */
    readonly code: CallErrorCode;
    /** The readable message. */
    readonly message: string;
    /** The structured details. */
    readonly data?: JsonValue;
}

/** A package's failure that knows the call error its caller receives. */
export interface DomainError extends Error {
    /** Convert the failure to the call error a caller receives. */
    toCallError(): CallErrorReport;
}

/** Recognise a package's failures that know the call error their caller receives. */
export const DomainError = {
    /** Report whether a value is an error with a call error. */
    is(value: unknown): value is DomainError {
        return (
            value instanceof Error &&
            "toCallError" in value &&
            typeof value.toCallError === "function"
        );
    },
};
