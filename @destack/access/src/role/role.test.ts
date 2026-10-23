import { expect, onTestFinished, refusal, test } from "@destack/test";
import { Scope } from "@destack/sync";
import { Transaction, Snapshot, and, asc, eq, ne } from "@destack/db";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import {
    accessRelationship,
    accessRole,
    accessRolePermission,
    Authorization,
    Authorizer,
    principal,
    role,
    Role,
    type AccessContext,
    Relationship,
    type RelationshipRequest,
    Invitation,
    Manager,
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
import { AccessFixture } from "../test/access.ts";

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
        permissions: { packageId: PackageId; type: string; name: string }[],
        scope = "personal",
    ) => {
        const id = schema
            .identifier("role")
            .parse(`role-01996ab0-0000-7000-8000-00000000010${number}`);
        await database.insert(accessRole).values({
            id,
            createdAt: 1,
            updatedAt: 1,
            scope,
            name: `role-${number}`,
            description: "A test role",
        });
        for (const [position, permission] of permissions.entries()) {
            await database.insert(accessRolePermission).values({
                id: schema
                    .identifier("role-permission")
                    .parse(
                        `role-permission-01996ab0-0000-7000-8000-0000000002${number}${position}`,
                    ),
                roleId: id,
                scope,
                ...permission,
            });
        }

        return id;
    };

    // relate objects and subjects directly, as declarations and replication do
    let next = 0;
    const relate = async (relationship: RelationshipRequest) => {
        const id = `relationship-01996ab0-0000-7000-8000-0000000003${String(next++).padStart(2, "0")}`;
        await Relationship.insert(
            database,
            { ...relationship, id, createdAt: 1, expiresAt: relationship.expiresAt ?? null },
            query.governingScope(relationship.object),
        );

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
    await database.delete(accessRelationship);
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
    const { database, readable, defineRole, relate } = await openRoleFixture();

    // place the personal scope inside the account of its members
    const access = new AccessFixture(database);
    await access.replicateScope(account.reference("universe", "account-1"));
    await access.bindOwner(account.reference("universe", "account-1"), carolMembership, 1);
    await access.replicateScope(space.reference("account-1", "personal"), "self");

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
    const home = new Authorizer(policies, homeMappings);
    const object = account.reference("universe", "account-1");
    await database.insert(accountTable).values({ id: "account-1", scope: "universe" });
    await expect(
        Authorization.run(
            home,
            database,
            () => carol,
            (authorization) => Role.delete(object, reader, 1, authorization),
        ),
    ).rejects.toThrow("role is still bound or included");
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
    const { database, query, defineRole, relate } = await openRoleFixture();

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

test("bind only roles whose permissions the granting caller has", async () => {
    const { database, query, alice, defineRole } = await openRoleFixture();

    // refuse binding a role that does not exist
    expect(
        await refusal(
            Authorization.run(
                query,
                database,
                () => alice,
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

    // refuse binding a role to a subject set that resolving never expands
    expect(
        await refusal(
            Authorization.run(
                query,
                database,
                () => alice,
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
    const { database, readable, alice } = await openRoleFixture();

    // create the personal space in its home database, carol as owner: she reads every note in it
    const home = new Authorizer(policies, homeMappings);
    const place = space.reference("universe", "personal");
    await database.insert(spaceTable).values({ id: "personal", account: "universe" });
    await Authorization.run(
        home,
        database,
        () => carol,
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
                home,
                database,
                () => alice,
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
        home,
        database,
        () => carol,
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

    // refuse a sharer who is not an owner binding the role granting everything
    const sharer = await Authorization.run(
        home,
        database,
        () => carol,
        (authorization) =>
            Role.create(
                place,
                {
                    name: "sharer",
                    description: "Shares the space",
                    permissions: [space.permission("share")],
                },
                authorization,
            ),
    );
    const dave: AccessContext = {
        grants: [],
        subject: principal.user.reference("universe", "dave"),
        groups: [],
        now: 1000,
        attributes: {},
        actors: [],
        contacts: [],
        linkSecrets: [],
    };
    await Authorization.run(
        home,
        database,
        () => carol,
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
        home,
        database,
        () => carol,
        (authorization) => Relationship.revoke(place, second.id, authorization),
    );
    expect(
        await refusal(
            Authorization.run(
                home,
                database,
                () => carol,
                (authorization) => Relationship.revoke(place, first.id, authorization),
            ),
        ),
    ).toEqual(["CONFLICT", "LAST_OWNER", "the last owner cannot be removed"]);
});

test("create a scope without separate owners, whose account's owners are its owners", async () => {
    const { database } = await openRoleFixture();

    // create carol's account and a space in it without a separate owner
    const home = new Authorizer(policies, homeMappings);
    const asCarol = <Value>(call: (authorization: Authorization) => Promise<Value>) =>
        Authorization.run(home, database, () => carol, call);
    const created = account.reference("universe", "account-1");
    const place = space.reference("account-1", "shared");
    await database.insert(accountTable).values({ id: "account-1", scope: "universe" });
    await database.insert(spaceTable).values({ id: "shared", account: "account-1" });
    await asCarol((authorization) => authorization.create(created, { owner: carol.subject }));
    await asCarol((authorization) => authorization.create(place, {}));

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
            asCarol((authorization) =>
                authorization.create(node.reference("shared", "a"), {
                    owner: carol.subject,
                }),
            ),
        ),
    ).toEqual(["BAD_REQUEST", "SCOPE_REQUIRED", "only a scope can have owners"]);
    expect(decision.isAllowed).toBe(true);
});

test("list the spaces an account contains to the account's owner", async () => {
    const { database, alice } = await openRoleFixture();
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
    const access = new AccessFixture(database);
    await access.replicateScope(account.reference("universe", "account-1"));
    await access.bindOwner(account.reference("universe", "account-1"), carol.subject, 1);
    await access.replicateScope(account.reference("universe", "account-2"));
    for (const [id, owner] of [
        ["space-1", "account-1"],
        ["space-2", "account-1"],
        ["space-3", "account-2"],
    ] as const) {
        await new AccessFixture(database).replicateScope(space.reference(owner, id), "self");
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
    await new AccessFixture(database).replicateScope(account.reference("account-1", "account-3"));
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
    const { database, query, defineRole, relate } = await openRoleFixture();
    const bob = principal.user.reference("universe", "bob");
    const dave = {
        subject: principal.user.reference("universe", "dave"),
        groups: [],
        now: 1000,
        attributes: {},
        actors: [],
        contacts: [],
        linkSecrets: [],
        grants: [],
    };

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
        () => ({
            grants: [],
            subject: bob,
            groups: [],
            now: 1000,
            attributes: {},
            actors: [],
            contacts: [],
            linkSecrets: [],
        }),
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
    await database
        .delete(accessRelationship)
        .where(eq(accessRelationship.id, schema.identifier("relationship").parse(binding)));
    expect(
        await refusal(
            Authorization.run(
                query,
                database,
                () => dave,
                (authorization) =>
                    Invitation.accept(offer.relationship.object, offer.id, authorization),
            ),
        ),
    ).toEqual(["FORBIDDEN", "PERMISSION_DENIED", "permission denied: share"]);
});

/** Open the role fixture with carol as owner of the personal space in its home database. */
async function openSpaceWithOwner() {
    const fixture = await openRoleFixture();
    const home = new Authorizer(policies, homeMappings);
    const place = space.reference("universe", "personal");
    await fixture.database.insert(spaceTable).values({ id: "personal", account: "universe" });
    await Authorization.run(
        home,
        fixture.database,
        () => carol,
        (authorization) =>
            authorization.create(place, {
                owner: carol.subject,
            }),
    );

    // authorize carol at a time
    const owner = <Value>(now: number, call: (authorization: Authorization) => Promise<Value>) =>
        Authorization.run(home, fixture.database, () => ({ ...carol, now }), call);

    return { ...fixture, home, place, owner };
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
    await database.update(accessRole).set({ detachedAt: 1 }).where(eq(accessRole.name, "editor"));
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

test("replace an inherent role's permissions with the declared ones, leaving the rows of the ones it grants already", async () => {
    const { database, place } = await openSpaceWithOwner();
    const read = { packageId: node.definition.packageId, type: node.name, name: "read" };
    const update = { ...read, name: "update" };
    const remove = { ...read, name: "delete" };
    const dave = principal.user.reference("universe", "dave");
    const declare = async (permissions: (typeof read)[], now: number) => {
        const transaction = await Transaction.open(database);
        const upserted = await Role.upsertInherent(
            transaction,
            place,
            { name: "maintainer", description: "Maintain nodes", permissions },
            dave,
            now,
        );
        await transaction.commit();

        return upserted;
    };
    const rows = async (id: string) =>
        database
            .select({ id: accessRolePermission.id, name: accessRolePermission.name })
            .from(accessRolePermission)
            .where(eq(accessRolePermission.roleId, schema.identifier("role").parse(id)))
            .orderBy(asc(accessRolePermission.name));

    // declare the role, then declare it with one permission swapped
    const declared = await declare([read, update], 1000);
    const first = await rows(declared);
    await declare([update, remove], 2000);
    const second = await rows(declared);

    // leave the update permission's row and replace the read one with a new delete row
    const [readRow, updateRow] = first;
    const [added] = second;
    const ids = [...first, ...second].map((row) => row.id);
    expect({ first, second, distinct: new Set(ids).size }).toEqual({
        first: [
            { id: readRow?.id, name: "read" },
            { id: updateRow?.id, name: "update" },
        ],
        second: [{ id: added?.id, name: "delete" }, updateRow],
        distinct: 3,
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
        Role.assign(place, reader.id, carol.subject, authorization),
    );
    const assigned = await owner(2000, (authorization) =>
        Role.assign(place, reader.id, dave, authorization),
    );
    const again = await owner(3000, (authorization) =>
        Role.assign(place, reader.id, dave, authorization),
    );

    // expect only dave's first binding on the scope's object
    const rows = await database
        .select()
        .from(accessRelationship)
        .where(eq(accessRelationship.roleId, schema.identifier("role").parse(reader.id)));
    const binding = {
        id: assigned.id,
        object: place,
        role: reader.id,
        subject: dave,
        createdAt: 2000,
        expiresAt: null,
    };
    expect([again, rows.map((row) => Relationship.decode(row))]).toEqual([binding, [binding]]);
});

test("refuse upserting and assigning roles to a caller without the permissions", async () => {
    const { place, home, database, alice, owner } = await openSpaceWithOwner();
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
        Authorization.run(home, database, () => alice, call);
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
                Role.assign(place, reader.id, alice.subject, authorization),
            ),
        ),
    ).toEqual(["FORBIDDEN", "PERMISSION_DENIED", "permission denied: share"]);
});
