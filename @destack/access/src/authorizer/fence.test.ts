import { expect, onTestFinished, test } from "@destack/test";
import { Snapshot } from "@destack/db";
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
        await copyScope(database, account.reference(Scope.universe.id, "account-a"));
        await copyScope(database, space.reference("account-a", "space-a"));
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
        await Scope.fence(database, "account-a", "host-b", 2000);
        expect(await moved()).toEqual([
            { scope: "account-a", cell: "host-b" },
            { scope: "account-a", cell: "host-b" },
        ]);
        await Scope.unfence(database, "account-a");
        expect(await moved()).toEqual([undefined, undefined]);

        // refuse fencing a scope the database does not keep
        await expect(Scope.fence(database, "account-z", "host-b", 2000)).rejects.toMatchObject({
            code: "NOT_FOUND",
            message: "unknown scope: account-z",
        });
    },
);

test.skipIf(!TEST_DIALECTS.includes("postgresql"))(
    "fence a scope only once the writes guarding it commit on postgresql",
    async () => {
        const storage = await TestDatabase.create("postgresql", fixtureDatabase, {
            isMigrated: true,
        });
        const other = await storage.connect(fixtureDatabase);
        onTestFinished(async () => {
            await other.close();
            await storage.close();
        });
        await copyScope(storage.database, account.reference(Scope.universe.id, "account-a"));

        // keep a write guarding the scope open while a fence starts on another connection
        const events: string[] = [];
        const guarded = Promise.withResolvers<void>();
        const release = Promise.withResolvers<void>();
        const writing = storage.database.transaction(async (transaction) => {
            await Scope.guard(transaction, "account-a");
            guarded.resolve();
            await release.promise;
            events.push("committed");
        });
        await guarded.promise;
        const fencing = Scope.fence(other, "account-a", "host-b", 2000).then(() =>
            events.push("fenced"),
        );

        // fence after the guarding write commits, even when it commits later
        await new Promise((resolve) => {
            setTimeout(resolve, 50);
        });
        release.resolve();
        await Promise.all([writing, fencing]);
        expect(events).toEqual(["committed", "fenced"]);
    },
);
