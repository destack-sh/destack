import { check, index, sql, type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { defineSchema, schema } from "@destack/schema";
import { space } from "@destack/space/object";
import { EmailContent, EmailRecipient } from "../email/message.ts";
import { PushContent, PushRecipient } from "../push/message.ts";
import { WebhookContent, WebhookRecipient } from "../webhook/message.ts";

/** The channels a message goes out on. */
export const CHANNELS = ["email", "push", "webhook"] as const;

/** A channel a message goes out on. */
export type Channel = (typeof CHANNELS)[number];

/** Where a message goes: an email address, a browser's Web Push endpoint, or an HTTPS webhook endpoint. */
export const Recipient = defineSchema(
    schema.discriminatedUnion("channel", [EmailRecipient, PushRecipient, WebhookRecipient]),
);
/** Where a message goes. */
export type Recipient = schema.Infer<typeof Recipient>;

/** What a message says on its channel: an email's subject and bodies, a push's payload, or a webhook's event. */
export const Content = defineSchema(
    schema.discriminatedUnion("channel", [EmailContent, PushContent, WebhookContent]),
);
/** What a message says on its channel. */
export type Content = schema.Infer<typeof Content>;

/** Why a provider refused a message. */
export const MessageError = defineSchema(
    schema.object({
        /** The provider's code, such as an SES error type or an HTTP status. */
        code: schema.string().min(1),
        /** The provider's message. */
        message: schema.string(),
    }),
);
/** Why a provider refused a message. */
export type MessageError = schema.Infer<typeof MessageError>;

/** One message on a channel to one recipient, which the channel's provider sends, after Knock's and Novu's messages. */
export const message = defineObject({
    name: "message",
    plural: "messages",
    scope: space,
    controlled: true,
    fields: {
        /** Where it goes. */
        to: field.json(Recipient),
        /** What it says. */
        content: field.json(Content),
        /** The secret signing a webhook message, after Standard Webhooks, absent on other channels. */
        secret: field.string().sensitive().optional(),

        // the controller's sending
        /** When the provider accepted it. */
        sentAt: field.time().optional(),
        /** When the provider refused it for good. */
        failedAt: field.time().optional(),
        /** When the next attempt is due after a refusal for now. */
        retryAt: field.time().optional(),
    },
    constraints: (entry) => [
        index("message_due").on(entry.sentAt, entry.failedAt),
        check("message_settled", sql`${entry.sentAt} IS NULL OR ${entry.failedAt} IS NULL`),
    ],
    permissions: ["read", "send"],
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        /** Send a message, whose identifier the requester may choose as its idempotency key. */
        create: method.create("send", { fields: ["to", "content", "secret"] }),
    }),
});
/** A message as its table stores it. */
export type Message = Select<typeof message.table>;
