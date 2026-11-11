import { account, residency } from "@destack/account/object";
import { definePackage } from "@destack/object";
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
} from "./object/index.ts";
import { stripe } from "./declare/connection.ts";
import { stripeWebhook, stripeWebhookSecret } from "./declare/webhook.ts";

/** The name the platform stack installs finance under in each residency home. */
export const FINANCE_INSTALLATION = "finance";

/** The handle stacks install finance by, and clients call its products and prices by. */
export default definePackage({
    objects: [
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
    ],
    mounts: [stripeWebhook],
    resources: [stripe, stripeWebhookSecret],
    errors: FinanceError,
    // mirror the provider's billing in its home, write entitlements and serve its residency's accounts
    access: {
        permissions: {
            space: {
                space: [
                    customer.permission("provide"),
                    seller.permission("provide"),
                    subscription.permission("provide"),
                    checkoutSession.permission("provide"),
                    invoice.permission("provide"),
                    charge.permission("provide"),
                ],
                universe: [
                    account.permission("entitle"),
                    account.permission("serve"),
                    residency.permission("locate"),
                ],
            },
        },
    },
});
