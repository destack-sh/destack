import { WebhookDescription, type Webhook } from "../webhook/index.ts";

/** Describe a webhook. */
export function describeWebhook(webhook: Webhook): WebhookDescription {
    return WebhookDescription.parse({
        name: webhook.name,
        verification: webhook.verification,
        route: webhook.route,
    });
}
