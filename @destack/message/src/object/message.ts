import { type Select } from "@destack/db";
import { ConditionSet, defineObject, field } from "@destack/object";
import * as identity from "@destack/identity";
import { schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { EmailContent, EmailDestination } from "../email/message.ts";
import { PushContent, PushDestination } from "../push/message.ts";
import { WebhookContent, WebhookDestination } from "../webhook/message.ts";

/** The code of a refusal for good from a recipient that forgot the endpoint (RFC 8030 7.3). */
export const GONE = "gone";

/** The label the message key pair derives under from an identity's root secret. */
const MESSAGE_LABEL = "@destack/message/key";

/** The states of a message: sending, sent, shown on its client, or failed. */
export const MESSAGE_STATES = ["pending", "sent", "delivered", "failed"] as const;

/** The desktop client a message shows on. */
export const DesktopDestination = schema.object({
    channel: schema.literal("desktop"),
    /** The signed-in native client. */
    client: schema.identifier("client"),
});

/** The schema of a destination. */
const DestinationSchema = schema.discriminatedUnion("channel", [
    EmailDestination,
    PushDestination,
    WebhookDestination,
    DesktopDestination,
]);

/** Where a message goes: an email address, a Web Push endpoint, a webhook or a desktop client. */
export const Destination = Object.assign(DestinationSchema, {
    /** Key a destination by its channel and its address, endpoint or client. */
    key(to: Destination): string {
        // key a desktop by its client, an email by its address, a browser by its endpoint
        if (to.channel === "desktop") {
            return `desktop:${to.client}`;
        } else if (to.channel === "email") {
            return `email:${to.address}`;
        } else if (to.channel === "push") {
            return `push:${to.url}`;
        }
        // key a webhook by its channel alone
        else {
            return "webhook";
        }
    },

    /** Report whether two destinations reach the same address, endpoint or client. */
    same(left: Destination, right: Destination): boolean {
        return Destination.key(left) === Destination.key(right);
    },
});
/** Where a message goes. */
export type Destination = schema.Infer<typeof DestinationSchema>;

/** What a message says: an email's subject and bodies, a push payload, or a webhook event. */
export const Content = schema.discriminatedUnion("channel", [
    EmailContent,
    PushContent,
    WebhookContent,
]);
/** What a message says on its channel. */
export type Content = schema.Infer<typeof Content>;

/** A home's message key, whose public half producers seal message content to. */
export class MessageKey {
    /** The key pair, its private half kept only on the home's machine. */
    readonly #recipient: identity.Recipient;

    /** Hold a home's message key. */
    private constructor(recipient: identity.Recipient) {
        this.#recipient = recipient;
    }

    /** Derive a home's message key from its identity's root secret, stable until it changes. */
    static async derive(root: Pick<identity.Deriver, "derivePrivateKey">): Promise<MessageKey> {
        return new MessageKey(await identity.Recipient.derive(root, MESSAGE_LABEL));
    }

    /** Identify a home's message key by its public half, as a producer sealing to it does. */
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

        return this.#recipient.seal(plaintext, MessageKey.#context(scope));
    }

    /** Open the content of a message of a scope, on the home's machine with the private half. */
    async open(sealed: identity.Ciphertext, scope: string): Promise<Content> {
        const plaintext = await this.#recipient.open(sealed, MessageKey.#context(scope));

        return Content.parse(JSON.parse(new TextDecoder().decode(plaintext)));
    }

    /** Bind sealed content to the scope of its message, so no other scope's message opens it. */
    static #context(scope: string): Uint8Array<ArrayBuffer> {
        return new TextEncoder().encode(`${MESSAGE_LABEL} ${scope}`);
    }
}

/** Why a provider refused a message. */
export const MessageError = schema.object({
    /** The provider's code, such as a mail service's error type or an HTTP status. */
    code: schema.string().min(1),
    /** The provider's message. */
    message: schema.string(),
});
/** Why a provider refused a message. */
export type MessageError = schema.Infer<typeof MessageError>;

/** One message on one channel to one recipient, which the channel's provider sends at once. */
export const message = defineObject(
    {
        name: "message",
        plural: "messages",
        controlled: true,
        fields: {
            /** Where it goes. */
            to: field.json(Destination),
            /** The object it is sent for, a notification or an endpoint. */
            source: field.object(),

            // sending
            /** Whether it is sending, went out, reached its client, or failed. */
            status: field.enum(MESSAGE_STATES).default("pending"),
            /** Its provider's error code once it failed. */
            reason: field.string().optional(),
            /** The content its producer sealed to the home's message key, absent for a desktop. */
            ciphertext: field.string(identity.Ciphertext).sensitive().optional(),
            /** When it settled, absent while it sends. */
            settledAt: field.time().optional(),
            /** When the next attempt is due after a temporary refusal, absent once settled. */
            retryAt: field.time().optional(),
        },
        permissions: ["read", "send"],
        // expire a settled message 7 days after it settles
        expiring: [
            {
                after: { days: 7 },
                from: "settledAt",
                where: { status: { in: ["sent", "delivered", "failed"] } },
            },
        ],
        constraints: {
            source: {
                kind: "index",
                on: ["sourcePackageId", "sourceType", "sourceScope", "sourceId", "createdAt"],
            },
            pending: {
                kind: "check",
                condition: {
                    OR: [
                        { status: { ne: "pending" } },
                        { ciphertext: { isNotNull: true }, settledAt: { isNull: true } },
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
            reason: {
                kind: "check",
                condition: {
                    OR: [
                        { status: "failed", reason: { isNotNull: true } },
                        { status: { ne: "failed" }, reason: { isNull: true } },
                    ],
                },
            },
        },
    },
    (method) => ({
        /** Send a message through its channel's provider, or show a desktop one at once. */
        create: method.create("send", {
            fields: [
                "to",
                "sourcePackageId",
                "sourceType",
                "sourceScope",
                "sourceId",
                "ciphertext",
            ],
            isPredicted: false,
        }),
        /** Read the VAPID key a scope's pushes are signed with, which browsers subscribe with. */
        serverKey: method.query({
            permission: "read",
            target: "scope",
            output: schema.object({
                /** The key, an uncompressed P-256 point in base64url (RFC 8292 3.2). */
                key: schema.string(),
            }),
        }),
        /** Read the public key producers seal a scope's message content to. */
        contentKey: method.query({
            permission: "read",
            target: "scope",
            output: schema.object({
                /** The key, the public half of the message installation's content key. */
                key: schema.string(),
            }),
        }),
        /** Record a desktop client showing a sent message, as its receipt. */
        receive: method.mutation({ permission: "read" }),
    }),
).implement({
    methods: {
        receive: async (call) => {
            // answer a message already delivered as it is
            const { target } = call;
            if (target.status === "delivered") {
                return target;
            }
            // refuse a message no desktop shows
            else if (target.to.channel !== "desktop" || target.status !== "sent") {
                throw new CallError("CONFLICT", {
                    message: `message ${target.id} is not a sent desktop message`,
                });
            }

            return call.updateStatus({ status: "delivered" });
        },
    },
});
/** A message as its table stores it. */
export type Message = Select<typeof message.table>;

/** The conditions the message controller reports: Sent, once settled or while retrying. */
export const messageConditions = new ConditionSet(["Sent"]);
