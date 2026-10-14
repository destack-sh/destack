import { expect, onTestFinished, test } from "@destack/test";
import { Scope } from "@destack/sync";
import { Transaction, Snapshot, and, asc, eq, ne } from "@destack/db";
import { PackageId } from "@destack/package";
import { aligned, schema } from "@destack/schema";
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
    subjects: [principal.user.reference("universe", "carol"), carolMembership],
    now: 1000,
    attributes: {},
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

test("find a scope's role by name, and read an object's relationships through a relation, a role or one subject", async () => {
    // bind a reader role on note a to carol's membership, and let dave view it
    const { database, defineRole, relate } = await openRoleFixture();
    const reader = await defineRole(1, [node.permission("read")]);
    const object = node.reference("personal", "a");
    const dave = principal.user.reference("universe", "dave");
    const bound = await relate({ object, role: reader, subject: carolMembership });
    const viewing = await relate({ object, relation: "viewer", subject: dave });

    // read the role and each selection of the note's relationships
    const snapshot = Snapshot.live(database);
    const read = async (match: Parameters<typeof Relationship.readOn>[2]) =>
        (await Relationship.readOn(snapshot, object, match)).map((row) => row.id).toSorted();
    expect({
        role: (await Role.find(snapshot, "personal", "role-1"))?.id,
        elsewhere: await Role.find(snapshot, "elsewhere", "role-1"),
        all: await read({}),
        bound: await read({ role: reader }),
        viewing: await read({ relation: "viewer" }),
        held: await read({ subject: dave }),
        unheld: await read({ relation: "viewer", subject: carolMembership }),
    }).toEqual({
        role: reader,
        elsewhere: null,
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

    // place the personal scope inside the account its members own
    const access = new AccessFixture(database);
    await access.replicateScope(account.reference("universe", "account-1"));
    await access.replicateOwner(account.reference("universe", "account-1"), carolMembership, 1);
    await access.replicateScope(space.reference("account-1", "personal"), "own");

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
            (authorization) => authorization.deleteRole(object, reader, 1),
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
        subjects: [
            ...carol.subjects,
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

    // relate the members of a team to "c" and keep carol as its member in a column
    await relate({
        object: node.reference("personal", "c"),
        relation: "viewer",
        subject: { ...team.reference("personal", "team-1"), relation: "member" },
    });
    expect(await readable(carol)).toEqual([]);
    await database.insert(teamTable).values({ id: "team-1", scope: "personal", member: "carol" });
    expect(await readable(carol)).toEqual(["c"]);
});

test("bind only roles whose permissions the granting caller has", async () => {
    const { database, query, alice, defineRole } = await openRoleFixture();

    // refuse binding a role that does not exist
    await expect(
        Authorization.run(
            query,
            database,
            () => alice,
            (authorization) =>
                authorization.grant({
                    object: node.reference("personal", "a"),
                    role: "role-01996ab0-0000-7000-8000-000000000999",
                    subject: carolMembership,
                }),
        ),
    ).rejects.toMatchObject({ code: "NOT_FOUND", message: "role not found" });

    // let the owner bind a role reading her note
    const reader = await defineRole(1, [node.permission("read")]);
    await Authorization.run(
        query,
        database,
        () => alice,
        (authorization) =>
            authorization.grant({
                object: node.reference("personal", "a"),
                role: reader,
                subject: carolMembership,
            }),
    );

    // reject binding a role granting a permission she does not have there
    const escalating = await defineRole(2, [node.permission("read"), cell.permission("edit")]);
    await expect(
        Authorization.run(
            query,
            database,
            () => alice,
            (authorization) =>
                authorization.grant({
                    object: node.reference("personal", "a"),
                    role: escalating,
                    subject: carolMembership,
                }),
        ),
    ).rejects.toMatchObject({
        code: "FORBIDDEN",
        message: "permission denied: edit",
    });

    // refuse binding a role to a subject set that resolving never expands
    await expect(
        Authorization.run(
            query,
            database,
            () => alice,
            (authorization) =>
                authorization.grant({
                    object: node.reference("personal", "a"),
                    role: reader,
                    subject: { ...node.reference("personal", "b"), relation: "viewer" },
                }),
        ),
    ).rejects.toMatchObject({
        code: "FORBIDDEN",
        message:
            "a role cannot bind to node#viewer: it is not a declared subject set or scope relation",
    });
});

test("let owners have everything in their scope, make owners, and never remove the last one", async () => {
    const { database, readable, alice } = await openRoleFixture();

    // create the personal space in its home database, owned by carol: she reads every note in it
    const home = new Authorizer(policies, homeMappings);
    const place = space.reference("universe", "personal");
    await database.insert(spaceTable).values({ id: "personal", account: "universe" });
    await Authorization.run(
        home,
        database,
        () => carol,
        (authorization) =>
            authorization.create(place, {
                owner: aligned(carol.subjects, 0),
            }),
    );
    expect(await readable(carol)).toEqual(["a", "b", "c"]);

    // let an owner make another owner, and refuse anyone else
    const [owner] = await database
        .select({ id: accessRole.id })
        .from(accessRole)
        .where(eq(accessRole.isUniversal, true));
    if (owner === undefined) {
        throw new Error("creating the space defines no owner role");
    }
    await expect(
        Authorization.run(
            home,
            database,
            () => alice,
            (authorization) =>
                authorization.grant({
                    object: place,
                    role: owner.id,
                    subject: principal.user.reference("universe", "bob"),
                }),
        ),
    ).rejects.toMatchObject({ code: "FORBIDDEN", message: "permission denied: share" });
    const second = await Authorization.run(
        home,
        database,
        () => carol,
        (authorization) =>
            authorization.grant({
                object: place,
                role: owner.id,
                subject: aligned(alice.subjects, 0),
            }),
    );

    // refuse a sharer who is not an owner binding the role granting everything
    const sharer = await Authorization.run(
        home,
        database,
        () => carol,
        (authorization) =>
            authorization.createRole(place, {
                name: "sharer",
                description: "Shares the space",
                permissions: [space.permission("share")],
            }),
    );
    const dave: AccessContext = {
        subjects: [principal.user.reference("universe", "dave")],
        now: 1000,
        attributes: {},
    };
    await Authorization.run(
        home,
        database,
        () => carol,
        (authorization) =>
            authorization.grant({
                object: place,
                role: sharer.id,
                subject: aligned(dave.subjects, 0),
            }),
    );
    await expect(
        Authorization.run(
            home,
            database,
            () => dave,
            (authorization) =>
                authorization.grant({
                    object: place,
                    role: owner.id,
                    subject: principal.user.reference("universe", "bob"),
                }),
        ),
    ).rejects.toMatchObject({
        code: "FORBIDDEN",
        message: "only owners may bind a role granting everything",
    });

    // keep at least one owner
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
        throw new Error("the space keeps no first owner");
    }
    await Authorization.run(
        home,
        database,
        () => carol,
        (authorization) => authorization.revoke(place, second.id),
    );
    await expect(
        Authorization.run(
            home,
            database,
            () => carol,
            (authorization) => authorization.revoke(place, first.id),
        ),
    ).rejects.toMatchObject({ code: "CONFLICT" });
});

test("create a scope without owners of its own, which the owners of its account own", async () => {
    const { database } = await openRoleFixture();

    // create carol's account and a space in it without its own owner
    const home = new Authorizer(policies, homeMappings);
    const asCarol = <Value>(call: (authorization: Authorization) => Promise<Value>) =>
        Authorization.run(home, database, () => carol, call);
    const owned = account.reference("universe", "account-1");
    const place = space.reference("account-1", "shared");
    await database.insert(accountTable).values({ id: "account-1", scope: "universe" });
    await database.insert(spaceTable).values({ id: "shared", account: "account-1" });
    await asCarol((authorization) =>
        authorization.create(owned, { owner: aligned(carol.subjects, 0) }),
    );
    await asCarol((authorization) => authorization.create(place, {}));

    // let carol have everything in the space through her account
    const access = await home.resolve(Snapshot.live(database), "shared", carol);
    const decision = await home.check(
        Snapshot.live(database),
        space.permission("update"),
        place,
        access,
    );
    await expect(
        asCarol((authorization) =>
            authorization.create(node.reference("shared", "a"), {
                owner: aligned(carol.subjects, 0),
            }),
        ),
    ).rejects.toMatchObject({ code: "INVALID_CONTEXT", message: "only a scope can have owners" });
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
    await access.replicateOwner(
        account.reference("universe", "account-1"),
        aligned(carol.subjects, 0),
        1,
    );
    await access.replicateScope(account.reference("universe", "account-2"));
    for (const [id, owner] of [
        ["space-1", "account-1"],
        ["space-2", "account-1"],
        ["space-3", "account-2"],
    ] as const) {
        await new AccessFixture(database).replicateScope(space.reference(owner, id), "own");
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
        subjects: [principal.user.reference("universe", "dave")],
        now: 1000,
        attributes: {},
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
            subjects: [bob],
            now: 1000,
            attributes: {},
        }),
        (authorization) =>
            authorization.invite({
                relationship: {
                    object: node.reference("personal", "a"),
                    relation: "viewer",
                    subject: aligned(dave.subjects, 0),
                },
            }),
    );

    // refuse acceptance once bob no longer has the role
    await database
        .delete(accessRelationship)
        .where(eq(accessRelationship.id, schema.identifier("relationship").parse(binding)));
    await expect(
        Authorization.run(
            query,
            database,
            () => dave,
            (authorization) => authorization.accept(offer.relationship.object, offer.id),
        ),
    ).rejects.toMatchObject({
        code: "FORBIDDEN",
        message: "permission denied: share",
    });
});

/** Open the role fixture with carol owning the personal space in its home database. */
async function openOwnedSpace() {
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
                owner: aligned(carol.subjects, 0),
            }),
    );

    // authorize carol at a time
    const owner = <Value>(now: number, call: (authorization: Authorization) => Promise<Value>) =>
        Authorization.run(home, fixture.database, () => ({ ...carol, now }), call);

    return { ...fixture, home, place, owner };
}

test("keep a role by name, replace its purpose and permissions, then keep it identically without a write", async () => {
    const { database, place, owner } = await openOwnedSpace();
    const read = { packageId: node.definition.packageId, type: node.name, name: "read" };
    const update = { ...read, name: "update" };

    // keep the role, keep it again under the same name with other permissions, and once more alike
    const created = await owner(1000, (authorization) =>
        authorization.keepRole(place, {
            name: "editor",
            description: "Read nodes",
            permissions: [read],
        }),
    );
    const changed = { name: "editor", description: "Update nodes", permissions: [update] };
    await owner(2000, (authorization) => authorization.keepRole(place, changed));
    const kept = await owner(3000, (authorization) => authorization.keepRole(place, changed));

    // expect one role at its second revision, last written at the change, granting only the kept permissions
    const reading = await Transaction.open(database);
    const record = await Role.read(reading, "personal", kept.id);
    const expected = {
        ...created,
        description: "Update nodes",
        permissions: [update],
        revision: 2,
    };
    expect([
        kept,
        Role.describe(record, await Role.permissions(reading, kept.id)),
        record.updatedAt,
    ]).toEqual([expected, expected, 2000]);
});

test("keep an inherent role's permissions as declared, keeping the rows of the ones it grants already", async () => {
    const { database, place } = await openOwnedSpace();
    const read = { packageId: node.definition.packageId, type: node.name, name: "read" };
    const update = { ...read, name: "update" };
    const remove = { ...read, name: "delete" };
    const dave = principal.user.reference("universe", "dave");
    const keep = async (permissions: (typeof read)[], now: number) => {
        const transaction = await Transaction.open(database);
        const keeper = await Role.keepInherent(
            transaction,
            place,
            { name: "keeper", description: "Keep nodes", permissions },
            dave,
            now,
        );
        await transaction.commit();

        return keeper;
    };
    const rows = async (id: string) =>
        database
            .select({ id: accessRolePermission.id, name: accessRolePermission.name })
            .from(accessRolePermission)
            .where(eq(accessRolePermission.roleId, schema.identifier("role").parse(id)))
            .orderBy(asc(accessRolePermission.name));

    // keep the role, then keep it with one permission swapped
    const keeper = await keep([read, update], 1000);
    const first = await rows(keeper);
    await keep([update, remove], 2000);
    const second = await rows(keeper);

    // keep the update permission's row and replace the read one with a new delete row
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

test("assign a role to one subject alone, revoking the previous subject and keeping an identical binding", async () => {
    const { database, place, owner } = await openOwnedSpace();
    const reader = await owner(1000, (authorization) =>
        authorization.keepRole(place, {
            name: "reader",
            description: "Read nodes",
            permissions: [],
        }),
    );

    // assign the role to carol, to dave and to dave again
    const dave = principal.user.reference("universe", "dave");
    await owner(1000, (authorization) =>
        authorization.assign(place, reader.id, aligned(carol.subjects, 0)),
    );
    const assigned = await owner(2000, (authorization) =>
        authorization.assign(place, reader.id, dave),
    );
    const kept = await owner(3000, (authorization) => authorization.assign(place, reader.id, dave));

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
    expect([kept, rows.map((row) => Relationship.decode(row))]).toEqual([binding, [binding]]);
});

test("refuse keeping and assigning roles to a caller without the permissions", async () => {
    const { place, home, database, alice, owner } = await openOwnedSpace();
    const reader = await owner(1000, (authorization) =>
        authorization.keepRole(place, {
            name: "reader",
            description: "Read the space",
            permissions: [space.permission("read")],
        }),
    );

    // refuse alice defining, changing and assigning a role in carol's space
    const outsider = <Value>(call: (authorization: Authorization) => Promise<Value>) =>
        Authorization.run(home, database, () => alice, call);
    await expect(
        outsider((authorization) =>
            authorization.keepRole(place, {
                name: "sharer",
                description: "Share",
                permissions: [],
            }),
        ),
    ).rejects.toMatchObject({ code: "FORBIDDEN", message: "permission denied: create" });
    await expect(
        outsider((authorization) =>
            authorization.keepRole(place, { name: "reader", description: "Read", permissions: [] }),
        ),
    ).rejects.toMatchObject({ code: "FORBIDDEN", message: "permission denied: update" });
    await expect(
        outsider((authorization) =>
            authorization.assign(place, reader.id, aligned(alice.subjects, 0)),
        ),
    ).rejects.toMatchObject({ code: "FORBIDDEN", message: "permission denied: share" });
});
