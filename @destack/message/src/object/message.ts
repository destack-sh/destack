import { type Select } from "@destack/db";
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

/** Where a message goes: an email address, a Web Push endpoint, or an HTTPS webhook endpoint. */
export const Destination = defineSchema(
    schema.discriminatedUnion("channel", [EmailDestination, PushDestination, WebhookDestination]),
);
/** Where a message goes. */
export type Destination = schema.Infer<typeof Destination>;

/** What a message says: an email's subject and bodies, a push payload, or a webhook event. */
export const Content = defineSchema(
    schema.discriminatedUnion("channel", [EmailContent, PushContent, WebhookContent]),
);
/** What a message says on its channel. */
export type Content = schema.Infer<typeof Content>;

/** A home's message key, whose public half producers seal message content to. */
export class MessageKey {
    /** The key pair, its private half held only by the home's machine. */
    readonly #recipient: identity.Recipient;

    /** Hold a home's message key. */
    private constructor(recipient: identity.Recipient) {
        this.#recipient = recipient;
    }

    /** Derive a home's message key from its identity's root secret, stable until it changes. */
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
        /** The content sealed by its producer to the home's message key. */
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
        /** When its sealed content was erased, as the change log shows when it leaves it out. */
        erasedAt: field.time().optional(),
    },
    permissions: ["read", "send", "erase"],
    constraints: {
        source: { kind: "index", on: ["source", "createdAt"] },
        pending: {
            kind: "check",
            condition: {
                OR: [
                    { status: { ne: "pending" } },
                    { settledAt: { isNull: true }, ciphertext: { isNotNull: true } },
                ],
            },
        },
        settled: {
            kind: "check",
            condition: {
                OR: [
                    { status: "pending" },
                    { settledAt: { isNotNull: true }, retryAt: { isNull: true } },
                ],
            },
        },
        erased: {
            kind: "check",
            condition: {
                OR: [
                    { ciphertext: { isNull: true }, erasedAt: { isNotNull: true } },
                    { ciphertext: { isNotNull: true }, erasedAt: { isNull: true } },
                ],
            },
        },
    },
})
    .methods((method) => ({
        /** Send a pending message with its sealed content. */
        create: method.create("send", {
            fields: ["to", "source"],
            variants: { status: { pending: ["ciphertext"] } },
            isPredicted: false,
        }),
        /** Erase a settled message's sealed content after its retention, as the controller does. */
        erase: method.mutation({ permission: "erase", isInternal: true }),
        /** Read the VAPID key a scope's pushes are signed with, which browsers subscribe with. */
        serverKey: method.query({
            permission: "read",
            target: "none",
            output: schema.object({
                /** The key, an uncompressed P-256 point in base64url (RFC 8292 3.2). */
                key: schema.string(),
            }),
        }),
    }))
    .implement({
        methods: { erase: (call) => call.updateStatus({ ciphertext: null, erasedAt: call.now }) },
    });
/** A message as its table stores it. */
export type Message = Select<typeof message.table>;

/** The conditions the message controller reports: Sent, once settled or while retrying. */
export const messageConditions = new Conditions(["Sent"]);

/** Bind sealed content to the scope of its message, so no other scope's message opens it. */
function context(scope: string): Uint8Array<ArrayBuffer> {
    return new TextEncoder().encode(`${MESSAGE_LABEL} ${scope}`);
}
