import { type Select } from "@destack/db";
import { ConditionSet, defineObject, field } from "@destack/object";
import * as identity from "@destack/identity";
import { defineSchema, Instant, schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { EmailContent, EmailDestination } from "../email/message.ts";
import { PushContent, PushDestination } from "../push/message.ts";
import { WebhookContent, WebhookDestination } from "../webhook/message.ts";

/** The code of a refusal for good from a recipient that forgot the endpoint (RFC 8030 7.3). */
export const GONE = "gone";

/** The label the message key pair derives under from an identity's root secret. */
const MESSAGE_LABEL = "@destack/message/key";

/** The states of a message: waiting for its time, sending, and settled. */
export const MESSAGE_STATES = [
    "scheduled",
    "skipped",
    "pending",
    "sent",
    "delivered",
    "failed",
] as const;

/** The desktop client a message shows on. */
export const DesktopDestination = schema.object({
    channel: schema.literal("desktop"),
    /** The signed-in native client. */
    client: schema.identifier("client"),
});

/** Where a message goes: an email address, a Web Push endpoint, a webhook or a desktop client. */
export const Destination = defineSchema(
    schema.discriminatedUnion("channel", [
        EmailDestination,
        PushDestination,
        WebhookDestination,
        DesktopDestination,
    ]),
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
    /** The key pair, its private half kept only on the home's machine. */
    readonly #recipient: identity.Recipient;

    /** Keep a home's message key. */
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

/** One message on one channel to one recipient, which the channel's provider sends when due. */
export const message = defineObject({
    name: "message",
    plural: "messages",
    controlled: true,
    fields: {
        /** Where it goes. */
        to: field.json(Destination),
        /** The object it is sent for, a notification or an endpoint. */
        source: field.object(),

        // schedule
        /** When it is due. */
        dueAt: field.time(),
        /** Whether it goes out in a scheduled summary. */
        isSummarized: field.boolean().default(false),

        // sending
        /** Whether it waits, was skipped, is sending, went out, reached its client, or failed. */
        status: field.enum(MESSAGE_STATES),
        /** Why it was skipped, or its provider's error code once it failed. */
        reason: field.string().optional(),
        /** The content sealed by its producer to the home's message key, absent until sending. */
        ciphertext: field.string(identity.Ciphertext).sensitive().optional(),
        /** When it settled, absent while it waits or sends. */
        settledAt: field.time().optional(),
        /** When the next attempt is due after a temporary refusal, absent once settled. */
        retryAt: field.time().optional(),
    },
    permissions: ["read", "send"],
    // expire a settled message 7 days after it settles, and a scheduled one 7 days past its time
    expiring: [
        {
            after: { days: 7 },
            from: "settledAt",
            where: { status: { in: ["skipped", "sent", "delivered", "failed"] } },
        },
        { after: { days: 7 }, from: "dueAt", where: { status: "scheduled" } },
    ],
    constraints: {
        source: {
            kind: "index",
            on: ["sourcePackageId", "sourceType", "sourceScope", "sourceId", "createdAt"],
        },
        due: { kind: "index", on: ["scope", "status", "dueAt"] },
        waiting: {
            kind: "check",
            condition: {
                OR: [
                    { status: { notIn: ["scheduled", "pending"] } },
                    { settledAt: { isNull: true } },
                ],
            },
        },
        scheduled: {
            kind: "check",
            condition: {
                OR: [
                    { status: { ne: "scheduled" } },
                    { ciphertext: { isNull: true }, retryAt: { isNull: true } },
                ],
            },
        },
        pending: {
            kind: "check",
            condition: { OR: [{ status: { ne: "pending" } }, { ciphertext: { isNotNull: true } }] },
        },
        settled: {
            kind: "check",
            condition: {
                OR: [
                    { status: { in: ["scheduled", "pending"] } },
                    { settledAt: { isNotNull: true }, retryAt: { isNull: true } },
                ],
            },
        },
        reason: {
            kind: "check",
            condition: {
                OR: [
                    { status: { in: ["skipped", "failed"] }, reason: { isNotNull: true } },
                    { status: { notIn: ["skipped", "failed"] }, reason: { isNull: true } },
                ],
            },
        },
    },
})
    .methods((method) => ({
        /** Create a message waiting for its time, or sending with its sealed content. */
        create: method.create("send", {
            fields: [
                "to",
                "sourcePackageId",
                "sourceType",
                "sourceScope",
                "sourceId",
                "dueAt",
                "isSummarized",
            ],
            variants: { status: { scheduled: [], pending: ["ciphertext"] } },
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
        /** Send a scheduled message with its sealed content, or show a desktop message. */
        send: method.mutation({
            permission: "send",
            isInternal: true,
            input: schema.object({
                /** Where it goes now, as the recipient's contact reads at its time. */
                to: Destination,
                /** The content sealed to the home's message key, absent for a desktop. */
                ciphertext: identity.Ciphertext.exactOptional(),
            }),
        }),
        /** Move a scheduled message to a later time, or into a scheduled summary. */
        defer: method.mutation({
            permission: "send",
            isInternal: true,
            input: schema.object({
                /** When it is due. */
                dueAt: Instant,
                /** Whether it goes out in a scheduled summary. */
                isSummarized: schema.boolean(),
            }),
        }),
        /** Settle a scheduled message without sending it. */
        skip: method.mutation({
            permission: "send",
            isInternal: true,
            input: schema.object({
                /** Why it was skipped. */
                reason: schema.string().min(1),
            }),
        }),
        /** Record a desktop client showing a sent message, as its receipt. */
        receive: method.mutation({ permission: "read" }),
    }))
    .implement({
        methods: {
            receive: async (call) => {
                // keep a message already delivered
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
