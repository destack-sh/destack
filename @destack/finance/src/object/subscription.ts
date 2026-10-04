import { through } from "@destack/access";
import { account } from "@destack/account/object";
import { check, index, sql, type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { BILLING_PERMISSIONS } from "./customer.ts";
import { FeatureGrant } from "./product.ts";
import { price, PriceTerms } from "./price.ts";
import { seller } from "./seller.ts";

/** The states in which a subscription grants its products' features. */
export const GRANTING_STATES = ["trialing", "active", "past_due"] as const;

/** An account's recurring purchase of a seller's prices, billed each period by the provider. */
export const subscription = defineObject({
    name: "subscription",
    plural: "subscriptions",
    scope: account,
    fields: {
        /** The seller billing the subscription. */
        seller: field.reference(seller, { qualified: true }),
        /** Where the provider's billing of the subscription stands. */
        status: field.state({
            initial: "incomplete",
            transitions: {
                trial: { from: ["incomplete"], to: "trialing", permission: "provide" },
                activate: {
                    from: ["incomplete", "trialing", "past_due", "unpaid", "paused"],
                    to: "active",
                    permission: "provide",
                },
                fail: { from: ["trialing", "active"], to: "past_due", permission: "provide" },
                lapse: { from: ["past_due"], to: "unpaid", permission: "provide" },
                pause: { from: ["trialing", "active"], to: "paused", permission: "provide" },
                expire: { from: ["incomplete"], to: "incomplete_expired", permission: "provide" },
                cancel: {
                    from: ["incomplete", "trialing", "active", "past_due", "unpaid", "paused"],
                    to: "canceled",
                    permission: "provide",
                },
            },
        }),
        /** When the current billing period started, in UTC epoch milliseconds. */
        currentPeriodStart: field.time(),
        /** When the current billing period ends, in UTC epoch milliseconds. */
        currentPeriodEnd: field.time(),
        /** Whether the subscription ends with the current period. */
        cancelAtPeriodEnd: field.boolean().default(false),
        /** When the trial ends. */
        trialEnd: field.time().optional(),
        /** When the subscription was canceled. */
        canceledAt: field.time().optional(),
        /** The provider's identifier of the subscription. */
        providerId: field.string().optional(),
    },
    permissions: BILLING_PERMISSIONS,
    reserved: ["provide"],
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        // TODO #Incomplete: create subscriptions from completed checkout sessions
        create: method.create(null, { isSystem: true }),
        update: method.update("bill", { fields: ["cancelAtPeriodEnd"] }),
        /** Record the provider's billing period, trial and identifier of the subscription. */
        record: method.update(null, {
            isSystem: true,
            fields: ["currentPeriodStart", "currentPeriodEnd", "trialEnd", "providerId"],
        }),
    }),
    constraints: (entry) => [
        index("subscription_status").on(entry.scope, entry.status),
        check("subscription_period", sql`${entry.currentPeriodStart} < ${entry.currentPeriodEnd}`),
    ],
});
/** A persisted subscription. */
export type Subscription = Select<typeof subscription.table>;

/** A price a subscription bills each period, for a quantity. */
export const subscriptionItem = defineObject({
    name: "subscription-item",
    plural: "subscriptionItems",
    scope: account,
    nested: { in: subscription, delete: "cascade", receive: "provide" },
    fields: {
        /** The seller's price. */
        price: field.reference(price, { qualified: true }),
        /** The units billed each period. */
        quantity: field.integer().default(1),
        /** The price's terms at creation. */
        terms: field.json(PriceTerms),
        /** The product's feature grants at creation, which entitlements derive from. */
        grants: field.json(schema.array(FeatureGrant)),
    },
    permissions: {
        read: through("parent", "read"),
    },
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create(null, { isSystem: true }),
        update: method.update(null, { isSystem: true, fields: ["quantity"] }),
        delete: method.delete(null, { isSystem: true }),
    }),
    constraints: (entry) => [check("subscription_item_quantity", sql`${entry.quantity} >= 0`)],
});
/** A persisted subscription item. */
export type SubscriptionItem = Select<typeof subscriptionItem.table>;
