import { schema } from "@destack/schema";

/** The headers every webhook post sets itself, which no endpoint's secret header may take. */
export const RESERVED_HEADERS: ReadonlySet<string> = new Set([
    "content-length",
    "content-type",
    "host",
    "transfer-encoding",
    "webhook-id",
    "webhook-signature",
    "webhook-timestamp",
]);

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
    /** The webhook channel. */
    channel: schema.literal("webhook"),
});

/** A webhook's event, or a batch of records as a JSON array or JSON lines. */
export const WebhookContent = schema.discriminatedUnion("format", [
    schema.object({
        /** The webhook channel. */
        channel: schema.literal("webhook"),
        /** The event type, such as `issue.regressed`. */
        type: schema.string().min(1),
        /** How the body encodes the data: as an event's envelope. */
        format: schema.literal("event"),
        /** The event's data. */
        data: schema.json(),
    }),
    schema.object({
        /** The webhook channel. */
        channel: schema.literal("webhook"),
        /** The batch's record kind, like `call`. */
        type: schema.string().min(1),
        /** How the body encodes the records: a JSON array or JSON lines. */
        format: schema.enum(["json", "ndjson"]),
        /** The batch's records. */
        data: schema.array(schema.json()),
    }),
]);

/** A webhook's event, or a batch of records. */
export type WebhookContent = schema.Infer<typeof WebhookContent>;

/** Encode a webhook body: an event's envelope, or a batch as a JSON array or JSON lines. */
export function encodeBody(createdAt: number, content: WebhookContent): string {
    // wrap an event in its envelope
    if (content.format === "event") {
        return JSON.stringify({
            type: content.type,
            timestamp: new Date(createdAt).toISOString(),
            data: content.data,
        });
    }

    // list a batch's records as a JSON array or JSON lines
    return content.format === "json"
        ? JSON.stringify(content.data)
        : content.data.map((record) => `${JSON.stringify(record)}\n`).join("");
}
