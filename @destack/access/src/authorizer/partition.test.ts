import { expect, onTestFinished, test } from "@destack/test";
import { Snapshot } from "@destack/db";
import { Scope } from "@destack/sync";
import { Authorization, Authorizer } from "../index.ts";
import {
    account,
    accountTable,
    homeMappings,
    policies,
    space,
    spaceTable,
} from "../test/fixture.ts";
import { openFixture, userSubject } from "../test/database.ts";

test("record each scope in its own partition where its type partitions, else in its parent's", async () => {
    // create an account in the universe and a partitioning space in it
    const home = await openFixture();
    onTestFinished(() => home.close());
    const owner = <Value>(call: (authorization: Authorization) => Promise<Value>) =>
        Authorization.run(
            new Authorizer(policies, homeMappings),
            home.database,
            () => ({
                ...home.alice,
                now: Date.now(),
            }),
            call,
        );
    await home.database.insert(accountTable).values({ id: "account-1", scope: "universe" });
    await owner((authorization) =>
        authorization.create(account.reference("universe", "account-1"), {
            owner: userSubject("alice"),
        }),
    );
    await home.database.insert(spaceTable).values({ id: "personal", account: "account-1" });
    await owner((authorization) =>
        authorization.create(space.reference("account-1", "personal"), {
            owner: userSubject("alice"),
        }),
    );

    // read the space in its own partition and the account in the universe's
    const chain = await Scope.chain(Snapshot.live(home.database), "personal");
    expect(chain.map((link) => [link.object.id, link.partition])).toEqual([
        ["personal", "personal"],
        ["account-1", "universe"],
        ["universe", "universe"],
    ]);
});

test("record a space's birth in the universe's partition, then refuse writing its access there, which its own partition keeps", async () => {
    // keep the universe's partition alone, with an account in it
    const universe = await openFixture();
    onTestFinished(() => universe.close());
    const owner = <Value>(call: (authorization: Authorization) => Promise<Value>) =>
        Authorization.run(
            new Authorizer(policies, homeMappings, { partition: Scope.universe.id }),
            universe.database,
            () => ({ ...universe.alice, now: Date.now() }),
            call,
        );
    await universe.database.insert(accountTable).values({ id: "account-1", scope: "universe" });
    await owner((authorization) =>
        authorization.create(account.reference("universe", "account-1"), {
            owner: userSubject("alice"),
        }),
    );

    // record the space's birth, then grant in the account and in the space
    await universe.database.insert(spaceTable).values({ id: "personal", account: "account-1" });
    const personal = space.reference("account-1", "personal");
    await owner((authorization) => authorization.create(personal, { owner: userSubject("alice") }));
    const grant = (object: typeof personal) =>
        owner((authorization) =>
            authorization.grant({ object, relation: "member", subject: userSubject("bob") }),
        );
    await grant(account.reference("universe", "account-1"));

    await expect(grant(personal)).rejects.toMatchObject({
        code: "FORBIDDEN",
        message: "access of space personal is written in the database keeping it",
    });
});
