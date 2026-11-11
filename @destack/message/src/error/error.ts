import { defineErrorFamily, type ReasonOf } from "@destack/error";
import { MailReason } from "@destack/mail/error";
import { schema } from "@destack/schema";
import type {} from "@destack/package/import-meta";

/** The message package's failures, each reason with the call error code it answers. */
export const MessageError = defineErrorFamily({
    reasons: {
        // not found
        RECIPIENT_GONE_NOT_FOUND: {},

        // invalid
        HEADER_INVALID: {},
        MESSAGE_INVALID: {},

        // required
        CONTENT_REQUIRED: {},

        // denied
        CLIENT_DENIED: {},

        // blocked
        ENDPOINT_BLOCKED: {},
        MESSAGE_BLOCKED: {},

        // exhausted
        PROVIDER_EXHAUSTED: {},

        // unsupported
        CHANNEL_UNSUPPORTED: {},

        // unavailable
        PROVIDER_UNAVAILABLE: {},

        // unreachable
        PROVIDER_UNREACHABLE: {},
    },
});

/** A stable cause of message failures. */
export type MessageReason = ReasonOf<typeof MessageError>;

/** A stable cause of message failures, as a refused send records it. */
export const MessageReason = MessageError.reason;

/** A stable cause of a refused send: a message failure's, or its mail transport's. */
export const RefusalReason = schema.union([MessageReason, MailReason]);
/** A stable cause of a refused send. */
export type RefusalReason = schema.Infer<typeof RefusalReason>;
