import { account, allowance, key, machine, organisation } from "@destack/account/object";
import { journal } from "@destack/audit/stack";
import { defineDatabase } from "@destack/db";
import { eventTables } from "@destack/event/stack";
import { announcement } from "@destack/notification";
import { usage } from "../meter/usage.ts";
import {
    charge,
    checkoutSession,
    customer,
    entitlement,
    invoice,
    price,
    product,
    productFeature,
    seller,
    subscription,
    subscriptionItem,
} from "../object/index.ts";

/** The tables of the residency home's main database that finance uses, with its copies. */
export const homeDatabase = defineDatabase({
    name: "main",
    tables: [
        ...[
            customer,
            seller,
            product,
            productFeature,
            price,
            subscription,
            subscriptionItem,
            checkoutSession,
            entitlement,
            invoice,
            charge,
            allowance,
            announcement,
        ].flatMap((type) => type.tables),
        ...eventTables([usage]),
        journal,
    ],
    copies: [account.table, organisation.table, machine.table, key.table],
});
