import { type Environment } from "@destack/environment";
import type { DatabaseConnection } from "@destack/db/database";
import { QueriesParameters, QUERIES_PUBLICATION, Shape } from "@destack/object";
import { Identifier, schema } from "@destack/schema";
import {
    ObjectReference,
    ReplicaSet,
    replica,
    replicaPosition,
    replicaSubscription,
} from "@destack/sync";
import type { FeatureGrant } from "../feature/grant.ts";
import { price } from "../object/price.ts";
import { seller } from "../object/seller.ts";
import { subscription, subscriptionItem } from "../object/subscription.ts";

/** The billing period of a subscription a test grants, a month in milliseconds. */
const PERIOD_MILLISECONDS = 30 * 24 * 60 * 60 * 1000;

/** The terms of the free monthly price a test's subscription item bills. */
const FREE_TERMS = {
    currency: "USD",
    unitAmount: 0,
    unitAmountDecimal: null,
    includedUsage: 0,
    recurring: { interval: "month", intervalCount: 1, usage: "licensed" },
    billingScheme: "per_unit",
    tiers: null,
} as const;

/** Accounts' plans as a test writes them into a follower's replica, standing in for its follow. */
export const TestPlan = {
    /** Mark a follower's replica of a shape's key at a scope complete, beside its earlier ones. */
    async follow(
        database: DatabaseConnection,
        followed: { readonly shape: Shape; readonly key: object; readonly scope: string },
    ): Promise<void> {
        // record the shape's key among the replica's queries, as the follow's subscription would
        const { scope } = followed;
        const name = Shape.replicaName(QUERIES_PUBLICATION, scope);
        const subscribed = Shape.subscription(
            scope,
            [{ shape: followed.shape, keys: [followed.key] }],
            { target: scope },
        );
        const recorded = await new ReplicaSet(database).subscription(name, scope);
        const queries = {
            ...QueriesParameters.parse(recorded?.parameters ?? {}).queries,
            ...QueriesParameters.parse(subscribed.parameters).queries,
        };
        const columns = ReplicaSet.subscriptionColumns(
            { ...subscribed, parameters: { queries: schema.json().parse(queries) } },
            "caller",
        );
        await database.transaction(async (transaction) => {
            await transaction
                .insert(replica)
                .values({ name, scope, tables: [] })
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
                .onConflictDoUpdate({
                    target: [replicaPosition.name, replicaPosition.scope],
                    set: { log: "test", epoch: "test", sequence: 1 },
                });
        });

        // mark the replica live, as its follow would
        new ReplicaSet(database).mark({ name, scope }, { log: "test", epoch: "test", sequence: 1 });
    },

    /** Grant an account a feature by an active subscription of a home, in a follower's replica. */
    async grant(
        database: DatabaseConnection,
        accountId: Identifier<"account">,
        granted: FeatureGrant,
        followed: { readonly shape: Shape; readonly key: object; readonly scope: string },
        home: Identifier<"space">,
        environment: Environment,
    ): Promise<void> {
        // mark the replica complete
        await TestPlan.follow(database, followed);

        // write an active subscription of the home with one item granting the feature, as the replica would
        const now = environment.clock.now();
        const scope = home;
        const subscriptionId = Identifier.create("subscription", environment);
        await database.transaction((transaction) =>
            transaction.log.asReplica(async () => {
                await transaction.insert(subscription.table).values({
                    id: subscriptionId,
                    scope,
                    accountId,
                    ...ObjectReference.columns(
                        "seller",
                        seller.reference(scope, Identifier.create("seller", environment)),
                    ),
                    status: "active",
                    currentPeriodStart: now,
                    currentPeriodEnd: now + PERIOD_MILLISECONDS,
                    providerId: subscriptionId,
                    createdAt: now,
                    updatedAt: now,
                });
                await transaction.insert(subscriptionItem.table).values({
                    id: Identifier.create("subscription-item", environment),
                    scope,
                    parentId: subscriptionId,
                    ...ObjectReference.columns(
                        "price",
                        price.reference(scope, Identifier.create("price", environment)),
                    ),
                    terms: FREE_TERMS,
                    grants: [granted],
                    createdAt: now,
                    updatedAt: now,
                });
            }),
        );
    },
};
