import { account, allowance, key, machine, organisation } from "@destack/account/object";
import { journal } from "@destack/audit/stack";
import { defineDatabase, type Table } from "@destack/db";
import { eventTables } from "@destack/event/stack";
import { defineBucket } from "@destack/bucket";
import { usage } from "../meter/usage.ts";
import { announcement } from "@destack/notification";
import {
    charge,
    customer,
    entitlement,
    invoice,
    price,
    product,
    productFeature,
    checkoutSession,
    seller,
    subscription,
    subscriptionItem,
} from "../object/index.ts";

/** The finance service's tables. */
export const financeTables: readonly Table[] = [
    ...customer.tables,
    ...seller.tables,
    ...product.tables,
    ...productFeature.tables,
    ...price.tables,
    ...subscription.tables,
    ...subscriptionItem.tables,
    ...checkoutSession.tables,
    ...entitlement.tables,
    ...allowance.tables,
    ...eventTables([usage]),
    ...invoice.tables,
    ...charge.tables,
    ...announcement.tables,
    journal,
];

/** The finance database, with copies of accounts, organisations, machines and keys. */
export const financeDatabase = defineDatabase({
    name: "main",
    tables: financeTables,
    copies: [account.table, organisation.table, machine.table, key.table],
});

/** The bucket keeping usage segments. */
export const usageBucket = defineBucket({ name: "usage", spec: { write: "system" } });
