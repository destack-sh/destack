import { defineSecret } from "@destack/vault/declare";
import { defineWebhook } from "@destack/workflow";
import type {} from "@destack/package/import-meta";
import { StripeSignature } from "../provider/stripe/signature.ts";

/** The secret Stripe signs the webhook endpoint's deliveries with. */
export const stripeWebhookSecret = defineSecret({ name: "stripe-webhook" });

/** Stripe's deliveries of the events finance follows, verified with the endpoint's secret. */
export const stripeWebhook = defineWebhook("stripe", {
    signature: new StripeSignature(),
    secret: stripeWebhookSecret,
});
