import type { Reply } from "./reply.ts";

/** Failures submitting mail over SMTP. */
export type SmtpErrorCode =
    | "INVALID_OPTIONS"
    | "INSECURE"
    | "INVALID_ENVELOPE"
    | "CONNECTION"
    | "TIMEOUT"
    | "PROTOCOL"
    | "UNSUPPORTED"
    | "REJECTED";

/** An SMTP failure with a stable code. */
export class SmtpError extends Error {
    /** The failure code. */
    readonly code: SmtpErrorCode;
    /** The server's reply that rejected the session, present for REJECTED only. */
    readonly reply: Reply | undefined;

    /** Create an SMTP failure. */
    constructor(code: SmtpErrorCode, message: string, reply?: Reply, options?: ErrorOptions) {
        // keep the code and the rejecting reply
        super(message, options);
        this.name = "SmtpError";
        this.code = code;
        this.reply = reply;
    }
}
