import { expect, onTestFinished, test } from "@destack/test";
import { Snapshot, Transaction } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { principal } from "../declare/principal.ts";
import { AccessFixture } from "../test/access.ts";
import { account, fixtureDatabase, mappings, policies, space } from "../test/fixture.ts";
import { Authorizer } from "./authorizer.ts";
import { Scope } from "@destack/sync";

test.for(TEST_DIALECTS)(
    "fence a scope for a transfer, moving it and the scopes below it until unfenced, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, fixtureDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        const database = storage.database;
        const authorizer = new Authorizer(policies, mappings);

        // insert an account and a space inside it
        const access = new AccessFixture(database);
        await access.replicateScope(account.reference(Scope.universe.id, "account-a"));
        await access.replicateScope(space.reference("account-a", "space-a"), "own");
        const context = {
            subjects: [principal.user.reference("universe", "alice")],
            now: 1000,
            attributes: {},
        };
        const moved = async () =>
            await Promise.all(
                ["account-a", "space-a"].map(
                    async (scope) =>
                        (await authorizer.resolve(Snapshot.live(database), scope, context)).moved,
                ),
            );

        // move the account and the space below it once fenced, and neither once unfenced
        await Transaction.run(database, (transaction) =>
            Scope.fence(transaction, "account-a", "machine-b", 2000),
        );
        expect(await moved()).toEqual([
            { scope: "account-a", machine: "machine-b" },
            { scope: "account-a", machine: "machine-b" },
        ]);
        await Transaction.run(database, (transaction) => Scope.unfence(transaction, "account-a"));
        expect(await moved()).toEqual([undefined, undefined]);

        // refuse fencing a scope the database does not keep
        await expect(
            Transaction.run(database, (transaction) =>
                Scope.fence(transaction, "account-z", "machine-b", 2000),
            ),
        ).rejects.toMatchObject({
            code: "NOT_FOUND",
            message: "unknown scope: account-z",
        });
    },
);
