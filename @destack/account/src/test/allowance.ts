import { and, type DatabaseConnection, eq } from "@destack/db";
import { Identifier } from "@destack/schema";
import { type Publisher, replica, replicaPosition, type Subscription } from "@destack/sync";
import { until } from "@destack/runtime/timer";
import { Shape } from "@destack/object";
import {
    allowance,
    type AllowanceGrant,
    type AllowanceResource,
    allowanceShape,
} from "../object/allowance.ts";
import { space } from "../object/space.ts";

/** The home space keeping the allowances a test writes. */
const TEST_HOME = space.identifier("space-019f5530-8000-7000-8000-0000000000f8");

/** Accounts' allowances as a test writes them into a follower's replica. */
export const TestAllowance = {
    /** A publisher that sends nothing, as the test writes the replicas itself. */
    publisher: {
        stream: async function* (_subscription: Subscription, signal: AbortSignal) {
            await until(signal);
            yield* [];
        },
    } satisfies Publisher,

    /** Apply an allowance on a resource to an account as its replica would. */
    async set(
        database: DatabaseConnection,
        accountId: Identifier<"account">,
        resource: AllowanceResource,
        grant: AllowanceGrant,
    ): Promise<void> {
        // mark the account's replica complete once
        const now = Date.now();
        const key = {
            name: Shape.replicaName(allowanceShape, { scope: TEST_HOME, account: accountId }),
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

        // replace the allowance on the resource
        await database
            .delete(allowance.table)
            .where(
                and(
                    eq(allowance.table.accountId, accountId),
                    eq(allowance.table.resource, resource),
                ),
            );
        await database.insert(allowance.table).values({
            id: Identifier.create("allowance"),
            scope: TEST_HOME,
            accountId,
            resource,
            grant,
            createdAt: now,
            updatedAt: now,
        });
    },
};
