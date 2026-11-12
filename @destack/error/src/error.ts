import type { ErrorAudience } from "./family.ts";
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

/** The conditions a reason ends with, after gRPC's canonical codes, with their call error codes. */
export const ERROR_CONDITIONS = {
    /** Not found, gRPC NOT_FOUND: `<noun> '<value>' not found`. */
    NOT_FOUND: "NOT_FOUND",
    /** Exists, gRPC ALREADY_EXISTS: `<noun> '<value>' already exists`. */
    EXISTS: "CONFLICT",
    /** Conflict, gRPC ABORTED: `cannot <verb> <noun> '<value>': <why>`. */
    CONFLICT: "CONFLICT",
    /** Invalid, gRPC INVALID_ARGUMENT: `<noun> '<value>' is invalid: <what> must …`. */
    INVALID: "BAD_REQUEST",
    /** Required, gRPC INVALID_ARGUMENT: `<what> is required`. */
    REQUIRED: "BAD_REQUEST",
    /** Unauthenticated, gRPC UNAUTHENTICATED: `authentication required: <why>`. */
    UNAUTHENTICATED: "UNAUTHORIZED",
    /** Denied, gRPC PERMISSION_DENIED: `<noun> '<value>' is forbidden: <who> cannot <verb>`. */
    DENIED: "FORBIDDEN",
    /** Blocked, gRPC FAILED_PRECONDITION: `cannot <verb> <noun> '<value>': <why>`. */
    BLOCKED: "PRECONDITION_FAILED",
    /** Exhausted, gRPC RESOURCE_EXHAUSTED: `<noun> '<value>' exceeds <limit>`. */
    EXHAUSTED: "TOO_MANY_REQUESTS",
    /** Unsupported, gRPC UNIMPLEMENTED: `<thing> is not supported`. */
    UNSUPPORTED: "NOT_IMPLEMENTED",
    /** Moved, HTTP 421: `<noun> '<value>' moved to '<where>'`, which a retry there reaches. */
    MOVED: "MISDIRECTED_REQUEST",
    /** Unavailable, gRPC UNAVAILABLE: `<noun> '<value>' is unavailable: <why>`, with a retry. */
    UNAVAILABLE: "SERVICE_UNAVAILABLE",
    /** Unreachable, gRPC DEADLINE_EXCEEDED or upstream: `could not <verb> <noun> '<value>'`. */
    UNREACHABLE: "BAD_GATEWAY",
    /** Cancelled, gRPC CANCELLED: `<what> was cancelled`. */
    CANCELLED: "CLIENT_CLOSED_REQUEST",
    /** Broken, gRPC INTERNAL: `<noun> '<value>' <fact that must never hold>`. */
    BROKEN: "INTERNAL_SERVER_ERROR",
} as const satisfies Readonly<Record<string, CallErrorCode>>;

/** A condition a reason ends with. */
export type ErrorCondition = keyof typeof ERROR_CONDITIONS;

/** A reason: what failed, then its condition, such as INSTALLATION_NOT_FOUND. */
export type Reason = `${string}_${ErrorCondition}`;

/** The conditions, the longest first, so a reason's suffix matches its longest condition. */
const CONDITIONS = Object.keys(ERROR_CONDITIONS)
    .filter((name): name is ErrorCondition => Object.hasOwn(ERROR_CONDITIONS, name))
    .toSorted((left, right) => right.length - left.length);

/** Read conditions from reasons. */
export const ErrorCondition = {
    /** Check whether a name is an error condition. */
    is(name: string): name is ErrorCondition {
        return Object.hasOwn(ERROR_CONDITIONS, name);
    },

    /** Read the condition a reason ends with, refusing one that ends with none. */
    of(reason: string): ErrorCondition {
        const found = ErrorCondition.find(reason);
        if (found === undefined) {
            throw new TypeError(`reason '${reason}' must end with an error condition`);
        }

        return found;
    },

    /** Find the condition a reason ends with, the longest when several match. */
    find(reason: string): ErrorCondition | undefined {
        return CONDITIONS.find((condition) => reason.endsWith(`_${condition}`));
    },
};

/** A package's failure that knows the call error its caller receives. */
export interface DomainError extends Error {
    /** Whether the failure is for callers or for the host alone. */
    readonly audience: ErrorAudience;
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
            typeof value.toCallError === "function" &&
            "audience" in value &&
            (value.audience === "caller" || value.audience === "host")
        );
    },
};
