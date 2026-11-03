import {
    account,
    claim,
    entitlement,
    key,
    machine,
    organization,
    residency,
    space,
    user,
} from "@destack/account/object";
import { build, installationRevision, rollout } from "@destack/stack/object";
import { call, installation, journal } from "@destack/object";
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
    seller,
    subscription,
    subscriptionItem,
} from "../object/index.ts";
import { usage } from "@destack/stack/object";

/** The tables of the residency home's main database that finance uses, with its replicas. */
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
            usage,
        ].flatMap((type) => type.tables),
        ...messageTables,
        ...notificationTables,
        ...[user, installation, rollout, installationRevision, build].flatMap(
            (type) => type.tables,
        ),
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
        residency.table,
    ],
});
