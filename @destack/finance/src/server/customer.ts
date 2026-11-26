import { account } from "@destack/account/object";
import type { Relation } from "@destack/db";
import { tag } from "@destack/registry/object";
import { usage } from "@destack/stack/object";
import {
    charge,
    customer,
    Customer,
    product,
    productFeature,
    seller,
    subscription,
    subscriptionItem,
} from "../object/index.ts";
import { CustomerQuota } from "./quota.ts";
import type { Metering } from "./metering.ts";

/** The subscription an item belongs to. */
const ITEM_SUBSCRIPTION: Relation = {
    table: subscription.table,
    cardinality: "one",
    on: { kind: "key", column: "id", parent: "parentId" },
};

/** The default product a feature belongs to. */
const FEATURE_DEFAULT_PRODUCT: Relation = {
    table: product.table,
    cardinality: "one",
    on: { kind: "key", column: "id", parent: "parentId" },
    where: { isDefault: true },
};

/** Implement customers: announce each account's usage alerts, and derive its quotas. */
export function implementCustomer(home: string, metering: Metering): typeof customer {
    return customer.implement({
        methods: {
            announce: async (call) => {
                // announce each usage alert once per billing period
                for (const alert of call.input.alerts) {
                    await Customer.announce(call.input.accountId, alert, call);
                }

                return {};
            },
        },
        controllers: [
            {
                // derive each replicated account's quotas
                name: "quota",
                by: ["accountId"],
                for: { table: account.table, column: "id" },
                reads: [
                    // derive an account again once it arrives or its usage is recorded
                    { table: account.table, fields: [] },
                    { table: usage.table, fields: [], on: { id: "scope" } },
                    // derive it again once a subscription's standing, period or limit changes
                    {
                        table: subscription.table,
                        fields: [
                            "status",
                            "sellerId",
                            "currentPeriodStart",
                            "currentPeriodEnd",
                            "spendingLimit",
                        ],
                        on: { id: "accountId" },
                    },
                    {
                        table: subscriptionItem.table,
                        fields: ["quantity", "terms", "grants"],
                        path: [ITEM_SUBSCRIPTION],
                        on: { id: "accountId" },
                    },
                    // derive it again once a usage charge of its period is rated
                    { table: charge.table, fields: ["billedCost"], on: { id: "accountId" } },
                    // derive every account again once a default product, its seller or its grants change
                    {
                        table: product.table,
                        where: { isDefault: true },
                        fields: ["isDefault", "archivedAt", "accountId"],
                        isListing: true,
                    },
                    { table: seller.table, fields: ["accountId"], isListing: true },
                    {
                        table: productFeature.table,
                        fields: ["packageId", "feature", "value"],
                        path: [FEATURE_DEFAULT_PRODUCT],
                        isListing: true,
                    },
                    // derive every account again once the metering tag moves
                    {
                        table: tag.table,
                        where: { name: metering.tag },
                        fields: ["version"],
                        isListing: true,
                    },
                ],
                reconcile: ({ key, store }) => new CustomerQuota(store, home, metering).derive(key),
            },
        ],
    });
}
