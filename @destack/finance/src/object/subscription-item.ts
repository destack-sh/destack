import { through } from "@destack/access";
import { account } from "@destack/account/object";
import { check, sql, type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { FeatureGrant } from "./product-feature.ts";
import { subscription } from "./subscription.ts";
import { price, PriceTerms } from "./price.ts";

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
        /** The provider's identifier of the item, absent on a subscription no provider bills. */
        providerId: field.string().optional(),
    },
    permissions: {
        read: through("parent", "read"),
    },
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create(null, { isSystem: true }),
        update: method.update(null, { isSystem: true, fields: ["quantity", "providerId"] }),
        delete: method.delete(null, { isSystem: true }),
    }),
    constraints: (entry) => [check("subscription_item_quantity", sql`${entry.quantity} >= 0`)],
});
/** A persisted subscription item. */
export type SubscriptionItem = Select<typeof subscriptionItem.table>;
