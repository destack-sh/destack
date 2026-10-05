import { journal } from "@destack/audit/stack";
import { account, organisation } from "@destack/account/object";
import { defineDatabase, type Table } from "@destack/db";
import {
    customer,
    entitlement,
    invoice,
    meterEvent,
    price,
    product,
    productFeature,
    purchase,
    seller,
    subscription,
    subscriptionItem,
} from "../object/index.ts";

/** The finance service's tables: its objects and its journal. */
export const financeTables: readonly Table[] = [
    ...customer.tables,
    ...seller.tables,
    ...product.tables,
    ...productFeature.tables,
    ...price.tables,
    ...subscription.tables,
    ...subscriptionItem.tables,
    ...purchase.tables,
    ...entitlement.tables,
    ...meterEvent.tables,
    ...invoice.tables,
    journal,
];

/** The finance database, with copies of its residency's accounts and organisations, whose payers customers record. */
export const financeDatabase = defineDatabase({
    name: "main",
    tables: financeTables,
    copies: [account.table, organisation.table],
});
