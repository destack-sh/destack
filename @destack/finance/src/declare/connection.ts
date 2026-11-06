import { defineConnection } from "@destack/connection/declare";
import type {} from "@destack/package/import-meta";

/** The Stripe API's origin. */
export const STRIPE_API = "https://api.stripe.com";

/** The connection finance calls Stripe through, whose key the egress applies. */
export const stripe = defineConnection("stripe", { resource: STRIPE_API });
