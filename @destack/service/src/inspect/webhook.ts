import { reference } from "@destack/package/declare";
import { WebhookDescription, type Webhook } from "../webhook/index.ts";

/** Describe a declared webhook with a reference to its secret declaration. */
export function describeWebhook(webhook: Webhook): WebhookDescription {
    return WebhookDescription.parse({
        name: webhook.name,
        version: webhook.version,
        verification: webhook.verification,
        secret: reference(webhook.secret),
    });
}
