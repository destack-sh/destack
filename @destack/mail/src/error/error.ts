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

/** The mail package's failures, each reason with the call error code it answers. */
export const MailError = defineErrorFamily({
    reasons: {
        // invalid
        ADDRESS_INVALID: {},
        COMMAND_INVALID: { data: SmtpReply },
        DATE_INVALID: {},
        ENVELOPE_INVALID: {},
        HEADER_INVALID: {},
        KEY_INVALID: {},
        MESSAGE_INVALID: { data: SesFailure },
        REQUEST_INVALID: { data: SesFailure },
        SENDER_INVALID: { data: SesFailure },

        // exhausted
        PROVIDER_EXHAUSTED: { data: SesFailure },

        // unavailable
        COMMAND_UNAVAILABLE: { data: SmtpReply },
        CREDENTIALS_UNAVAILABLE: { data: SesFailure },
        PROVIDER_UNAVAILABLE: { data: SesFailure },
        SENDING_UNAVAILABLE: { data: SesFailure },

        // unreachable
        EXTENSION_UNREACHABLE: {},
        PROVIDER_UNREACHABLE: {},
        RESPONSE_UNREACHABLE: {},

        // denied
        CREDENTIALS_DENIED: { data: SesFailure },
    },
});

/** A stable cause of mail failures. */
export type MailReason = ReasonOf<typeof MailError>;

/** A stable cause of mail failures, as a refused send records it. */
export const MailReason = MailError.reason;
