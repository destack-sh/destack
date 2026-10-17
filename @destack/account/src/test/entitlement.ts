import { and, type DatabaseConnection, eq } from "@destack/db";
import { PackageId } from "@destack/package";
import type { FeatureReference } from "@destack/runtime";
import { Identifier } from "@destack/schema";
import { type Publisher, replica, replicaPosition, type Subscription } from "@destack/sync";
import { until } from "@destack/runtime/timer";
import { Shape } from "@destack/object";
import {
    entitlement,
    type Entitlement,
    type EntitlementResource,
    entitlementShape,
} from "../object/entitlement.ts";
import { space } from "../object/space.ts";

/** The package declaring the features a test entitles accounts to. */
const TEST_PACKAGE = PackageId.parse("package-019f5530-8000-7000-8000-0000000000f9");

/** The home space storing the entitlements a test writes. */
const TEST_HOME = space.identifier("space-019f5530-8000-7000-8000-0000000000f8");

/** Accounts' entitlements as a test writes them into a follower's replica. */
export const TestEntitlement = {
    /** A publisher that sends nothing, as the test writes the replicas itself. */
    publisher: {
        stream: async function* (_subscription: Subscription, signal: AbortSignal) {
            await until(signal);
            yield* [];
        },
    } satisfies Publisher,

    /** Apply an entitlement to a resource or feature to an account as its replica would. */
    async set(
        database: DatabaseConnection,
        accountId: Identifier<"account">,
        granted: EntitlementResource | { readonly feature: FeatureReference },
        terms: Pick<Entitlement, "kind"> &
            Partial<Pick<Entitlement, "value" | "limit" | "usage" | "state">>,
    ): Promise<void> {
        // mark the account's replica complete once
        const now = Date.now();
        const key = {
            name: Shape.replicaName(entitlementShape, { scope: TEST_HOME, account: accountId }),
            scope: TEST_HOME,
        };
        await database
            .insert(replica)
            .values({ ...key, tables: [] })
            .onConflictDoNothing();
        await database
            .insert(replicaPosition)
            .values({ ...key, epoch: "test", sequence: 1, confirmedAt: now })
            .onConflictDoNothing();

        // replace the entitlement, a resource's under a test feature of its name
        const feature =
            typeof granted === "string"
                ? { packageId: TEST_PACKAGE, name: granted, resource: granted }
                : { ...granted.feature, resource: null };
        await database
            .delete(entitlement.table)
            .where(
                and(
                    eq(entitlement.table.accountId, accountId),
                    eq(entitlement.table.packageId, feature.packageId),
                    eq(entitlement.table.feature, feature.name),
                ),
            );
        await database.insert(entitlement.table).values({
            id: Identifier.create("entitlement"),
            scope: TEST_HOME,
            accountId,
            packageId: feature.packageId,
            feature: feature.name,
            resource: feature.resource,
            ...terms,
            createdAt: now,
            updatedAt: now,
        });
    },
};
