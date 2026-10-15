import { schema } from "@destack/schema";

/** How a webhook message encodes its body. */
export const WEBHOOK_FORMATS = ["event", "json", "ndjson"] as const;

/** How a webhook endpoint authenticates its messages: a signature, or a header. */
export const WebhookAuthentication = schema.discriminatedUnion("kind", [
    schema.object({
        kind: schema.literal("signature"),
    }),
    schema.object({
        kind: schema.literal("header"),
        /** The header with the secret's value, such as an API key header (RFC 9110 5.1). */
        name: schema.string().regex(/^[!#$%&'*+.^_`|~0-9A-Za-z-]+$/u),
    }),
]);

/** How a webhook endpoint authenticates its messages. */
export type WebhookAuthentication = schema.Infer<typeof WebhookAuthentication>;

/** A webhook message's destination: the endpoint its source names, with its URL and secret. */
export const WebhookDestination = schema.object({
    channel: schema.literal("webhook"),
});

/** A webhook's event, or a batch of records. */
export const WebhookContent = schema.object({
    channel: schema.literal("webhook"),
    /** The event type, such as `issue.regressed`, or a batch's record kind, like `call`. */
    type: schema.string().min(1),
    /** How the body encodes the data. */
    format: schema.enum(WEBHOOK_FORMATS),
    /** The event's data, or the batch's records. */
    data: schema.json(),
});

/** Encode a webhook body: an event's envelope, or a batch as a JSON array or JSON lines. */
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
        throw new TypeError(`batch ${id} has no records`);
    }

    return content.format === "json"
        ? JSON.stringify(records)
        : records.map((record) => `${JSON.stringify(record)}\n`).join("");
}
