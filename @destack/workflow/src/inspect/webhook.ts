import { DeclarationReference } from "@destack/package/declare";
import { type Webhook, WebhookDescription } from "../webhook/webhook.ts";

/** Describe a webhook: its path, its signature's name and its bound secret. */
export function describeWebhook(webhook: Webhook): WebhookDescription {
    const { name, template, signature, secret } = webhook;

    return WebhookDescription.parse({
        name,
        template: template.template,
        verification: signature.name,
        secret: DeclarationReference.of(secret),
    });
}
