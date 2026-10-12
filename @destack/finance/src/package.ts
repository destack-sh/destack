import { definePackage } from "@destack/package/declare";
import { financeDatabase, usageBucket } from "./stack/index.ts";
import { stripeSecretKey } from "./stripe/secret.ts";

/** The handle stacks import to install finance, with its database, bucket and Stripe secrets. */
export default definePackage({
    resources: { main: financeDatabase, usage: usageBucket },
    secrets: { "stripe-secret-key": stripeSecretKey },
});
