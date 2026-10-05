import { schema } from "@destack/schema";

/** The HTTPS webhook endpoint a message goes to. */
export const WebhookRecipient = schema.object({
    channel: schema.literal("webhook"),
    /** The endpoint URL. */
    url: schema.url({ protocol: /^https$/u }),
});

/** A webhook's event. */
export const WebhookContent = schema.object({
    channel: schema.literal("webhook"),
    /** The event type, such as `issue.regressed`. */
    type: schema.string().min(1),
    /** The event's data. */
    data: schema.json(),
});
