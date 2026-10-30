import { TEST_START } from "@destack/test/world";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { and, asc, type DatabaseConnection, eq, Snapshot } from "@destack/db";
import { Scope } from "@destack/sync";
import {
    accessInvitation,
    accessRelationship,
    accessRole,
    Authorization,
    Authorizer,
    Role,
    Invitation,
    Relationship,
} from "../index.ts";
import {
    account,
    accountTable,
    homeMappings,
    policies,
    space,
    spaceTable,
} from "../test/fixture.ts";
import { openFixture, userSubject } from "../test/database.ts";
import { TestAccess } from "../test/access.ts";

test("record each scope in its partition where its type partitions, else in its parent's", async () => {
    // create an account in the universe and a partitioning space in it
    const home = await openFixture();
    onTestFinished(() => home.close());
    const owner = <Value>(call: (authorization: Authorization) => Promise<Value>) =>
        Authorization.run(
            new Authorizer(policies, homeMappings, { partition: Scope.universe.id }),
            home.database,
            () => ({
                ...home.alice,
                now: TEST_START,
            }),
            home.environment,
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

    // read the space in its partition and the account in the universe's
    const chain = await Scope.chain(Snapshot.live(home.database), "personal");
    expect(chain.map((link) => [link.object.id, link.partition])).toEqual([
        ["personal", "personal"],
        ["account-1", "universe"],
        ["universe", "universe"],
    ]);
});

test("store a space's own access beside its row in its account's home, refusing it at the space's home, which records the space's scope alone", async () => {
    // open the universe's partition, with an account and a space alice owns created in it
    const universe = await openFixture();
    onTestFinished(() => universe.close());
    const owner = <Value>(call: (authorization: Authorization) => Promise<Value>) =>
        Authorization.run(
            new Authorizer(policies, homeMappings, { partition: Scope.universe.id }),
            universe.database,
            () => ({ ...universe.alice, now: TEST_START }),
            universe.environment,
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

    // bear the space in its partition below its replicated account
    const partition = await openFixture();
    onTestFinished(() => partition.close());
    await new TestAccess(partition.database, partition.environment).replicateScope(
        account.reference("universe", "account-1"),
        account,
    );
    await partition.database.insert(spaceTable).values({ id: "personal", account: "account-1" });
    const home = <Value>(call: (authorization: Authorization) => Promise<Value>) =>
        Authorization.run(
            new Authorizer(policies, homeMappings, { partition: "personal" }),
            partition.database,
            () => ({ ...partition.alice, now: TEST_START }),
            partition.environment,
            call,
        );
    await home((authorization) => authorization.receive(personal, ["account-1"]));

    // make bob an owner beside the space's row, and refuse the same at its home
    const ownBob = { object: personal, role: Role.OWNER, subject: userSubject("bob") };
    const granted = await owner((authorization) => Relationship.grant(ownBob, authorization));
    const refused = await refusal(
        home((authorization) => Relationship.grant(ownBob, authorization)),
    );

    // record the space in its partition below the account, its owners bound beside its row alone
    const chain = await Scope.chain(Snapshot.live(partition.database), "personal");
    expect({
        granted: granted.subject,
        refused,
        chain: chain.map((link) => [link.object.id, link.partition]),
        home: await owners(partition.database),
        universe: await owners(universe.database),
    }).toEqual({
        granted: userSubject("bob"),
        refused: [
            "FORBIDDEN",
            "ACCESS_NOT_LOCAL",
            "cannot write the access of space personal here: it lives at the home of account-1",
        ],
        chain: [
            ["personal", "personal"],
            ["account-1", "universe"],
            ["universe", "universe"],
        ],
        home: [],
        universe: ["alice", "bob"],
    });

    // remove the space beside its row: its scope row and its own access go, its home keeps its scope
    await owner((authorization) => authorization.remove(personal));
    expect({
        universe: await Scope.read(Snapshot.live(universe.database), ["personal"]),
        owners: await owners(universe.database),
        home: (await Scope.read(Snapshot.live(partition.database), ["personal"])).length,
    }).toEqual({ universe: [], owners: [], home: 1 });
});

test("forget a removed scope's relationships, roles, invitations and row, and reuse its identifier", async () => {
    // store an account of alice with a role bound to bob and an invitation for carol
    const universe = await openFixture();
    onTestFinished(() => universe.close());
    const owner = <Value>(call: (authorization: Authorization) => Promise<Value>) =>
        Authorization.run(
            new Authorizer(policies, homeMappings, { partition: Scope.universe.id }),
            universe.database,
            () => ({ ...universe.alice, now: TEST_START }),
            universe.environment,
            call,
        );
    const object = account.reference("universe", "account-1");
    await universe.database.insert(accountTable).values({ id: "account-1", scope: "universe" });
    await owner((authorization) => authorization.create(object, { owner: userSubject("alice") }));
    const reader = await owner((authorization) =>
        Role.create(
            object,
            { name: "reader", description: "Reads", permissions: [] },
            authorization,
        ),
    );
    await owner((authorization) =>
        Relationship.grant({ object, role: reader.id, subject: userSubject("bob") }, authorization),
    );
    await owner((authorization) =>
        Invitation.create(
            {
                relationship: { object, relation: "member", subject: userSubject("carol") },
            },
            authorization,
        ),
    );

    // remove the account's access, then create it again under the same identifier
    await owner((authorization) => authorization.remove(object));
    const removed = await stored(universe.database);
    await owner((authorization) => authorization.create(object, { owner: userSubject("alice") }));
    expect({ removed, recreated: await stored(universe.database) }).toEqual({
        removed: { relationships: [], roles: [], invitations: [], scopes: [] },
        recreated: {
            relationships: [[Role.OWNER, "alice"]],
            roles: [],
            invitations: [],
            scopes: ["account-1"],
        },
    });
});

/** Read account-1's access rows: the relationships and invitations on it, its roles and scope. */
async function stored(database: DatabaseConnection) {
    const relationships = await database
        .select({ roleId: accessRelationship.roleId, subjectId: accessRelationship.subjectId })
        .from(accessRelationship)
        .where(eq(accessRelationship.objectId, "account-1"));
    const roles = await database
        .select({ id: accessRole.id })
        .from(accessRole)
        .where(eq(accessRole.scope, "account-1"));
    const invitations = await database
        .select({ id: accessInvitation.id })
        .from(accessInvitation)
        .where(eq(accessInvitation.objectId, "account-1"));
    const scopes = await database
        .select({ scope: Scope.table.scope })
        .from(Scope.table)
        .where(eq(Scope.table.scope, "account-1"));

    return {
        relationships: relationships.map((row) => [row.roleId, row.subjectId]),
        roles: roles.map((row) => row.id),
        invitations: invitations.map((row) => row.id),
        scopes: scopes.map((row) => row.scope),
    };
}

/** List the subjects bound to the owner role on the personal space, in order. */
async function owners(database: DatabaseConnection): Promise<string[]> {
    const rows = await database
        .select({ subjectId: accessRelationship.subjectId })
        .from(accessRelationship)
        .where(
            and(
                eq(accessRelationship.objectId, "personal"),
                eq(accessRelationship.roleId, Role.OWNER),
            ),
        )
        .orderBy(asc(accessRelationship.subjectId));

    return rows.map((row) => row.subjectId);
}
