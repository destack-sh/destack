import type { Failure } from "./failure.ts";

/** The HTTP status of each call error code: oRPC's codes, and RFC 9110's names beyond them. */
export const CALL_ERROR_STATUSES = {
    BAD_REQUEST: 400,
    UNAUTHORIZED: 401,
    PAYMENT_REQUIRED: 402,
    FORBIDDEN: 403,
    NOT_FOUND: 404,
    METHOD_NOT_SUPPORTED: 405,
    NOT_ACCEPTABLE: 406,
    TIMEOUT: 408,
    CONFLICT: 409,
    GONE: 410,
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

/** Recognise call error codes and read them from statuses. */
export const CallErrorCode = {
    /** Report whether a code is a call error code. */
    is(code: string): code is CallErrorCode {
        return Object.hasOwn(CALL_ERROR_STATUSES, code);
    },

    /** Read a status's HTTP reason code, else its class's: a bad request or an internal error. */
    of(status: number): CallErrorCode {
        // take the status's code
        const code = Object.entries(CALL_ERROR_STATUSES).find((entry) => entry[1] === status)?.[0];
        if (code !== undefined && CallErrorCode.is(code)) {
            return code;
        }

        return status >= 400 && status < 500 ? "BAD_REQUEST" : "INTERNAL_SERVER_ERROR";
    },
};

/** A package's failure that knows the call error its caller receives. */
export interface DomainError extends Error {
    /** Convert the failure to the call error a caller receives. */
    readonly toCallError: () => Failure;
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
