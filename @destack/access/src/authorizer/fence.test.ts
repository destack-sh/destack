import { testSimulation } from "@destack/test/world";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { Snapshot, Transaction } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { principal } from "../declare/principal.ts";
import { TestAccess } from "../test/access.ts";
import { account, fixtureDatabase, mappings, policies, space } from "../test/fixture.ts";
import { Authorizer } from "./authorizer.ts";
import { AccessContext, Caller } from "../context/context.ts";
import { Scope } from "@destack/sync";

test("fence a scope for a transfer, moving it and the scopes below it until unfenced", async () => {
    await using simulation = testSimulation();
    const environment = simulation.environment("access");
    const storage = await TestDatabase.create(fixtureDatabase, {
        environment: environment,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    const authorizer = new Authorizer(policies, mappings);

    // insert an account and a space inside it
    const access = new TestAccess(database, environment);
    await access.replicateScope(account.reference(Scope.universe.id, "account-a"), account);
    await access.replicateScope(space.reference("account-a", "space-a"), space);
    const context = AccessContext.of(
        Caller.of(principal.user.reference("universe", "alice")),
        1000,
    );
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

    // refuse fencing a scope the database does not store
    expect(
        await refusal(
            Transaction.run(database, (transaction) =>
                Scope.fence(transaction, "account-z", "machine-b", 2000),
            ),
        ),
    ).toEqual(["NOT_FOUND", "NOT_FOUND", "unknown scope: account-z"]);
});
