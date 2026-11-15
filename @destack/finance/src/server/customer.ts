import { account } from "@destack/account/object";
import type { Relation } from "@destack/db";
import { tag } from "@destack/registry/object";
import { usage } from "@destack/stack/object";
import {
    customer,
    Customer,
    product,
    productFeature,
    subscription,
    subscriptionItem,
} from "../object/index.ts";
import { CustomerEntitlement } from "./entitlement.ts";
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

/** Implement customers: announce each account's usage alerts, and derive its entitlements. */
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
        projections: [
            {
                // derive each replicated account's entitlements
                name: "entitle",
                key: "accountId",
                all: { table: account.table, column: "id" },
                watches: [
                    { table: account.table },
                    { table: usage.table, on: { id: "scope" } },
                    { table: subscription.table, on: { id: "accountId" } },
                    {
                        table: subscriptionItem.table,
                        path: [ITEM_SUBSCRIPTION],
                        on: { id: "accountId" },
                    },
                    // derive every account again once a default product or its grants change
                    { table: product.table, where: { isDefault: true }, isListing: true },
                    {
                        table: productFeature.table,
                        path: [FEATURE_DEFAULT_PRODUCT],
                        isListing: true,
                    },
                    // derive every account again once the metering tag moves
                    { table: tag.table, where: { name: metering.tag }, isListing: true },
                ],
                project: (accountId, { store }) =>
                    new CustomerEntitlement(store, home, metering).derive(accountId),
            },
        ],
    });
}
