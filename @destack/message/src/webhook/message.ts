import { schema } from "@destack/schema";
import { ObjectReference, Subject } from "@destack/sync";

/** How a webhook message encodes its body: a Standard Webhooks event, or a batch of records as one JSON array or one JSON object per line. */
export const WEBHOOK_FORMATS = ["event", "json", "ndjson"] as const;

/** The HTTPS webhook endpoint a message goes to, and how it authenticates the message. */
export const WebhookDestination = schema.object({
    channel: schema.literal("webhook"),
    /** The endpoint URL. */
    url: schema.url({ protocol: /^https$/u }),
    /** How the endpoint authenticates the message with its secret: a Standard Webhooks signature, or the secret's value in a header. */
    authentication: schema.discriminatedUnion("kind", [
        schema.object({
            kind: schema.literal("signature"),
        }),
        schema.object({
            kind: schema.literal("header"),
            /** The header holding the secret's value, such as an intake's API key header, a field name token (RFC 9110 5.1). */
            name: schema.string().regex(/^[!#$%&'*+.^_`|~0-9A-Za-z-]+$/u),
        }),
    ]),
    /** The vault secret authenticating the message. */
    secret: ObjectReference,
    /** The principal the secret is read as, such as the endpoint's user. */
    reader: Subject,
});

/** A webhook's event, or a batch of records. */
export const WebhookContent = schema.object({
    channel: schema.literal("webhook"),
    /** The event type, such as `issue.regressed`, or the kind of a batch's records, such as `audit.call`. */
    type: schema.string().min(1),
    /** How the body encodes the data. */
    format: schema.enum(WEBHOOK_FORMATS),
    /** The event's data, or the batch's records. */
    data: schema.json(),
});

/** Encode a webhook message's body: an event's envelope, or a batch's records as one JSON array or one JSON object per line. */
export function encodeBody(
    id: string,
    createdAt: number,
    content: schema.Infer<typeof WebhookContent>,
): string {
    // wrap an event in its envelope
    if (content.format === "event") {
        return JSON.stringify({
            type: content.type,
            timestamp: new Date(createdAt).toISOString(),
            data: content.data,
        });
    }

    // list a batch's records, refusing a batch without records
    const records = content.data;
    if (!Array.isArray(records)) {
        throw new TypeError(`batch ${id} holds no records`);
    }

    return content.format === "json"
        ? JSON.stringify(records)
        : records.map((record) => `${JSON.stringify(record)}\n`).join("");
}
