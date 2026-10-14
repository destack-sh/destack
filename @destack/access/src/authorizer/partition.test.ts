import { expect, onTestFinished, test } from "@destack/test";
import { and, eq, Snapshot } from "@destack/db";
import { present } from "@destack/schema";
import { Scope } from "@destack/sync";
import { AccessError, accessRole, Authorization, Authorizer } from "../index.ts";
import {
    account,
    accountTable,
    homeMappings,
    policies,
    space,
    spaceTable,
} from "../test/fixture.ts";
import { openFixture, userSubject } from "../test/database.ts";
import { AccessFixture } from "../test/access.ts";

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

test("keep a space's own access with its row in the universe, refusing it in the space's partition, which replicates it", async () => {
    // keep only the universe's partition, with an account and a space created in it
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
    await universe.database.insert(spaceTable).values({ id: "personal", account: "account-1" });
    const personal = space.reference("account-1", "personal");
    await owner((authorization) => authorization.create(personal, { owner: userSubject("alice") }));

    // make bob an owner of the space where its row lives
    const [role] = await universe.database
        .select({ id: accessRole.id })
        .from(accessRole)
        .where(and(eq(accessRole.scope, "personal"), eq(accessRole.name, "owner")));
    const roleId = present(role, "the space's owner role").id;
    const assigned = await owner((authorization) =>
        authorization.assign(personal, roleId, userSubject("bob")),
    );

    // refuse the same in the space's partition, with its replica of the chain
    const partition = await openFixture();
    onTestFinished(() => partition.close());
    const access = new AccessFixture(partition.database);
    await access.replicateScope(account.reference("universe", "account-1"));
    await access.replicateScope(personal, "own");
    const replicatedRole = await access.replicateOwner(personal, userSubject("alice"));
    const refused = Authorization.run(
        new Authorizer(policies, homeMappings, { partition: "personal" }),
        partition.database,
        () => ({ ...partition.alice, now: Date.now() }),
        (authorization) => authorization.assign(personal, replicatedRole, userSubject("bob")),
    );

    // keep bob's role at the universe and refuse writing the replica
    expect(assigned.subject).toEqual(userSubject("bob"));
    await expect(refused).rejects.toMatchObject({
        code: "FORBIDDEN",
        message: "cannot write the access of space personal here: this database has a replica",
    });
});

test("open a space in its own partition below its replicated account, refusing a scope whose access this database keeps", async () => {
    // keep an account in the universe, and replica it into the space's partition
    const universe = await openFixture();
    onTestFinished(() => universe.close());
    const partition = await openFixture();
    onTestFinished(() => partition.close());
    await universe.database.insert(accountTable).values({ id: "account-1", scope: "universe" });
    await Authorization.run(
        new Authorizer(policies, homeMappings, { partition: Scope.universe.id }),
        universe.database,
        () => ({ ...universe.alice, now: Date.now() }),
        (authorization) =>
            authorization.create(account.reference("universe", "account-1"), {
                owner: userSubject("alice"),
            }),
    );
    await new AccessFixture(partition.database).replicateScope(
        account.reference("universe", "account-1"),
    );

    // attach the space in its partition, and refuse attaching it where its account is kept
    const personal = space.reference("account-1", "personal");
    const attach = (database: typeof partition.database, at: string) =>
        Authorization.run(
            new Authorizer(policies, homeMappings, { partition: at }),
            database,
            () => ({ ...partition.alice, now: Date.now() }),
            (authorization) => authorization.attach(personal, ["account-1"]),
        );
    await attach(partition.database, "personal");
    const refused = await attach(universe.database, Scope.universe.id).catch(
        (error: unknown) => error,
    );

    // record the space in its own partition below the account, with no access of its own
    const chain = await Scope.chain(Snapshot.live(partition.database), "personal");
    const owners = await partition.database
        .select({ id: accessRole.id })
        .from(accessRole)
        .where(eq(accessRole.scope, "personal"));
    expect({
        chain: chain.map((link) => [link.object.id, link.partition]),
        owners,
        refused,
    }).toEqual({
        chain: [
            ["personal", "personal"],
            ["account-1", "universe"],
            ["universe", "universe"],
        ],
        owners: [],
        refused: new AccessError(
            "INVALID_CONTEXT",
            "this database keeps the access of space personal: receive it",
        ),
    });
});
