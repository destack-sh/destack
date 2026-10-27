import { and, type DatabaseConnection, eq } from "@destack/db";
import { PackageId } from "@destack/package";
import { type FeatureReference, QueriesParameters, Shape } from "@destack/object";
import { Identifier, schema } from "@destack/schema";
import {
    type Publisher,
    Replica,
    replica,
    replicaPosition,
    replicaSubscription,
    Scope,
    type Subscription,
} from "@destack/sync";
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
        request: async () => {},
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
        // mark the replica of the universe's entitlements complete, following the account's key beside the earlier ones
        const now = environment.clock.now();
        const scope = Scope.universe.id;
        const name = Shape.replicaName(scope);
        const followed = Shape.subscription(
            scope,
            [{ shape: entitlementShape, keys: [{ account: accountId }] }],
            scope,
        );
        const recorded = await Replica.subscription(database, name, scope);
        const queries = {
            ...QueriesParameters.parse(recorded?.parameters ?? {}).queries,
            ...QueriesParameters.parse(followed.parameters).queries,
        };
        const columns = Replica.subscriptionColumns(
            { ...followed, parameters: { queries: schema.json().parse(queries) } },
            "caller",
        );
        await database.transaction(async (transaction) => {
            await transaction
                .insert(replica)
                .values({ name, scope, tables: [], living: {} })
                .onConflictDoNothing();
            await transaction
                .insert(replicaSubscription)
                .values({ name, scope, ...columns })
                .onConflictDoUpdate({
                    target: [replicaSubscription.name, replicaSubscription.scope],
                    set: columns,
                });
            await transaction
                .insert(replicaPosition)
                .values({ name, scope, log: "test", epoch: "test", sequence: 1 })
                .onConflictDoNothing();
        });

        // hear the replica's source, as its follow would
        database.tracker.track(Replica.owner(name, scope));

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
