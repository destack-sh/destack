import { account } from "@destack/account/object";
import { check, index, sql, type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { BILLING_PERMISSIONS } from "./customer.ts";
import { FeatureGrant } from "./product.ts";
import { price, PriceTerms } from "./price.ts";
import { seller } from "./seller.ts";

/** An account's one-time purchase of a seller's price. */
export const purchase = defineObject({
    name: "purchase",
    plural: "purchases",
    scope: account,
    fields: {
        /** The seller charging the purchase. */
        seller: field.reference(seller, { qualified: true }),
        /** The seller's price. */
        price: field.reference(price, { qualified: true }),
        /** The units bought. */
        quantity: field.integer().default(1),
        /** The price's terms at creation. */
        terms: field.json(PriceTerms),
        /** The product's feature grants at creation, which entitlements derive from. */
        grants: field.json(schema.array(FeatureGrant)),
        /** The amount charged, in the minor unit of the terms' currency. */
        amount: field.integer(),
        /** Where the provider's charge stands. */
        status: field.state({
            initial: "pending",
            transitions: {
                pay: { from: ["pending"], to: "paid", permission: "provide" },
                fail: { from: ["pending"], to: "failed", permission: "provide" },
                refund: { from: ["paid"], to: "refunded", permission: "provide" },
            },
        }),
        /** When the charge succeeded. */
        paidAt: field.time().optional(),
        /** The provider's identifier of the charge. */
        providerId: field.string().optional(),
    },
    permissions: BILLING_PERMISSIONS,
    reserved: ["provide"],
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        // TODO #Incomplete: create purchases from completed checkout sessions
        create: method.create(null, { isSystem: true }),
        /** Record the provider's identifier of the charge. */
        record: method.update(null, { isSystem: true, fields: ["providerId"] }),
    }),
    constraints: (entry) => [
        index("purchase_status").on(entry.scope, entry.status),
        check("purchase_amount", sql`${entry.quantity} >= 1 AND ${entry.amount} >= 0`),
    ],
});
/** A persisted purchase. */
export type Purchase = Select<typeof purchase.table>;
