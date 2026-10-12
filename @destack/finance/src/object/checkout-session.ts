import { account, space } from "@destack/account/object";
import { type Select, uniqueIndex } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { BILLING_PERMISSIONS } from "./customer.ts";
import { Currency } from "../rate/amount.ts";
import { PriceMirror } from "./price.ts";
import { product } from "./product.ts";
import { seller } from "./seller.ts";
import { ProviderSubscription, subscription } from "./subscription.ts";

/** An account's purchase in progress of a product's recurring prices. */
export const checkoutSession = defineObject({
    name: "checkout-session",
    plural: "checkoutSessions",
    scope: space,
    owner: "account",
    fields: {
        /** The account the row belongs to. */
        account: field.reference(account).relate("account"),
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
            fields: ["accountId", "product", "currency"],
            input: schema.object({
                /** The page Checkout returns the buyer to once they paid. */
                successUrl: schema.url(),
                /** The page Checkout returns the buyer to when they leave without paying. */
                cancelUrl: schema.url(),
            }),
            prepared: schema.object({
                /** The provider's identifier of the buying customer. */
                customer: schema.string(),
                /** The checkout the buyer pays at. */
                session: schema.object({ providerId: schema.string(), url: schema.url() }),
                /** The buyer's earlier open session with the seller, expired for this one. */
                replaced: schema.string().nullable(),
                /** The product and prices the checkout mirrored first. */
                mirrored: PriceMirror,
            }),
        }),
        /** Complete the session with its subscription. */
        settle: method.mutation({
            permission: "provide",
            isInternal: true,
            input: schema.object({
                /** The subscription as the provider bills it. */
                subscription: ProviderSubscription,
            }),
        }),
        /** Record the subscription the session started. */
        record: method.update("provide", { isInternal: true, fields: ["subscriptionId"] }),
    }),
    constraints: (entry) => [
        uniqueIndex("checkout_session_provider").on(entry.providerId),
        uniqueIndex("checkout_session_open")
            .on(entry.accountId, entry.seller)
            .where({ status: "open" }),
    ],
});
/** A persisted checkout session. */
export type CheckoutSession = Select<typeof checkoutSession.table>;
