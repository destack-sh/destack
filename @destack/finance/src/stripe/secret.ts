import { defineSecret } from "@destack/secret";

/** The platform account's secret Stripe API key, which a deployment binds from a vault. */
export const stripeSecretKey = defineSecret({ name: "stripe-secret-key" });

/** The secret Stripe signs the finance webhook endpoint's deliveries with. */
export const stripeWebhookSecret = defineSecret({ name: "stripe-webhook" });
