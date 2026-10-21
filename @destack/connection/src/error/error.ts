import type { DomainError, CallErrorCode, Failure } from "@destack/error";

/** The call error code of each connection failure. */
const CALL_CODES = {
    REFUSED: "BAD_GATEWAY",
    INVALID_RESPONSE: "BAD_GATEWAY",
    UNREACHABLE: "BAD_GATEWAY",
    INVALID_AUTHORIZATION: "BAD_REQUEST",
    UNAUTHORIZED: "PRECONDITION_FAILED",
    UNSUPPORTED: "PRECONDITION_FAILED",
    UNAVAILABLE: "SERVICE_UNAVAILABLE",
} as const satisfies Readonly<Record<string, CallErrorCode>>;

/** A failure code of connections. */
export type ConnectionErrorCode = keyof typeof CALL_CODES;

/** A failure of a connection or its authorization server, which never includes a credential. */
export class ConnectionError extends Error implements DomainError {
    /** The stable failure code. */
    readonly code: ConnectionErrorCode;
    /** The authorization server's OAuth error code (RFC 6749 5.2), absent for other failures. */
    readonly error: string | undefined;

    /** Report a failure and its cause, with the server's OAuth error code for a refusal. */
    constructor(
        code: ConnectionErrorCode,
        message: string,
        options: { readonly error?: string; readonly cause?: unknown } = {},
    ) {
        // take the code, the authorization server's error and the cause
        super(message, "cause" in options ? { cause: options.cause } : {});
        this.name = "ConnectionError";
        this.code = code;
        this.error = options.error;
    }

    /** Convert the failure to the call error a caller receives. */
    toCallError(): Failure {
        return { code: CALL_CODES[this.code], message: this.message };
    }
}
