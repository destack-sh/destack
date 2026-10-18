import { entitlement } from "@destack/account/object";
import { defineAdapter, definePackage } from "@destack/runtime";
import {
    charge,
    checkoutSession,
    customer,
    invoice,
    price,
    product,
    productFeature,
    seller,
    subscription,
    subscriptionItem,
    usage,
} from "./object/index.ts";
import { stripeSecretKey } from "./provider/stripe/secret.ts";

/** The name the platform stack installs finance under in each residency home. */
export const FINANCE_INSTALLATION = "finance";

/** The handle stacks install finance by, and clients call its products and prices by. */
export default definePackage({
    objects: {
        customer,
        seller,
        product,
        productFeature,
        price,
        subscription,
        subscriptionItem,
        checkoutSession,
        charge,
        invoice,
        usage,
    },
    adapters: { stripe: defineAdapter("stripe", { path: "/stripe" }) },
    secrets: { "stripe-secret-key": stripeSecretKey },
    // write each account's entitlements at the universe
    permissions: { installation: { universe: [entitlement.permission("provide")] } },
});
