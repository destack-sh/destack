import type { ReportableError, ServiceErrorCode, ServiceErrorReport } from "@destack/schema";
import type { Reply } from "./reply.ts";

/** The service error code of each SMTP failure: a bad envelope, a refused message, the relay's, or the host's. */
const SERVICE_CODES = {
    INVALID_OPTIONS: "INTERNAL_SERVER_ERROR",
    INSECURE: "INTERNAL_SERVER_ERROR",
    INVALID_ENVELOPE: "BAD_REQUEST",
    CONNECTION: "BAD_GATEWAY",
    TIMEOUT: "GATEWAY_TIMEOUT",
    PROTOCOL: "BAD_GATEWAY",
    UNSUPPORTED: "BAD_GATEWAY",
    REJECTED: "UNPROCESSABLE_CONTENT",
} as const satisfies Readonly<Record<string, ServiceErrorCode>>;

/** Failures submitting mail over SMTP. */
export type SmtpErrorCode = keyof typeof SERVICE_CODES;

/** An SMTP failure with a stable code. */
export class SmtpError extends Error implements ReportableError {
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

    /** Convert the failure to the service error a caller receives. */
    toServiceError(): ServiceErrorReport {
        return { code: SERVICE_CODES[this.code], message: this.message };
    }
}
