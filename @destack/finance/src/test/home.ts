import { installation, build, installationRevision, usage } from "@destack/stack/object";
import {
    account,
    claim,
    key,
    machine,
    organization,
    region,
    space,
    user,
} from "@destack/account/object";
import { call, journal } from "@destack/object";
import { defineDatabase } from "@destack/db";
import { packageObject, release, tag } from "@destack/registry/object";
import { messageTables } from "@destack/message/stack";
import { notificationTables } from "@destack/notification/stack";
import {
    charge,
    checkoutSession,
    customer,
    invoice,
    price,
    product,
    productFeature,
    quota,
    seller,
    subscription,
    subscriptionItem,
} from "../object/index.ts";

/** The tables of the platform home's main database that finance uses, with its replicas. */
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
            quota,
            invoice,
            charge,
            usage,
        ].flatMap((type) => type.tables),
        ...messageTables,
        ...notificationTables,
        ...[user, installation, installationRevision, build].flatMap((type) => type.tables),
        ...[packageObject, release, tag, claim].flatMap((type) => type.tables),
        journal,
        ...call.tables,
    ],
    replicated: [
        account.table,
        organization.table,
        space.table,
        machine.table,
        key.table,
        region.table,
    ],
});
