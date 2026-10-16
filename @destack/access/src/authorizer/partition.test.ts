import { expect, onTestFinished, test } from "@destack/test";
import { and, asc, type DatabaseConnection, eq, Snapshot } from "@destack/db";
import { Scope } from "@destack/sync";
import {
    AccessError,
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

test("store a space's own access in its partition, its home, refusing it where its row was created, which removes only the space's row", async () => {
    // open the universe's partition, with an account and a space created in it
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

    // bear the space in its partition below its replicated account, with alice its owner
    const partition = await openFixture();
    onTestFinished(() => partition.close());
    await new AccessFixture(partition.database).replicateScope(
        account.reference("universe", "account-1"),
    );
    await partition.database.insert(spaceTable).values({ id: "personal", account: "account-1" });
    const home = <Value>(call: (authorization: Authorization) => Promise<Value>) =>
        Authorization.run(
            new Authorizer(policies, homeMappings, { partition: "personal" }),
            partition.database,
            () => ({ ...partition.alice, now: Date.now() }),
            call,
        );
    await home((authorization) =>
        authorization.receive(personal, ["account-1"], { owner: userSubject("alice") }),
    );

    // make bob an owner at the space's home, and refuse the same where its row was created
    const granted = await home((authorization) =>
        Relationship.grant(
            { object: personal, role: Role.OWNER, subject: userSubject("bob") },
            authorization,
        ),
    );

    // record the space in its own partition below the account, its owners bound there alone
    const chain = await Scope.chain(Snapshot.live(partition.database), "personal");
    expect({
        granted: granted.subject,
        chain: chain.map((link) => [link.object.id, link.partition]),
        home: await owners(partition.database),
        universe: await owners(universe.database),
    }).toEqual({
        granted: userSubject("bob"),
        chain: [
            ["personal", "personal"],
            ["account-1", "universe"],
            ["universe", "universe"],
        ],
        home: ["alice", "bob"],
        universe: [],
    });
    await expect(
        owner((authorization) =>
            Relationship.grant(
                {
                    object: personal,
                    role: Role.OWNER,
                    subject: userSubject("bob"),
                },
                authorization,
            ),
        ),
    ).rejects.toEqual(
        new AccessError(
            "FORBIDDEN",
            "cannot write the access of space personal here: it lives at its home",
        ),
    );

    // remove the space where its row was created: its scope row goes, its own access stays at its home
    await owner((authorization) => authorization.remove(personal));
    expect({
        universe: await Scope.read(Snapshot.live(universe.database), ["personal"]),
        home: await owners(partition.database),
    }).toEqual({ universe: [], home: ["alice", "bob"] });
});

test("forget a removed scope's relationships, roles, invitations and row, and reuse its identifier", async () => {
    // store an account owned by alice with a role bound to bob and an invitation for carol
    const universe = await openFixture();
    onTestFinished(() => universe.close());
    const owner = <Value>(call: (authorization: Authorization) => Promise<Value>) =>
        Authorization.run(
            new Authorizer(policies, homeMappings, { partition: Scope.universe.id }),
            universe.database,
            () => ({ ...universe.alice, now: Date.now() }),
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

/** Read the access rows of account-1: its relationships, roles, invitations and scope row. */
async function stored(database: DatabaseConnection) {
    const relationships = await database
        .select({ roleId: accessRelationship.roleId, subjectId: accessRelationship.subjectId })
        .from(accessRelationship)
        .where(eq(accessRelationship.scope, "account-1"));
    const roles = await database
        .select({ id: accessRole.id })
        .from(accessRole)
        .where(eq(accessRole.scope, "account-1"));
    const invitations = await database
        .select({ id: accessInvitation.id })
        .from(accessInvitation)
        .where(eq(accessInvitation.scope, "account-1"));
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
