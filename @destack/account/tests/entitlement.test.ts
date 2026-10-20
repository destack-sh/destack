import { defineDatabase } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { replica, replicaPosition, Scope } from "@destack/sync";
import { expect, onTestFinished, test } from "@destack/test";
import { Identifier } from "@destack/schema";
import { account, entitlement, Entitlement, entitlementShape } from "../src/object/index.ts";
import { PackageId } from "@destack/package";

/** The account whose entitlements a machine checks. */
const ACCOUNT = account.identifier("account-019f5530-8000-7000-8000-000000000501");

/** The package declaring the platform's features. */
const PRICING = PackageId.parse("package-019f5530-8000-7000-8000-000000000503");

/** A metered entitlement to a resource with a limit of 1000, blocked at it or within it. */
function metered(resource: "database" | "storage" | "network", state: "blocked" | "within") {
    return { resource, kind: "metered" as const, limit: 1000, state };
}

test.for(TEST_DIALECTS)(
    "decide a use from an account's replicated entitlements: pending until replicated, blocked at its limit, unentitled without a grant, and by tenancy on %s",
    async (dialect) => {
        // open a machine's replicas of the account's entitlements
        const storage = await TestDatabase.create(
            dialect,
            defineDatabase({ name: "machine", tables: [...entitlement.tables, replica] }),
            { isMigrated: true },
        );
        onTestFinished(() => storage.close());
        const { database } = storage;
        const decide = async () =>
            Promise.all(
                (
                    [
                        "database",
                        "storage",
                        "compute",
                        "network",
                        { tenancy: "shared" },
                        { tenancy: "dedicated" },
                    ] as const
                ).map(async (use) => (await Entitlement.check(database, ACCOUNT, use)).reason),
            );

        // decide before the replica completes
        const pending = await decide();

        // replicate blocked database and network entitlements, storage within its limit, and the shared tenancy
        const now = Date.now();
        const key = {
            name: entitlementShape.replicaName({
                scope: Scope.universe.id,
                account: ACCOUNT,
            }),
            scope: Scope.universe.id,
        };
        await database.insert(replica).values({ ...key, tables: [] });
        await database
            .insert(replicaPosition)
            .values({ ...key, log: "log", epoch: "epoch", sequence: 1, confirmedAt: now });
        const rows = [
            metered("database", "blocked"),
            metered("storage", "within"),
            metered("network", "blocked"),
            { resource: "tenancy" as const, kind: "static" as const, value: ["shared", "own"] },
        ];
        for (const row of rows) {
            await database.insert(entitlement.table).values({
                ...row,
                id: Identifier.create("entitlement"),
                scope: ACCOUNT,
                packageId: PRICING,
                feature: row.resource,
                createdAt: now,
                updatedAt: now,
            });
        }

        // decide each use from the replicated entitlements
        expect({ pending, replicated: await decide() }).toEqual({
            pending: ["Pending", "Pending", "Pending", "Pending", "Pending", "Pending"],
            replicated: ["Blocked", "Within", "Unentitled", "Blocked", "Within", "Unentitled"],
        });
    },
);
