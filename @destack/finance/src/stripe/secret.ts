import { defineSecret } from "@destack/vault/declare";

/** The platform account's secret Stripe API key, which a deployment binds from a vault. */
export const stripeSecretKey = defineSecret({ name: "stripe-secret-key" });
