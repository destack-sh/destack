import type { JsonValue } from "@destack/schema";

/** The HTTP status of each service error code (RFC 9110, 6585 and 8470), beside Destack's own codes. */
export const SERVICE_ERROR_STATUSES = {
    BAD_REQUEST: 400,
    UNAUTHORIZED: 401,
    INSUFFICIENT_AUTHENTICATION: 401,
    QUOTA_EXCEEDED: 402,
    FORBIDDEN: 403,
    INSUFFICIENT_GRANT: 403,
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

/** A service error code. */
export type ServiceErrorCode = keyof typeof SERVICE_ERROR_STATUSES;

/** The service error a caller receives for a failure, as plain values. */
export interface ServiceErrorReport {
    /** The service error code. */
    readonly code: ServiceErrorCode;
    /** The readable message. */
    readonly message: string;
    /** The structured details. */
    readonly data?: JsonValue;
}

/** A package's failure that knows the service error its caller receives. */
export interface DomainError extends Error {
    /** Convert the failure to the service error a caller receives. */
    toServiceError(): ServiceErrorReport;
}

/** Recognise a package's failures that know the service error their caller receives. */
export const DomainError = {
    /** Report whether a value is an error with a service error. */
    is(value: unknown): value is DomainError {
        return (
            value instanceof Error &&
            "toServiceError" in value &&
            typeof value.toServiceError === "function"
        );
    },
};
