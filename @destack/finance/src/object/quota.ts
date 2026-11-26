import { through, union } from "@destack/access";
import { account, space } from "@destack/account/object";
import type { Select } from "@destack/db";
import { argument, plural, t } from "@destack/locale";
import { defineObject, field } from "@destack/object";
import { DeclarationName, PackageId } from "@destack/package";
import type {} from "@destack/package/import-meta";
import { USAGE_STATES } from "../feature/allowance.ts";
import { BILLING_PERMISSIONS } from "./billing.ts";

/** An account's usage of a metered feature against its limit, as finance last derived it. */
export const quota = defineObject(
    {
        name: "quota",
        plural: "quotas",
        title: t`${plural(argument("count"), { one: "Quota", other: "Quotas" })}`,
        scope: space,
        owner: "account",
        fields: {
            /** The account using the feature. */
            account: field.reference(account).relate("account"),
            /** The package declaring the feature. */
            packageId: field.string(PackageId),
            /** The feature's name in its package. */
            feature: field.string(DeclarationName),
            /** The sum of the grants'  limits, unlimited without one. */
            limit: field.integer().optional(),
            /** The usage within the current period of the grant resetting first. */
            usage: field.number(),
            /** When the usage resets, absent for usage that never resets. */
            resetAt: field.time().optional(),
            /** Where the usage stands against the limit and the subscription's budget. */
            state: field.enum(USAGE_STATES),
            /** The start of the billing period ending first among the grants granting it. */
            periodStart: field.time(),
        },
        constraints: {
            feature: { kind: "index", on: ["accountId", "packageId", "feature"], unique: true },
        },
        access: {
            permissions: {
                ...BILLING_PERMISSIONS,
                // let the account's readers and the machines serving it read where its usage stands
                read: union(
                    through("account", "read"),
                    through("account", "serve"),
                    through("account", "sell"),
                ),
            },
            reserved: ["provide"],
        },
    },
    (method) => ({
        create: method.create("provide", {
            visibility: "package",
            fields: [
                "accountId",
                "packageId",
                "feature",
                "limit",
                "usage",
                "resetAt",
                "state",
                "periodStart",
            ],
        }),
        update: method.update("provide", {
            visibility: "package",
            fields: ["limit", "usage", "resetAt", "state", "periodStart"],
        }),
        delete: method.delete("provide", { visibility: "package" }),
    }),
);
/** An account's usage of a metered feature against its limit. */
export type Quota = Select<typeof quota.table>;
