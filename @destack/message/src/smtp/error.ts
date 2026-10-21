import type { DomainError, CallErrorCode, Failure } from "@destack/error";
import type { Reply } from "./reply.ts";

/** The call error code per SMTP failure: an envelope, a message, the relay or the machine. */
const CALL_CODES = {
    INVALID_OPTIONS: "INTERNAL_SERVER_ERROR",
    INSECURE: "INTERNAL_SERVER_ERROR",
    INVALID_ENVELOPE: "BAD_REQUEST",
    CONNECTION: "BAD_GATEWAY",
    TIMEOUT: "GATEWAY_TIMEOUT",
    PROTOCOL: "BAD_GATEWAY",
    UNSUPPORTED: "BAD_GATEWAY",
    REJECTED: "UNPROCESSABLE_CONTENT",
} as const satisfies Readonly<Record<string, CallErrorCode>>;

/** Failures submitting mail over SMTP. */
export type SmtpErrorCode = keyof typeof CALL_CODES;

/** An SMTP failure with a stable code. */
export class SmtpError extends Error implements DomainError {
    /** The failure code. */
    readonly code: SmtpErrorCode;
    /** The server's reply that rejected the session, present for REJECTED only. */
    readonly reply: Reply | undefined;

    /** Create an SMTP failure. */
    constructor(code: SmtpErrorCode, message: string, reply?: Reply, options?: ErrorOptions) {
        // store the code and the rejecting reply
        super(message, options);
        this.name = "SmtpError";
        this.code = code;
        this.reply = reply;
    }

    /** Convert the failure to the call error a caller receives. */
    toCallError(): Failure {
        return { code: CALL_CODES[this.code], message: this.message };
    }
}
