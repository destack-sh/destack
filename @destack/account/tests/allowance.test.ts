import { defineDatabase } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { replica, replicaPosition } from "@destack/sync";
import { expect, onTestFinished, test } from "@destack/test";
import { Shape } from "@destack/object";
import { account, allowance, Allowance, allowanceShape, space } from "../src/object/index.ts";

/** The account whose allowances a machine checks. */
const ACCOUNT = account.identifier("account-019f5530-8000-7000-8000-000000000501");

/** The home space storing the account's allowances. */
const HOME = space.identifier("space-019f5530-8000-7000-8000-000000000502");

test.for(TEST_DIALECTS)(
    "decide a use from an account's replicated allowances: pending until replicated, blocked at its limit, unentitled without a grant, and by tenancy on %s",
    async (dialect) => {
        // open a machine's replicas of the account's allowances
        const storage = await TestDatabase.create(
            dialect,
            defineDatabase({ name: "machine", tables: [...allowance.tables, replica] }),
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
                ).map(async (use) => (await Allowance.check(database, ACCOUNT, use)).reason),
            );

        // decide before the replica completes
        const pending = await decide();

        // replicate blocked database and network allowances, storage within its limit, and the shared tenancy
        const now = Date.now();
        const key = {
            name: Shape.replicaName(allowanceShape, { scope: HOME, account: ACCOUNT }),
            scope: HOME,
        };
        await database.insert(replica).values({ ...key, tables: [] });
        await database
            .insert(replicaPosition)
            .values({ ...key, epoch: "epoch", sequence: 1, confirmedAt: now });
        const rows = [
            {
                id: "allowance-019f5530-8000-7000-8000-000000000511",
                resource: "database" as const,
                grant: {
                    kind: "metered" as const,
                    limit: 1000,
                    used: 1200,
                    state: "blocked" as const,
                },
            },
            {
                id: "allowance-019f5530-8000-7000-8000-000000000513",
                resource: "storage" as const,
                grant: {
                    kind: "metered" as const,
                    limit: 1000,
                    used: 10,
                    state: "within" as const,
                },
            },
            {
                id: "allowance-019f5530-8000-7000-8000-000000000514",
                resource: "network" as const,
                grant: {
                    kind: "metered" as const,
                    limit: 1000,
                    used: 1000,
                    state: "blocked" as const,
                },
            },
            {
                id: "allowance-019f5530-8000-7000-8000-000000000512",
                resource: "tenancy" as const,
                grant: { kind: "listed" as const, values: ["shared", "own"] },
            },
        ];
        for (const row of rows) {
            await database.insert(allowance.table).values({
                ...row,
                id: allowance.identifier(row.id),
                scope: HOME,
                accountId: ACCOUNT,
                createdAt: now,
                updatedAt: now,
            });
        }

        // decide each use from the replicated allowances
        expect({ pending, replicated: await decide() }).toEqual({
            pending: ["Pending", "Pending", "Pending", "Pending", "Pending", "Pending"],
            replicated: ["Blocked", "Within", "Unentitled", "Blocked", "Within", "Unentitled"],
        });
    },
);
