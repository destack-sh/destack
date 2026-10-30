import { expect, onTestFinished, refusal, test } from "@destack/test";
import { Scope, type Subject } from "@destack/sync";
import { Transaction, Snapshot, and, asc, eq, ne } from "@destack/db";
import { PackageId } from "@destack/package";
import { present, schema } from "@destack/schema";
import {
    AccessContext,
    accessRelationship,
    accessRole,
    accessRolePermission,
    Authorization,
    Authorizer,
    Caller,
    Invitation,
    Manager,
    principal,
    Relationship,
    type RelationshipRequest,
    role,
    Role,
} from "../index.ts";
import {
    account,
    accountTable,
    cell,
    group,
    homeMappings,
    item,
    mappings,
    node,
    policies,
    space,
    spaceTable,
    team,
    teamTable,
} from "../test/fixture.ts";
import { openFixture } from "../test/database.ts";
import { TestAccess } from "../test/access.ts";

/** The members of carol's account as a subject set. */
const carolMembership = { ...account.reference("universe", "account-1"), relation: "member" };

/** A member of an account who has roles through the membership. */
const carol: AccessContext = {
    grants: [],
    subject: principal.user.reference("universe", "carol"),
    groups: [carolMembership],
    now: 1000,
    attributes: {},
    actors: [],
    contacts: [],
    linkSecrets: [],
};

/** Open the fixture with role grants covering ancestors. */
async function openRoleFixture() {
    const fixture = await openFixture();
    onTestFinished(() => fixture.close());
    const { database } = fixture;
    const query = new Authorizer(
        policies,
        mappings.map((mapping) =>
            mapping.policy === node ? { ...mapping, parent: "parent" } : mapping,
        ),
    );

    // select readable nodes
    const readable = async (context: AccessContext) => {
        const ids = (
            await database
                .select({ id: item.id })
                .from(item)
                .where(
                    query.where(
                        node.permission("read"),
                        await query.resolve(Snapshot.live(database), "personal", context),
                    ),
                )
                .orderBy(asc(item.id))
        ).map((row) => row.id);

        return ids;
    };

    // define roles granting permissions of the fixture's types
    const defineRole = async (
        number: number,
        permissions: { packageId: PackageId; type: string; name: string; approvals?: number }[],
        scope = "personal",
    ) => {
        const id = schema
            .identifier("role")
            .parse(`role-01996ab0-0000-7000-8000-00000000010${number}`);
        await database.transaction(
            async (transaction) =>
                await transaction.insert(accessRole).values({
                    id,
                    createdAt: 1,
                    updatedAt: 1,
                    scope,
                    name: `role-${number}`,
                    description: "A test role",
                }),
        );
        for (const [position, permission] of permissions.entries()) {
            await database.transaction(
                async (transaction) =>
                    await transaction.insert(accessRolePermission).values({
                        id: schema
                            .identifier("role-permission")
                            .parse(
                                `role-permission-01996ab0-0000-7000-8000-0000000002${number}${position}`,
                            ),
                        roleId: id,
                        scope,
                        ...permission,
                    }),
            );
        }

        return id;
    };

    // relate objects and subjects directly, as declarations and replication do
    let next = 0;
    const relate = async (relationship: RelationshipRequest) => {
        const id = `relationship-01996ab0-0000-7000-8000-0000000003${String(next++).padStart(2, "0")}`;
        await Relationship.insert(database, {
            ...relationship,
            id,
            createdAt: 1,
            expiresAt: relationship.expiresAt ?? null,
        });

        return id;
    };

    return { ...fixture, query, readable, defineRole, relate };
}

test("bind roles on an ancestor, covering its descendants until the binding expires", async () => {
    const { database, readable, defineRole, relate } = await openRoleFixture();
    expect(await readable(carol)).toEqual([]);

    // bind a role reading the subtree under "a" to carol's membership
    const reader = await defineRole(1, [node.permission("read")]);
    await relate({
        object: node.reference("personal", "a"),
        role: reader,
        subject: carolMembership,
        expiresAt: 2000,
    });
    expect(await readable(carol)).toEqual(["a", "b", "c"]);
    expect(await readable({ ...carol, now: 2000 })).toEqual([]);

    // grant nothing once the binding is deleted
    await database.transaction(async (transaction) => await transaction.delete(accessRelationship));
    expect(await readable(carol)).toEqual([]);
});

test("decide a permission of another type on a node from a role bound on or above it, and refuse roles granting it elsewhere or no longer", async () => {
    const { database, query, defineRole, relate } = await openRoleFixture();
    const decide = async (target: string, context: AccessContext = carol) => {
        const snapshot = Snapshot.live(database);
        const access = await query.resolve(snapshot, "personal", context);
        const decision = await query.check(
            snapshot,
            cell.permission("read"),
            node.reference("personal", target),
            access,
        );

        return decision.isAllowed;
    };

    // bind a role reading cells on "a" until 2000, and a role reading only nodes on "d"
    const cells = await defineRole(1, [cell.permission("read")]);
    const nodes = await defineRole(2, [node.permission("read")]);
    await relate({
        object: node.reference("personal", "a"),
        role: cells,
        subject: carolMembership,
        expiresAt: 2000,
    });
    await relate({
        object: node.reference("personal", "d"),
        role: nodes,
        subject: carolMembership,
    });

    // allow the cells' read on "a" and below it while bound, never on "d" or after the expiry
    expect({
        bound: [await decide("a"), await decide("b"), await decide("c")],
        elsewhere: await decide("d"),
        expired: await decide("b", { ...carol, now: 2000 }),
    }).toEqual({ bound: [true, true, true], elsewhere: false, expired: false });
});

test("give a permission a role grants on approval only to a call whose change gathered as many approvals from others holding it, and answer how many it needs", async () => {
    const { database, query, defineRole, relate } = await openRoleFixture();
    const reading = node.permission("read");
    const target = node.reference("personal", "b");
    const erin = principal.user.reference("universe", "erin");
    const frank = principal.user.reference("universe", "frank");
    const gina = principal.user.reference("universe", "gina");
    const decide = async (approvers: readonly Subject[]) => {
        const snapshot = Snapshot.live(database);
        const access = await query.resolve(snapshot, "personal", { ...carol, approvers });

        return (await query.check(snapshot, reading, target, access)).isAllowed;
    };

    // bind a role reading the subtree under "a" once two approvals gathered, and let erin and frank read everywhere
    const reviewed = await defineRole(1, [{ ...reading, approvals: 2 }]);
    await relate({
        object: node.reference("personal", "a"),
        role: reviewed,
        subject: carolMembership,
    });
    const reader = await defineRole(2, [reading], Scope.universe.id);
    for (const subject of [erin, frank]) {
        await relate({ object: Scope.universe, role: reader, subject });
    }

    // refuse the read at once, with one approval, with carol's own or gina's, who cannot read, as the second; allow it with two readers
    const snapshot = Snapshot.live(database);
    const access = await query.resolve(snapshot, "personal", carol);
    const needed = await query.requiredApprovals(
        snapshot,
        reading,
        access,
        async (approved) => (await query.check(snapshot, reading, target, approved)).isAllowed,
    );
    expect({
        direct: await decide([]),
        one: await decide([erin]),
        own: await decide([erin, carol.subject]),
        unqualified: await decide([erin, gina]),
        two: await decide([erin, frank]),
        needed,
    }).toEqual({ direct: false, one: false, own: false, unqualified: false, two: true, needed: 2 });
});

test("find a scope's role by name, and read an object's relationships through a relation, a role or one subject", async () => {
    // bind a reader role on note a to carol's membership, and let dave view it
    const { database, defineRole, relate } = await openRoleFixture();
    const reader = await defineRole(1, [node.permission("read")]);
    const object = node.reference("personal", "a");
    const dave = principal.user.reference("universe", "dave");
    const bound = await relate({ object, role: reader, subject: carolMembership });
    const viewing = await relate({ object, relation: "viewer", subject: dave });

    // read each selection of the note's relationships
    const snapshot = Snapshot.live(database);
    const read = async (match: Parameters<typeof Relationship.readOn>[2]) =>
        (await Relationship.readOn(snapshot, object, match)).map((row) => row.id).toSorted();
    expect({
        all: await read({}),
        bound: await read({ role: reader }),
        viewing: await read({ relation: "viewer" }),
        held: await read({ subject: dave }),
        unheld: await read({ relation: "viewer", subject: carolMembership }),
    }).toEqual({
        all: [bound, viewing],
        bound: [bound],
        viewing: [viewing],
        held: [viewing],
        unheld: [],
    });
});

test("ignore roles defined outside the object's scope chain", async () => {
    const { readable, defineRole, relate } = await openRoleFixture();

    // bind a reader role another scope defines on a note
    const foreign = await defineRole(4, [node.permission("read")], "elsewhere");
    await relate({
        object: node.reference("personal", "a"),
        role: foreign,
        subject: carolMembership,
    });
    expect(await readable(carol)).toEqual([]);
});

test("bind roles on enclosing scopes and compose roles through includes", async () => {
    const { database, readable, defineRole, relate, environment } = await openRoleFixture();

    // place the personal scope inside the account of its members
    const access = new TestAccess(database, environment, Scope.universe.id);
    await access.replicateScope(account.reference("universe", "account-1"), account);
    await access.bindOwner(account.reference("universe", "account-1"), carolMembership, 1);
    await access.replicateScope(space.reference("account-1", "personal"), space);

    // bind an editor role including the reader role on the account
    const reader = await defineRole(1, [node.permission("read")], "account-1");
    const editor = await defineRole(2, [node.permission("edit")], "account-1");
    await relate({
        object: role.reference("account-1", editor),
        relation: "includes",
        subject: role.reference("account-1", reader),
    });
    await relate({
        object: account.reference("universe", "account-1"),
        role: editor,
        subject: carolMembership,
    });
    expect(await readable(carol)).toEqual(["a", "b", "c"]);

    // refuse deleting the included reader role, even for the account's owners
    const home = new Authorizer(policies, homeMappings, { partition: Scope.universe.id });
    const object = account.reference("universe", "account-1");
    await database.insert(accountTable).values({ id: "account-1", scope: "universe" });
    expect(
        await refusal(
            Authorization.run(
                home,
                database,
                () => carol,
                environment,
                (authorization) => Role.delete(object, reader, 1, authorization),
            ),
        ),
    ).toEqual(["CONFLICT", "ROLE_IN_USE", "role is still bound or included"]);
});

test("bind roles to groups, nested groups and verified subject sets", async () => {
    const { readable, defineRole, relate } = await openRoleFixture();

    // bind a reader role on "b" to the members of the engineering group
    const reader = await defineRole(1, [node.permission("read")]);
    const engineering = { ...group.reference("account-1", "engineering"), relation: "member" };
    await relate({
        object: node.reference("personal", "b"),
        role: reader,
        subject: engineering,
    });
    expect(await readable(carol)).toEqual([]);

    // accept a subject set the authenticating authority verified
    const verified: AccessContext = {
        ...carol,
        groups: [
            ...carol.groups,
            { ...group.reference("account-1", "engineering"), relation: "member" },
        ],
    };
    expect(await readable(verified)).toEqual(["b", "c"]);

    // expand membership through a nested group
    await relate({
        object: group.reference("account-1", "engineering"),
        relation: "member",
        subject: { ...group.reference("account-1", "platform"), relation: "member" },
    });
    await relate({
        object: group.reference("account-1", "platform"),
        relation: "member",
        subject: carolMembership,
    });
    expect(await readable(carol)).toEqual(["b", "c"]);
});

test("relate the members of a set in a field", async () => {
    const { database, readable, relate } = await openRoleFixture();

    // relate the members of a team to "c" and store carol as its member in a column
    await relate({
        object: node.reference("personal", "c"),
        relation: "viewer",
        subject: { ...team.reference("personal", "team-1"), relation: "member" },
    });
    expect(await readable(carol)).toEqual([]);
    await database.insert(teamTable).values({ id: "team-1", scope: "personal", member: "carol" });
    expect(await readable(carol)).toEqual(["c"]);
});

test("decide anew once a transaction adds the caller to a group or to a set's field", async () => {
    const { database, query, defineRole, relate, environment } = await openRoleFixture();

    // bind a reader on "a" to a group's members, and share "c" with a team's
    const reader = await defineRole(1, [node.permission("read")]);
    const engineering = group.reference("account-1", "engineering");
    await relate({
        object: node.reference("personal", "a"),
        role: reader,
        subject: { ...engineering, relation: "member" },
    });
    await relate({
        object: node.reference("personal", "c"),
        relation: "viewer",
        subject: { ...team.reference("personal", "team-1"), relation: "member" },
    });

    // decide carol's reads before and after joining each set in one transaction
    const decisions = await Authorization.run(
        query,
        database,
        () => carol,
        environment,
        async (authorization) => {
            const read = async (target: string) =>
                (
                    await authorization.check(
                        node.permission("read"),
                        node.reference("personal", target),
                    )
                ).isAllowed;
            const before = [await read("a"), await read("c")];
            await Relationship.create(
                { object: engineering, relation: "member", subject: carol.subject },
                engineering.scope,
                authorization,
            );
            const joined = await read("a");
            await authorization.transaction.write({
                table: teamTable,
                before: null,
                after: { id: "team-1", scope: "personal", member: "carol" },
            });

            return [...before, joined, await read("c")];
        },
    );
    expect(decisions).toEqual([false, false, true, true]);
});

test("bind only roles whose permissions the granting caller has at their approvals", async () => {
    const { database, query, alice, defineRole, environment } = await openRoleFixture();

    // refuse binding a role that does not exist
    expect(
        await refusal(
            Authorization.run(
                query,
                database,
                () => alice,
                environment,
                (authorization) =>
                    Relationship.grant(
                        {
                            object: node.reference("personal", "a"),
                            role: "role-01996ab0-0000-7000-8000-000000000999",
                            subject: carolMembership,
                        },
                        authorization,
                    ),
            ),
        ),
    ).toEqual(["NOT_FOUND", "ROLE_NOT_FOUND", "role not found"]);

    // let the owner bind a role reading her note
    const reader = await defineRole(1, [node.permission("read")]);
    await Authorization.run(
        query,
        database,
        () => alice,
        environment,
        (authorization) =>
            Relationship.grant(
                {
                    object: node.reference("personal", "a"),
                    role: reader,
                    subject: carolMembership,
                },
                authorization,
            ),
    );

    // reject binding a role granting a permission she does not have there
    const escalating = await defineRole(2, [node.permission("read"), cell.permission("edit")]);
    expect(
        await refusal(
            Authorization.run(
                query,
                database,
                () => alice,
                environment,
                (authorization) =>
                    Relationship.grant(
                        {
                            object: node.reference("personal", "a"),
                            role: escalating,
                            subject: carolMembership,
                        },
                        authorization,
                    ),
            ),
        ),
    ).toEqual(["FORBIDDEN", "PERMISSION_DENIED", "permission denied: edit"]);

    // reject binding a role granting that permission on approvals too
    const gated = await defineRole(3, [{ ...cell.permission("edit"), approvals: 2 }]);
    expect(
        await refusal(
            Authorization.run(
                query,
                database,
                () => alice,
                environment,
                (authorization) =>
                    Relationship.grant(
                        {
                            object: node.reference("personal", "a"),
                            role: gated,
                            subject: carolMembership,
                        },
                        authorization,
                    ),
            ),
        ),
    ).toEqual(["FORBIDDEN", "PERMISSION_DENIED", "permission denied: edit"]);

    // refuse binding a role to a subject set that resolving never expands
    expect(
        await refusal(
            Authorization.run(
                query,
                database,
                () => alice,
                environment,
                (authorization) =>
                    Relationship.grant(
                        {
                            object: node.reference("personal", "a"),
                            role: reader,
                            subject: { ...node.reference("personal", "b"), relation: "viewer" },
                        },
                        authorization,
                    ),
            ),
        ),
    ).toEqual([
        "FORBIDDEN",
        "ROLE_SUBJECT_REFUSED",
        "a role cannot bind to node#viewer: roles bind to subject sets and scope relations",
    ]);
});

test("let owners have everything in their scope, make owners, and never remove the last one", async () => {
    const { database, readable, alice, environment } = await openRoleFixture();

    // create the personal space in the universe, carol as owner: she reads every note in it
    const universe = new Authorizer(policies, homeMappings, { partition: Scope.universe.id });
    const home = new Authorizer(policies, homeMappings, { partition: "personal" });
    const place = space.reference("universe", "personal");
    await database.insert(spaceTable).values({ id: "personal", account: "universe" });
    await Authorization.run(
        universe,
        database,
        () => carol,
        environment,
        (authorization) =>
            authorization.create(place, {
                owner: carol.subject,
            }),
    );
    expect(await readable(carol)).toEqual(["a", "b", "c"]);

    // let an owner make another owner, and refuse anyone else
    const owner = { id: Role.OWNER };
    expect(
        await refusal(
            Authorization.run(
                universe,
                database,
                () => alice,
                environment,
                (authorization) =>
                    Relationship.grant(
                        {
                            object: place,
                            role: owner.id,
                            subject: principal.user.reference("universe", "bob"),
                        },
                        authorization,
                    ),
            ),
        ),
    ).toEqual(["FORBIDDEN", "PERMISSION_DENIED", "permission denied: share"]);
    const second = await Authorization.run(
        universe,
        database,
        () => carol,
        environment,
        (authorization) =>
            Relationship.grant(
                {
                    object: place,
                    role: owner.id,
                    subject: alice.subject,
                },
                authorization,
            ),
    );

    // refuse a sharer of the space's nodes who is not an owner binding the role granting everything on one
    const sharer = await Authorization.run(
        home,
        database,
        () => carol,
        environment,
        (authorization) =>
            Role.create(
                place,
                {
                    name: "sharer",
                    description: "Shares the space's nodes",
                    permissions: [node.permission("share")],
                },
                authorization,
            ),
    );
    const dave = AccessContext.of(Caller.of(principal.user.reference("universe", "dave")), 1000);
    await Authorization.run(
        home,
        database,
        () => carol,
        environment,
        (authorization) =>
            Relationship.grant(
                {
                    object: place,
                    role: sharer.id,
                    subject: dave.subject,
                },
                authorization,
            ),
    );
    expect(
        await refusal(
            Authorization.run(
                home,
                database,
                () => dave,
                environment,
                (authorization) =>
                    Relationship.grant(
                        {
                            object: node.reference("personal", "a"),
                            role: owner.id,
                            subject: principal.user.reference("universe", "bob"),
                        },
                        authorization,
                    ),
            ),
        ),
    ).toEqual(["FORBIDDEN", "OWNER_REQUIRED", "only owners may bind a role granting everything"]);

    // leave at least one owner
    const [first] = await database
        .select({ id: accessRelationship.id })
        .from(accessRelationship)
        .where(
            and(
                eq(accessRelationship.roleId, owner.id),
                ne(accessRelationship.id, schema.identifier("relationship").parse(second.id)),
            ),
        );
    if (first === undefined) {
        throw new Error("the space has no first owner");
    }
    await Authorization.run(
        universe,
        database,
        () => carol,
        environment,
        (authorization) => Relationship.revoke(place, second.id, authorization),
    );
    expect(
        await refusal(
            Authorization.run(
                universe,
                database,
                () => carol,
                environment,
                (authorization) => Relationship.revoke(place, first.id, authorization),
            ),
        ),
    ).toEqual([
        "CONFLICT",
        "LAST_RELATIONSHIP",
        "the last owner of space personal cannot be removed",
    ]);
});

test("create a scope without separate owners, whose account's owners are its owners", async () => {
    const { database, environment } = await openRoleFixture();

    // create carol's account and a space in it without a separate owner
    const universe = new Authorizer(policies, homeMappings, { partition: Scope.universe.id });
    const home = new Authorizer(policies, homeMappings, { partition: "shared" });
    const asCarol = <Value>(
        authorizer: Authorizer,
        call: (authorization: Authorization) => Promise<Value>,
    ) => Authorization.run(authorizer, database, () => carol, environment, call);
    const created = account.reference("universe", "account-1");
    const place = space.reference("account-1", "shared");
    await database.insert(accountTable).values({ id: "account-1", scope: "universe" });
    await database.insert(spaceTable).values({ id: "shared", account: "account-1" });
    await asCarol(universe, (authorization) =>
        authorization.create(created, { owner: carol.subject }),
    );
    await asCarol(universe, (authorization) => authorization.create(place, {}));

    // let carol have everything in the space through her account
    const access = await home.resolve(Snapshot.live(database), "shared", carol);
    const decision = await home.check(
        Snapshot.live(database),
        space.permission("update"),
        place,
        access,
    );
    expect(
        await refusal(
            asCarol(home, (authorization) =>
                authorization.create(node.reference("shared", "a"), {
                    owner: carol.subject,
                }),
            ),
        ),
    ).toEqual(["BAD_REQUEST", "SCOPE_REQUIRED", "only a scope can have owners"]);
    expect(decision.isAllowed).toBe(true);
});

test("list the spaces an account contains to the account's owner", async () => {
    const { database, alice, environment } = await openRoleFixture();
    const query = new Authorizer(policies, [
        ...mappings,
        {
            policy: space,
            table: spaceTable,
            id: "id",
            scope: "account",
            attributes: {},
            relations: {},
        },
    ]);

    // record two accounts, their spaces, and carol as the first account's owner
    const access = new TestAccess(database, environment, Scope.universe.id);
    await access.replicateScope(account.reference("universe", "account-1"), account);
    await access.bindOwner(account.reference("universe", "account-1"), carol.subject, 1);
    await access.replicateScope(account.reference("universe", "account-2"), account);
    for (const [id, owner] of [
        ["space-1", "account-1"],
        ["space-2", "account-1"],
        ["space-3", "account-2"],
    ] as const) {
        await new TestAccess(database, environment).replicateScope(
            space.reference(owner, id),
            space,
        );
        await database.insert(spaceTable).values({ id, account: owner });
    }

    // list by container: the owner sees the account's spaces, and a stranger sees none
    const listed = async (context: AccessContext) =>
        (
            await database
                .select({ id: spaceTable.id })
                .from(spaceTable)
                .where(
                    query.where(
                        space.permission("read"),
                        await query.resolve(Snapshot.live(database), "account-1", context),
                    ),
                )
                .orderBy(asc(spaceTable.id))
        ).map((row) => row.id);
    expect(await listed(carol)).toEqual(["space-1", "space-2"]);
    expect(await listed(alice)).toEqual([]);

    // list the same spaces from the scope records of every database
    await new TestAccess(database, environment).replicateScope(
        account.reference("account-1", "account-3"),
        account,
    );
    const scopes = new Authorizer(policies, mappings);
    const recorded = await database
        .select({ id: Scope.table.scope })
        .from(Scope.table)
        .where(
            scopes.where(
                space.permission("read"),
                await scopes.resolve(Snapshot.live(database), "account-1", carol),
            ),
        )
        .orderBy(asc(Scope.table.scope));
    expect(recorded.map((row) => row.id)).toEqual(["space-1", "space-2"]);
});

test("refuse an offer whose inviting principal lost the authority to grant it", async () => {
    const { database, query, defineRole, relate, environment } = await openRoleFixture();
    const bob = principal.user.reference("universe", "bob");
    const dave = AccessContext.of(Caller.of(principal.user.reference("universe", "dave")), 1000);

    // let bob share note a through a role, and offer dave a view of it
    const sharer = await defineRole(5, [node.permission("share"), node.permission("read")]);
    const binding = await relate({
        object: node.reference("personal", "a"),
        role: sharer,
        subject: bob,
    });
    const offer = await Authorization.run(
        query,
        database,
        () => AccessContext.of(Caller.of(bob), 1000),
        environment,
        (authorization) =>
            Invitation.create(
                {
                    relationship: {
                        object: node.reference("personal", "a"),
                        relation: "viewer",
                        subject: dave.subject,
                    },
                },
                authorization,
            ),
    );

    // refuse acceptance once bob no longer has the role
    await database.transaction(
        async (transaction) =>
            await transaction
                .delete(accessRelationship)
                .where(eq(accessRelationship.id, schema.identifier("relationship").parse(binding))),
    );
    expect(
        await refusal(
            Authorization.run(
                query,
                database,
                () => dave,
                environment,
                (authorization) =>
                    Invitation.accept(offer.relationship.object, offer.id, authorization),
            ),
        ),
    ).toEqual(["FORBIDDEN", "PERMISSION_DENIED", "permission denied: share"]);
});

/** Open the role fixture with carol as owner of the personal space, created in the universe. */
async function openSpaceWithOwner() {
    const fixture = await openRoleFixture();
    const universe = new Authorizer(policies, homeMappings, { partition: Scope.universe.id });
    const home = new Authorizer(policies, homeMappings, { partition: "personal" });
    const place = space.reference("universe", "personal");
    await fixture.database.insert(spaceTable).values({ id: "personal", account: "universe" });
    await Authorization.run(
        universe,
        fixture.database,
        () => carol,
        fixture.environment,
        (authorization) =>
            authorization.create(place, {
                owner: carol.subject,
            }),
    );

    // authorize carol at a time
    const owner = <Value>(now: number, call: (authorization: Authorization) => Promise<Value>) =>
        Authorization.run(
            home,
            fixture.database,
            () => ({ ...carol, now }),
            fixture.environment,
            call,
        );

    return { ...fixture, universe, home, place, owner };
}

test("upsert a role by name, replace its purpose and permissions, then upsert it identically without a write", async () => {
    const { database, place, owner } = await openSpaceWithOwner();
    const read = { packageId: node.definition.packageId, type: node.name, name: "read" };
    const update = { ...read, name: "update" };

    // upsert the role, again under the same name with other permissions, and once more alike
    const created = await owner(1000, (authorization) =>
        Role.upsert(
            place,
            {
                name: "editor",
                description: "Read nodes",
                permissions: [read],
            },
            authorization,
        ),
    );
    const changed = { name: "editor", description: "Update nodes", permissions: [update] };
    await owner(2000, (authorization) => Role.upsert(place, changed, authorization));
    const upserted = await owner(3000, (authorization) =>
        Role.upsert(place, changed, authorization),
    );

    // expect one role at its second revision, last written at the change, granting only the changed permissions
    const reading = await Transaction.open(database);
    const record = await Role.read(reading, "personal", upserted.id);
    const expected = {
        ...created,
        description: "Update nodes",
        permissions: [update],
        revision: 2,
    };
    expect([
        upserted,
        Role.describe(record, await Role.permissions(reading, upserted.id)),
        record.updatedAt,
    ]).toEqual([expected, expected, 2000]);
});

test("grant a space's own role bound on the space inside it alone, never on the space's row", async () => {
    const { database, home, place, owner, bob, readable } = await openSpaceWithOwner();

    // define a role in the space reading its nodes and the space, and bind it on the space to bob
    const reader = await owner(1000, (authorization) =>
        Role.create(
            place,
            {
                name: "reader",
                description: "Read the space and its nodes",
                permissions: [node.permission("read"), space.permission("read")],
            },
            authorization,
        ),
    );
    await owner(1000, (authorization) =>
        Relationship.grant({ object: place, role: reader.id, subject: bob.subject }, authorization),
    );

    // decide the space's row for bob and its owner, in memory and in SQL
    const snapshot = Snapshot.live(database);
    const decide = async (context: AccessContext) => {
        const access = await home.resolve(snapshot, "personal", context);
        const listed = await database
            .select({ id: spaceTable.id })
            .from(spaceTable)
            .where(home.where(space.permission("read"), access));

        return [
            (await home.check(snapshot, space.permission("read"), place, access)).isAllowed,
            listed.map((row) => row.id),
        ];
    };

    // store the binding inside the space, granting bob its nodes but not the space itself
    const [binding] = await database
        .select({ scope: accessRelationship.scope })
        .from(accessRelationship)
        .where(eq(accessRelationship.roleId, reader.id));
    expect({
        binding,
        nodes: await readable(bob),
        bob: await decide(bob),
        carol: await decide(carol),
    }).toEqual({
        binding: { scope: "personal" },
        nodes: ["a", "b", "c"],
        bob: [false, []],
        carol: [true, ["personal"]],
    });
});

test("upsert a declaration's role as its manager, refuse it to others until released, then let others change it and refuse the declaration", async () => {
    const { database, place, owner } = await openSpaceWithOwner();
    const read = { packageId: node.definition.packageId, type: node.name, name: "read" };
    const manager = Manager.schema.parse({
        installationId: "installation-01996ab0-0000-7000-8000-000000000001",
        packageId: "package-01996ab0-0000-7000-8000-000000000002",
        name: "roles/editor",
    });
    const request = { name: "editor", description: "Read nodes", permissions: [read] };
    const declaring = (description: string) =>
        refusal(
            owner(1000, (authorization) =>
                Role.upsert(place, { ...request, description }, authorization.applying(manager)),
            ),
        );
    const direct = (description: string) =>
        refusal(
            owner(1000, (authorization) =>
                Role.upsert(place, { ...request, description }, authorization),
            ),
        );

    // upsert the role as its declaration, and refuse carol changing it directly
    const declared = await declaring("Read nodes");
    const refused = await direct("Edit nodes");

    // release it as its stack does, then let carol change it and refuse the declaration taking it back
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(accessRole)
                .set({ detachedAt: 1 })
                .where(eq(accessRole.name, "editor")),
    );
    const changed = await direct("Edit nodes");
    const retaken = await declaring("Read nodes");

    expect({ declared, refused, changed, retaken }).toEqual({
        declared: "done",
        refused: [
            "CONFLICT",
            "MANAGED",
            "role is managed by its declaration: detach it before changing it",
        ],
        changed: "done",
        retaken: ["CONFLICT", "ROLE_NAME_TAKEN", "role name is already in use"],
    });
});

test("replace an inherent role's permissions with the declared ones, keeping the rows it grants already, and drop it once it grants none", async () => {
    const { database, place, owner } = await openSpaceWithOwner();
    const read = { packageId: node.definition.packageId, type: node.name, name: "read" };
    const update = { ...read, name: "update" };
    const remove = { ...read, name: "delete" };
    const dave = principal.user.reference("universe", "dave");
    const declare = (permissions: (typeof read)[], now: number) =>
        owner(now, (authorization) =>
            Role.applyInherent(
                place,
                { name: "maintainer", description: "Maintain nodes", permissions },
                dave,
                authorization,
            ),
        );
    const rows = async (id: string) =>
        database
            .select({ id: accessRolePermission.id, name: accessRolePermission.name })
            .from(accessRolePermission)
            .where(eq(accessRolePermission.roleId, schema.identifier("role").parse(id)))
            .orderBy(asc(accessRolePermission.name));

    // declare the role, then declare it with one permission swapped, then with none
    await declare([read, update], 1000);
    const [maintainer] = await database
        .select({ id: accessRole.id })
        .from(accessRole)
        .where(eq(accessRole.name, "maintainer"));
    const declared = present(maintainer, "the maintainer role").id;
    const first = await rows(declared);
    await declare([update, remove], 2000);
    const second = await rows(declared);
    await declare([], 3000);
    const dropped = {
        roles: await database
            .select({ id: accessRole.id })
            .from(accessRole)
            .where(eq(accessRole.id, declared)),
        permissions: await rows(declared),
        bindings: await database
            .select({ id: accessRelationship.id })
            .from(accessRelationship)
            .where(eq(accessRelationship.roleId, declared)),
    };

    // leave the update permission's row and replace the read one with a new delete row
    const [readRow, updateRow] = first;
    const [added] = second;
    const ids = [...first, ...second].map((row) => row.id);
    expect({ first, second, distinct: new Set(ids).size, dropped }).toEqual({
        first: [
            { id: readRow?.id, name: "read" },
            { id: updateRow?.id, name: "update" },
        ],
        second: [{ id: added?.id, name: "delete" }, updateRow],
        distinct: 3,
        // drop the role with its permissions and binding once it grants nothing
        dropped: { roles: [], permissions: [], bindings: [] },
    });
});

test("assign a role to one subject alone, revoking the previous subject and leaving an identical binding", async () => {
    const { database, place, owner } = await openSpaceWithOwner();
    const reader = await owner(1000, (authorization) =>
        Role.upsert(
            place,
            {
                name: "reader",
                description: "Read nodes",
                permissions: [],
            },
            authorization,
        ),
    );

    // assign the role to carol, to dave and to dave again
    const dave = principal.user.reference("universe", "dave");
    await owner(1000, (authorization) =>
        Role.assign(place, reader.id, [carol.subject], authorization),
    );
    const [assigned] = await owner(2000, (authorization) =>
        Role.assign(place, reader.id, [dave], authorization),
    );
    const again = await owner(3000, (authorization) =>
        Role.assign(place, reader.id, [dave], authorization),
    );

    // expect only dave's first binding on the scope's object
    const rows = await database
        .select()
        .from(accessRelationship)
        .where(eq(accessRelationship.roleId, schema.identifier("role").parse(reader.id)));
    const binding = {
        id: present(assigned, "dave's binding").id,
        object: place,
        role: reader.id,
        subject: dave,
        createdAt: 2000,
        expiresAt: null,
    };
    expect([again, rows.map((row) => Relationship.decode(row))]).toEqual([[binding], [binding]]);
});

test("refuse upserting and assigning roles to a caller without the permissions", async () => {
    const { place, home, database, alice, owner, environment } = await openSpaceWithOwner();
    const reader = await owner(1000, (authorization) =>
        Role.upsert(
            place,
            {
                name: "reader",
                description: "Read the space",
                permissions: [space.permission("read")],
            },
            authorization,
        ),
    );

    // refuse alice defining, changing and assigning a role in carol's space
    const outsider = <Value>(call: (authorization: Authorization) => Promise<Value>) =>
        Authorization.run(home, database, () => alice, environment, call);
    expect(
        await refusal(
            outsider((authorization) =>
                Role.upsert(
                    place,
                    {
                        name: "sharer",
                        description: "Share",
                        permissions: [],
                    },
                    authorization,
                ),
            ),
        ),
    ).toEqual(["FORBIDDEN", "PERMISSION_DENIED", "permission denied: create"]);
    expect(
        await refusal(
            outsider((authorization) =>
                Role.upsert(
                    place,
                    {
                        name: "reader",
                        description: "Read",
                        permissions: [],
                    },
                    authorization,
                ),
            ),
        ),
    ).toEqual(["FORBIDDEN", "PERMISSION_DENIED", "permission denied: update"]);
    expect(
        await refusal(
            outsider((authorization) =>
                Role.assign(place, reader.id, [alice.subject], authorization),
            ),
        ),
    ).toEqual(["FORBIDDEN", "PERMISSION_DENIED", "permission denied: share"]);
});

test("bind a scope's owner at the home of the scope keeping the binding, and refuse it in a database homing another partition", async () => {
    const { universe, database, environment, place, owner } = await openSpaceWithOwner();
    const dave = principal.user.reference("universe", "dave");
    const bind = (authorizer: Authorizer) =>
        Authorization.run(
            authorizer,
            database,
            () => carol,
            environment,
            (authorization) => Role.bind(place, Role.OWNER, dave, authorization),
        );

    // bind dave as an owner where the universe homes the binding, and refuse it in the space's database
    expect([
        await refusal(
            owner(0, (authorization) => Role.bind(place, Role.OWNER, dave, authorization)),
        ),
        await bind(universe),
    ]).toEqual([
        [
            "FORBIDDEN",
            "ACCESS_NOT_LOCAL",
            "cannot write the access of space personal here: it lives at the home of universe",
        ],
        undefined,
    ]);
});

test("define a role giving a permission on approvals only at as many as the definer needs, apart from the change its call applies", async () => {
    const { database, home, place, bob, defineRole, relate, environment } =
        await openSpaceWithOwner();

    // let bob define roles anywhere in the universe, and read nodes once two others approved
    const reviewed = await defineRole(
        3,
        [role.permission("create"), { ...node.permission("read"), approvals: 2 }],
        Scope.universe.id,
    );
    await relate({
        object: Scope.universe,
        role: reviewed,
        subject: bob.subject,
    });

    // define roles reading nodes as bob, at two or three approvals, at none, and at none through a twice approved change
    const erin = principal.user.reference("universe", "erin");
    const frank = principal.user.reference("universe", "frank");
    const define = (name: string, approvals: number, approvers: readonly Subject[] = []) =>
        Authorization.run(
            home,
            database,
            () => ({ ...bob, approvers }),
            environment,
            async (authorization) =>
                (
                    await Role.create(
                        place,
                        {
                            name,
                            description: "Reads nodes",
                            permissions: [{ ...node.permission("read"), approvals }],
                        },
                        authorization,
                    )
                ).name,
        );
    expect([
        await define("two", 2),
        await define("three", 3),
        await refusal(define("none", 0)),
        await refusal(define("approved", 0, [erin, frank])),
        await refusal(
            Authorization.run(
                home,
                database,
                () => bob,
                environment,
                (authorization) =>
                    Role.create(
                        place,
                        {
                            name: "twice",
                            description: "Reads nodes twice",
                            permissions: [
                                { ...node.permission("read"), approvals: 2 },
                                { ...node.permission("read"), approvals: 3 },
                            ],
                        },
                        authorization,
                    ),
            ),
        ),
    ]).toEqual([
        "two",
        "three",
        ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: read"],
        ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: read"],
        ["BAD_REQUEST", "PERMISSION_REPEATED", "a role grants node.read once"],
    ]);
});
