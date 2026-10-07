import { definePackage } from "@destack/package/declare";
import { stripeSecretKey, stripeWebhookSecret } from "./stripe/secret.ts";

/** The handle stacks import to install this package, binding the Stripe secrets it reads. */
export default definePackage({
    secrets: { "stripe-secret-key": stripeSecretKey, "stripe-webhook": stripeWebhookSecret },
});
