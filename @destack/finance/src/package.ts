import { t } from "@destack/locale";
import { claim } from "@destack/account/object";
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
    quota,
    seller,
    subscription,
    subscriptionItem,
} from "./object/index.ts";
import { stripe } from "./declare/connection.ts";
import { stripeWebhook, stripeWebhookSecret } from "./declare/webhook.ts";

/** The handle stacks install finance by, and clients call its products and prices by. */
export default definePackage({
    title: t.context("title")`Finance`,
    description: t`Sell products with features, bill accounts for subscriptions and usage, and derive their quotas.`,
    icon: "receipt",
    color: "purple",
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
        quota,
    ],
    mounts: [stripeWebhook],
    resources: [stripe, stripeWebhookSecret],
    errors: FinanceError,
    // mirror the provider's billing in its home, derive its buyers' quotas and resolve the holders of names
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
                    quota.permission("provide"),
                ],
                universe: [claim.permission("resolve")],
            },
        },
    },
});
