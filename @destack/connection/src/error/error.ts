import type { DomainError, ServiceErrorCode, ServiceErrorReport } from "@destack/error";

/** The service error code of each connection failure. */
const SERVICE_CODES = {
    REFUSED: "BAD_GATEWAY",
    INVALID_RESPONSE: "BAD_GATEWAY",
    UNREACHABLE: "BAD_GATEWAY",
    INVALID_AUTHORIZATION: "BAD_REQUEST",
    UNAUTHORIZED: "PRECONDITION_FAILED",
    UNSUPPORTED: "PRECONDITION_FAILED",
} as const satisfies Readonly<Record<string, ServiceErrorCode>>;

/** A failure code of connections: an authorization server's refusal or answer, an authorization's mismatch, or a connection unfit to lend. */
export type ConnectionErrorCode = keyof typeof SERVICE_CODES;

/** A failure of a connection or its authorization server, which never holds a credential. */
export class ConnectionError extends Error implements DomainError {
    /** The stable failure code. */
    readonly code: ConnectionErrorCode;
    /** The OAuth error code the authorization server answered (RFC 6749 5.2), absent for other failures. */
    readonly error: string | undefined;

    /** Report a failure, with the authorization server's OAuth error code for a refusal and the failure causing it. */
    constructor(
        code: ConnectionErrorCode,
        message: string,
        options: { readonly error?: string; readonly cause?: unknown } = {},
    ) {
        // keep the code, the authorization server's error and the cause
        super(message, "cause" in options ? { cause: options.cause } : {});
        this.name = "ConnectionError";
        this.code = code;
        this.error = options.error;
    }

    /** Convert the failure to the service error a caller receives. */
    toServiceError(): ServiceErrorReport {
        return { code: SERVICE_CODES[this.code], message: this.message };
    }
}
