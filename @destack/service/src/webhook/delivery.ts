import { defineSchema, schema } from "@destack/schema";

/** The values of a webhook route's parameters, by name. */
export const WebhookParameters = defineSchema(
    schema.record(schema.string(), schema.string().min(1)),
);
/** The values of a webhook route's parameters, by name. */
export type WebhookParameters = schema.Infer<typeof WebhookParameters>;

/** One verified webhook delivery. */
export const WebhookDelivery = defineSchema(
    schema.object({
        /** The sender's delivery identifier. */
        id: schema.string().min(1),
        /** The event type, such as push. */
        event: schema.string().min(1),
        /** The decoded body. */
        payload: schema.json(),
        /** The route's parameters. */
        parameters: WebhookParameters,
        /** The receiving time, in UTC epoch milliseconds. */
        receivedAt: schema.number().int(),
    }),
);
/** One verified webhook delivery. */
export type WebhookDelivery = schema.Infer<typeof WebhookDelivery>;
