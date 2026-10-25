import { and, type DatabaseConnection, eq } from "@destack/db";
import { PackageId } from "@destack/package";
import type { FeatureReference } from "@destack/object";
import { Identifier } from "@destack/schema";
import { type Publisher, replica, replicaPosition, Scope, type Subscription } from "@destack/sync";
import { type Environment, until } from "@destack/environment";
import {
    entitlement,
    type Entitlement,
    type EntitlementResource,
    entitlementShape,
} from "../object/entitlement.ts";

/** The package declaring the features a test entitles accounts to, by resource too. */
export const ENTITLED_PACKAGE = PackageId.parse("package-019f5530-8000-7000-8000-0000000000f9");

/** Accounts' entitlements as a test writes them into a follower's replica. */
export const TestEntitlement = {
    /** A publisher that sends nothing until stopped. */
    publisher: {
        confirm: async () => {},
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
        terms: Pick<Entitlement, "kind"> & Partial<Pick<Entitlement, "value" | "limit" | "state">>,
        environment: Environment,
    ): Promise<void> {
        // mark the account's replica complete once
        const now = environment.clock.now();
        const key = {
            name: entitlementShape.replicaName({
                scope: Scope.universe.id,
                account: accountId,
            }),
            scope: Scope.universe.id,
        };
        await database.transaction(
            async (transaction) =>
                await transaction
                    .insert(replica)
                    .values({ ...key, tables: [] })
                    .onConflictDoNothing(),
        );
        await database.transaction(
            async (transaction) =>
                await transaction
                    .insert(replicaPosition)
                    .values({ ...key, log: "test", epoch: "test", sequence: 1, confirmedAt: now })
                    .onConflictDoNothing(),
        );

        // replace the entitlement, a resource's under a test feature of its name
        const feature =
            typeof granted === "string"
                ? { packageId: ENTITLED_PACKAGE, name: granted, resource: granted }
                : { ...granted.feature, resource: null };
        await database.transaction(
            async (transaction) =>
                await transaction
                    .delete(entitlement.table)
                    .where(
                        and(
                            eq(entitlement.table.scope, accountId),
                            eq(entitlement.table.packageId, feature.packageId),
                            eq(entitlement.table.feature, feature.name),
                        ),
                    ),
        );
        await database.transaction(
            async (transaction) =>
                await transaction.insert(entitlement.table).values({
                    id: Identifier.create("entitlement", environment),
                    scope: accountId,
                    packageId: feature.packageId,
                    feature: feature.name,
                    resource: feature.resource,
                    ...terms,
                    createdAt: now,
                    updatedAt: now,
                }),
        );
    },
};
