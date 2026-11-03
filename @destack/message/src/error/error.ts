import { defineErrorFamily, type ReasonOf } from "@destack/error";
import { MailReason } from "@destack/mail/error";
import { schema } from "@destack/schema";
import type {} from "@destack/package/import-meta";

/** The message package's failures, each reason with the call error code it answers. */
export const MessageError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    HEADER_RESERVED: { code: "BAD_REQUEST" },
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
    MESSAGE_REJECTED: { code: "UNPROCESSABLE_CONTENT" },

    // too many requests
    PROVIDER_THROTTLED: { code: "TOO_MANY_REQUESTS" },

    // bad gateway
    PROVIDER_UNREACHABLE: { code: "BAD_GATEWAY" },

    // service unavailable
    PROVIDER_UNAVAILABLE: { code: "SERVICE_UNAVAILABLE" },

    // not implemented
    CHANNEL_UNSERVED: { code: "NOT_IMPLEMENTED" },

    // gateway timeout
    PROVIDER_TIMEOUT: { code: "GATEWAY_TIMEOUT" },
});

/** A stable cause of message failures. */
export type MessageReason = ReasonOf<typeof MessageError>;

/** A stable cause of message failures, as a refused send records it. */
export const MessageReason = MessageError.reason;

/** A stable cause of a refused send: a message failure's, or its mail transport's. */
export const RefusalReason = schema.union([MessageReason, MailReason]);
/** A stable cause of a refused send. */
export type RefusalReason = schema.Infer<typeof RefusalReason>;
