import { account, entitlement, residency } from "@destack/account/object";
import { defineAdapter, definePackage } from "@destack/object";
import { FinanceError } from "./error/error.ts";
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
    errors: FinanceError,
    // mirror the provider's billing in its home, write entitlements and serve its residency's accounts
    permissions: {
        installation: {
            space: [
                customer.permission("provide"),
                seller.permission("provide"),
                subscription.permission("provide"),
                checkoutSession.permission("provide"),
                invoice.permission("provide"),
                charge.permission("provide"),
            ],
            universe: [
                entitlement.permission("provide"),
                account.permission("serve"),
                residency.permission("locate"),
            ],
        },
    },
});
