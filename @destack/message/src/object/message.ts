import { check, index, sql, type Select } from "@destack/db";
import { Conditions, defineObject, field } from "@destack/object";
import * as identity from "@destack/identity";
import { defineSchema, schema } from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import { EmailContent, EmailDestination } from "../email/message.ts";
import { PushContent, PushDestination } from "../push/message.ts";
import { WebhookContent, WebhookDestination } from "../webhook/message.ts";

/** The label the message key pair derives under from an identity's root secret. */
const MESSAGE_LABEL = "@destack/message/key";

/** The channels a message goes out on. */
export const CHANNELS = ["email", "push", "webhook"] as const;

/** A channel a message goes out on. */
export type Channel = (typeof CHANNELS)[number];

/** Where a message goes: an email address, a browser's Web Push endpoint, or an HTTPS webhook endpoint. */
export const Destination = defineSchema(
    schema.discriminatedUnion("channel", [EmailDestination, PushDestination, WebhookDestination]),
);
/** Where a message goes. */
export type Destination = schema.Infer<typeof Destination>;

/** What a message says on its channel: an email's subject and bodies, a push's payload, or a webhook's event. */
export const Content = defineSchema(
    schema.discriminatedUnion("channel", [EmailContent, PushContent, WebhookContent]),
);
/** What a message says on its channel. */
export type Content = schema.Infer<typeof Content>;

/**
 * A home's message key: producers seal a message's content to its public half, and the home's machine opens it to send.
 *
 * Sealing binds the message's scope, so no other scope's message opens the content.
 */
export class MessageKey {
    /** The key pair, its private half held only by the home's machine. */
    readonly #recipient: identity.Recipient;

    /** Hold a home's message key. */
    private constructor(recipient: identity.Recipient) {
        this.#recipient = recipient;
    }

    /** Derive a home's message key from its identity's root secret, the same until the root changes. */
    static async derive(root: Pick<identity.Deriver, "derivePrivateKey">): Promise<MessageKey> {
        return new MessageKey(await identity.Recipient.derive(root, MESSAGE_LABEL));
    }

    /** Name a home's message key by its public half, as a producer sealing to it does. */
    static of(key: string): MessageKey {
        return new MessageKey(identity.Recipient.of(key));
    }

    /** The public half producers seal to. */
    get key(): string {
        return this.#recipient.key;
    }

    /** Seal a message's content to the key, as a compact JWE bound to the message's scope. */
    seal(content: Content, scope: string): Promise<identity.Ciphertext> {
        const plaintext = new TextEncoder().encode(JSON.stringify(Content.parse(content)));

        return this.#recipient.seal(plaintext, context(scope));
    }

    /** Open the content of a message of a scope, on the home's machine holding the private half. */
    async open(sealed: identity.Ciphertext, scope: string): Promise<Content> {
        const plaintext = await this.#recipient.open(sealed, context(scope));

        return Content.parse(JSON.parse(new TextDecoder().decode(plaintext)));
    }
}

/** Why a provider refused a message. */
export const MessageError = defineSchema(
    schema.object({
        /** The provider's code, such as a mail service's error type or an HTTP status. */
        code: schema.string().min(1),
        /** The provider's message. */
        message: schema.string(),
    }),
);
/** Why a provider refused a message. */
export type MessageError = schema.Infer<typeof MessageError>;

/** One message on a channel to one recipient, which the channel's provider sends. */
export const message = defineObject({
    name: "message",
    plural: "messages",
    controlled: true,
    fields: {
        /** Where it goes. */
        to: field.json(Destination),
        /** What it says, sealed by its producer to its home's message key, and erased once its retention after settling passes. */
        ciphertext: field.string(identity.Ciphertext).sensitive().optional(),
        /** The object the message delivers for: an endpoint, or a notification's delivery. */
        source: field.json(ObjectReference),

        // the controller's sending
        /** Whether its provider has yet to send it, sent it, or refused it for good. */
        status: field.enum(["pending", "sent", "failed"]).default("pending"),
        /** When its provider sent it or refused it for good, absent while pending. */
        settledAt: field.time().optional(),
        /** When the next attempt is due after a temporary refusal, absent once settled. */
        retryAt: field.time().optional(),
        /** When its sealed content was erased, which the change log shows where it leaves the content out. */
        erasedAt: field.time().optional(),
    },
    constraints: (entry) => [
        index("message_source").on(entry.source, entry.createdAt),
        check(
            "message_pending",
            sql`${entry.status} <> 'pending' OR (${entry.settledAt} IS NULL AND ${entry.ciphertext} IS NOT NULL)`,
        ),
        check(
            "message_settled",
            sql`${entry.status} = 'pending' OR (${entry.settledAt} IS NOT NULL AND ${entry.retryAt} IS NULL)`,
        ),
        check(
            "message_erased",
            sql`(${entry.ciphertext} IS NULL) = (${entry.erasedAt} IS NOT NULL)`,
        ),
    ],
    permissions: ["read", "send", "erase"],
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        /** Send a pending message with its sealed content, whose identifier the requester may choose as its idempotency key. */
        create: method.create("send", {
            fields: ["to", "source"],
            variants: { status: { pending: ["ciphertext"] } },
            isPredicted: false,
        }),
        /** Erase a settled message's sealed content once its retention passes, as the message controller does. */
        erase: method
            .mutation({ permission: "erase", isInternal: true })
            .handle((call) => call.updateStatus({ ciphertext: null, erasedAt: call.now })),
    }),
});
/** A message as its table stores it. */
export type Message = Select<typeof message.table>;

/** The conditions the message controller reports: Sent, once its provider settles a message or while it retries. */
export const messageConditions = new Conditions(["Sent"]);

/** Bind sealed content to the scope of its message, so no other scope's message opens it. */
function context(scope: string): Uint8Array<ArrayBuffer> {
    return new TextEncoder().encode(`${MESSAGE_LABEL} ${scope}`);
}
