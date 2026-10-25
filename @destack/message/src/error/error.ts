import { defineErrorFamily, type ReasonOf } from "@destack/error";
import { schema } from "@destack/schema";
import type {} from "@destack/package/import-meta";

/** An SMTP server's negative reply to a command (RFC 5321 4.2). */
export const SmtpReply = schema.object({
    /** The reply code, such as 550. */
    code: schema.number().int(),
    /** The RFC 3463 enhanced status code, such as 5.1.1. */
    enhancedCode: schema.string().exactOptional(),
    /** The reply's text. */
    text: schema.string(),
});
/** An SMTP server's negative reply to a command. */
export type SmtpReply = schema.Infer<typeof SmtpReply>;

/** SES's refusal of a send: its AWS error code, absent when SES gave none. */
export const SesFailure = schema.object({
    /** The AWS error code, such as MessageRejected. */
    awsCode: schema.string().exactOptional(),
});
/** SES's refusal of a send. */
export type SesFailure = schema.Infer<typeof SesFailure>;

/** The message package's failures, each reason with the call error code it answers. */
export const MessageError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    ADDRESS_INVALID: { code: "BAD_REQUEST" },
    DATE_INVALID: { code: "BAD_REQUEST" },
    ENVELOPE_INVALID: { code: "BAD_REQUEST" },
    HEADER_INVALID: { code: "BAD_REQUEST" },
    HEADER_RESERVED: { code: "BAD_REQUEST" },
    KEY_INVALID: { code: "BAD_REQUEST" },
    REQUEST_INVALID: { code: "BAD_REQUEST", data: SesFailure },
    SEALED_CONTENT_MISSING: { code: "BAD_REQUEST" },

    // forbidden
    CLIENT_FOREIGN: { code: "FORBIDDEN" },

    // not found
    RECIPIENT_GONE: { code: "NOT_FOUND" },

    // conflict
    DESKTOP_MESSAGE_REQUIRED: { code: "CONFLICT" },
    RECIPIENT_DISABLED: { code: "CONFLICT" },
    SECRET_UNAVAILABLE: { code: "CONFLICT" },

    // unprocessable content
    COMMAND_DEFERRED: { code: "UNPROCESSABLE_CONTENT" },
    COMMAND_REJECTED: { code: "UNPROCESSABLE_CONTENT", data: SmtpReply },
    MESSAGE_REJECTED: { code: "UNPROCESSABLE_CONTENT", data: SesFailure },
    SENDER_UNVERIFIED: { code: "UNPROCESSABLE_CONTENT", data: SesFailure },

    // too many requests
    PROVIDER_THROTTLED: { code: "TOO_MANY_REQUESTS", data: SesFailure },

    // internal server error
    CREDENTIALS_REFUSED: { code: "INTERNAL_SERVER_ERROR", data: SesFailure },

    // bad gateway
    EXTENSION_UNSUPPORTED: { code: "BAD_GATEWAY" },
    RESPONSE_INVALID: { code: "BAD_GATEWAY" },
    PROVIDER_UNREACHABLE: { code: "BAD_GATEWAY" },

    // service unavailable
    CREDENTIALS_EXPIRED: { code: "SERVICE_UNAVAILABLE", data: SesFailure },
    PROVIDER_UNAVAILABLE: { code: "SERVICE_UNAVAILABLE", data: SesFailure },
    SENDING_SUSPENDED: { code: "SERVICE_UNAVAILABLE", data: SesFailure },

    // not implemented
    CHANNEL_UNSERVED: { code: "NOT_IMPLEMENTED" },

    // gateway timeout
    PROVIDER_TIMEOUT: { code: "GATEWAY_TIMEOUT" },
});

/** A stable cause of message failures. */
export type MessageReason = ReasonOf<typeof MessageError>;

/** A stable cause of message failures, as a refused send records it. */
export const MessageReason = MessageError.reason;
