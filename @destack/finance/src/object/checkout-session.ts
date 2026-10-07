import { account } from "@destack/account/object";
import { index, type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { BILLING_PERMISSIONS } from "./customer.ts";
import { Currency } from "../rate/amount.ts";
import { product } from "./product.ts";
import { seller } from "./seller.ts";
import { ProviderSubscription, ProviderWork, subscription } from "./subscription.ts";

/** An account's purchase in progress of a product's recurring prices. */
export const checkoutSession = defineObject({
    name: "checkout-session",
    plural: "checkoutSessions",
    scope: account,
    fields: {
        /** The seller of the product. */
        seller: field.reference(seller, { qualified: true }),
        /** The seller's product. */
        product: field.reference(product, { qualified: true }),
        /** The currency of the prices bought. */
        currency: field.string(Currency),
        /** Where the purchase stands. */
        status: field.state({
            initial: "open",
            transitions: {
                complete: { from: ["open"], to: "complete", permission: "provide" },
                expire: { from: ["open"], to: "expired", permission: "provide" },
            },
        }),
        /** The provider's page the buyer pays at. */
        url: field.string(schema.url()),
        /** The subscription the completed session started. */
        subscription: field.reference(subscription).optional(),
        /** The provider's identifier of the session. */
        providerId: field.string(),
    },
    permissions: BILLING_PERMISSIONS,
    reserved: ["provide"],
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("bill", {
            fields: ["product", "currency"],
            prepared: ProviderWork,
        }),
        /** Complete the session with its subscription. */
        settle: method.mutation({
            permission: null,
            isSystem: true,
            input: schema.object({
                /** The subscription as the provider bills it. */
                subscription: ProviderSubscription,
            }),
        }),
        /** Record the subscription the session started. */
        record: method.update(null, { isSystem: true, fields: ["subscriptionId"] }),
    }),
    constraints: (entry) => [index("checkout_session_provider").on(entry.providerId)],
});
/** A persisted checkout session. */
export type CheckoutSession = Select<typeof checkoutSession.table>;
