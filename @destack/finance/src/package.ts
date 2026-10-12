import { definePackage } from "@destack/package/declare";
import { stripeSecretKey } from "./stripe/secret.ts";

/** The handle stacks import to install finance, with its Stripe secret. */
export default definePackage({ secrets: { "stripe-secret-key": stripeSecretKey } });
