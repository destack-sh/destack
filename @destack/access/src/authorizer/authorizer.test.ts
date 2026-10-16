import { expect, test } from "@destack/test";
import { Snapshot, Transaction, eq, type DatabaseConnection, Table } from "@destack/db";
import { nth, Identifier, schema } from "@destack/schema";
import { Scope, type Subject } from "@destack/sync";
import {
    AccessError,
    ACCESS_PACKAGE_ID,
    accessGrant,
    accessRelationship,
    Grant,
    Contact,
    Authorizer,
    LinkSecret,
    Policy,
    principal,
    through,
    type AccessContext,
    type RelationshipCondition,
    ANONYMOUS,
    Invitation,
    Relationship,
} from "../index.ts";
import {
    account,
    cell,
    entity,
    group,
    item,
    mappings,
    module1,
    node,
    policies,
    rows,
    space,
} from "../test/fixture.ts";
import { Authorization } from "./authorization.ts";
import { openFixture, sessionCredential, userSubject } from "../test/database.ts";

/** Provide an isolated database with notes and an explicit editor grant. */
const databaseTest = test.extend<{
    fixture: Awaited<ReturnType<typeof openFixture>>;
}>({
    fixture: async ({ task: _task }, use) => {
        const fixture = await openFixture();
        try {
            await use(fixture);
        } finally {
            await fixture.close();
        }
    },
});

databaseTest("authorize notes, ranges and world entities", async ({ fixture }) => {
    const { database, authorizer, alice, bob } = fixture;

    // retain the prepared mapping when the caller changes its input records
    const selections = mappings.map((mapping) => ({
        ...mapping,
        relations: { ...mapping.relations },
        attributes: { ...mapping.attributes },
    }));
    const query = new Authorizer(policies, selections);
    const nodeSelection = selections.find((mapping) => mapping.policy === node);
    const cellSelection = selections.find((mapping) => mapping.policy === cell);
    if (nodeSelection === undefined || cellSelection === undefined) {
        throw new Error("the fixture maps nodes and cells");
    }
    nodeSelection.relations["owner"] = { column: "parent", scope: "other" };
    cellSelection.attributes["row"] = "column";

    expect(
        await database
            .select({ id: item.id })
            .from(item)
            .where(
                query.where(
                    node.permission("edit"),
                    await query.resolve(Snapshot.live(database), "personal", bob),
                ),
            )
            .orderBy(item.id),
    ).toEqual([{ id: "b" }]);
    await expect(
        Authorization.run(
            authorizer,
            database,
            () => bob,
            (authorization) =>
                Relationship.grant(
                    {
                        object: node.reference("personal", "b"),
                        relation: "editor",
                        subject: userSubject("alice"),
                    },
                    authorization,
                ),
        ),
    ).rejects.toMatchObject({
        code: "FORBIDDEN",
        message: "permission denied: share",
    });

    // select each example's editable rows for its caller
    const elevated = {
        ...bob,
        attributes: { team: 1, phase: "edit" },
        credential: {
            ...sessionCredential("bob", bob.now),
            assurance: { level: 2, authenticatedAt: bob.now },
        },
    };
    const examples = [
        { type: node, context: bob, editable: ["b"] },
        {
            type: cell,
            context: {
                ...alice,
                attributes: {
                    "first-row": 1,
                    "last-row": 3,
                    "first-column": 1,
                    "last-column": 1,
                },
            },
            editable: ["a", "b"],
        },
        { type: entity, context: elevated, editable: ["a"] },
    ];
    for (const { type, context, editable } of examples) {
        const selected = await database
            .select({ id: item.id })
            .from(item)
            .where(
                authorizer.where(
                    type.permission("edit"),
                    await authorizer.resolve(Snapshot.live(database), "personal", context),
                ),
            )
            .orderBy(item.id);
        expect(selected.map((row) => row.id)).toEqual(editable);
    }

    // apply the elevated edit permission only after recent strong authentication
    const stale = {
        ...elevated,
        credential: {
            ...sessionCredential("bob", bob.now),
            assurance: { level: 2, authenticatedAt: bob.now - 3_600_000 },
        },
    };
    expect(
        await database
            .select({ id: item.id })
            .from(item)
            .where(
                authorizer.where(
                    entity.permission("edit"),
                    await authorizer.resolve(Snapshot.live(database), "personal", stale),
                ),
            ),
    ).toEqual([]);

    // challenge the stale caller for the elevation's authentication, and refuse one no authentication admits
    const snapshot = Snapshot.live(database);
    const refusals = [];
    for (const context of [stale, { ...stale, attributes: { team: 1, phase: "view" } }]) {
        const access = await authorizer.resolve(snapshot, "personal", context);
        refusals.push(
            await authorizer
                .require(
                    snapshot,
                    [entity.permission("edit")],
                    entity.reference("personal", "a"),
                    access,
                )
                .catch((error: AccessError) => [error.code, error.message, error.stepUp]),
        );
    }
    expect(refusals).toEqual([
        [
            "INSUFFICIENT_AUTHENTICATION",
            "authentication assurance is too low: authenticate again",
            { assurance: 2, maxAge: 15 * 60 * 1000 },
        ],
        ["FORBIDDEN", "permission denied: edit", undefined],
    ]);
});

databaseTest(
    "share notes through wildcards and expiring, revocable credentials",
    async ({ fixture }) => {
        const { database, authorizer, alice } = fixture;
        // relate every user to a note, but no anonymous caller
        await Authorization.run(
            authorizer,
            database,
            () => alice,
            (authorization) =>
                Relationship.grant(
                    {
                        object: node.reference("personal", "a"),
                        relation: "viewer",
                        subject: principal.user.reference("*", "*"),
                    },
                    authorization,
                ),
        );
        const carol = {
            ...alice,
            subject: principal.user.reference("universe", "carol"),
            groups: [],
        };
        const anonymous = { ...alice, subject: ANONYMOUS, groups: [] };
        for (const [context, expected] of [
            [carol, [{ id: "a" }]],
            [anonymous, []],
        ] as const) {
            expect(
                await database
                    .select({ id: item.id })
                    .from(item)
                    .where(
                        authorizer.where(
                            node.permission("read"),
                            await authorizer.resolve(Snapshot.live(database), "personal", context),
                        ),
                    ),
            ).toEqual(expected);
        }
        const shared = await Authorization.run(
            authorizer,
            database,
            () => alice,
            (authorization) =>
                Relationship.link(
                    {
                        object: node.reference("personal", "b"),
                        relation: "subtree-editor",
                        expiresAt: 2000,
                    },
                    authorization,
                ),
        );

        // let anyone presenting the link edit the subtree until it expires
        const visitor = {
            ...anonymous,
            contacts: [],
            linkSecrets: [await LinkSecret.digest(shared.secret)],
        };
        await expectEditable(database, authorizer, node, visitor, ["b", "c"]);
        await expectEditable(database, authorizer, node, { ...visitor, now: 2000 }, []);
        const stranger = {
            ...anonymous,
            contacts: [],
            linkSecrets: [(await LinkSecret.create()).digest],
        };
        await expectEditable(database, authorizer, node, stranger, []);

        // revoke the link by deleting its relationship
        await Authorization.run(
            authorizer,
            database,
            () => alice,
            (authorization) =>
                Relationship.revoke(node.reference("personal", "b"), shared.id, authorization),
        );
        await expectEditable(database, authorizer, node, visitor, []);

        // create a second link beside another, and refuse writing an existing relationship again
        const first = await Authorization.run(
            authorizer,
            database,
            () => alice,
            (authorization) =>
                Relationship.link(
                    {
                        object: node.reference("personal", "a"),
                        relation: "viewer",
                    },
                    authorization,
                ),
        );
        const second = await Authorization.run(
            authorizer,
            database,
            () => alice,
            (authorization) =>
                Relationship.link(
                    {
                        object: node.reference("personal", "a"),
                        relation: "viewer",
                    },
                    authorization,
                ),
        );
        expect(second.id).not.toBe(first.id);
        await expect(
            Authorization.run(
                authorizer,
                database,
                () => alice,
                (authorization) =>
                    Relationship.grant(
                        {
                            object: node.reference("personal", "b"),
                            relation: "editor",
                            subject: userSubject("bob"),
                        },
                        authorization,
                    ),
            ),
        ).rejects.toMatchObject({
            code: "CONFLICT",
            message: "relationship already exists",
        });
    },
);

databaseTest(
    "apply invited relationships only once a grantor or the addressee accepts",
    async ({ fixture }) => {
        const { database, authorizer, alice, bob } = fixture;
        const carol: AccessContext = {
            ...alice,
            subject: principal.user.reference("universe", "carol"),
            groups: [],
            contacts: [Contact.email("carol@example.com")],
        };
        const dave: AccessContext = {
            ...alice,
            subject: principal.user.reference("universe", "dave"),
            groups: [],
        };

        // let carol ask for access without it applying until the owner accepts
        const asked = await Authorization.run(
            authorizer,
            database,
            () => carol,
            (authorization) =>
                Invitation.create(
                    {
                        relationship: {
                            object: node.reference("personal", "c"),
                            relation: "editor",
                            subject: userSubject("carol"),
                        },
                        purpose: "review the draft",
                    },
                    authorization,
                ),
        );
        await expectEditable(database, authorizer, node, carol, []);
        expect(
            (
                await Authorization.run(
                    authorizer,
                    database,
                    () => alice,
                    (authorization) =>
                        Invitation.list(
                            { object: node.reference("personal", "c") },
                            {
                                limit: 10,
                            },
                            authorization,
                        ),
                )
            ).map((invitation) => [invitation.id, invitation.purpose]),
        ).toEqual([[asked.id, "review the draft"]]);

        // hide the lapsed invitation and refuse unbounded pages
        expect(
            await Authorization.run(
                authorizer,
                database,
                () => ({
                    ...alice,
                    now: asked.expiresAt,
                }),
                (authorization) =>
                    Invitation.list(
                        { object: node.reference("personal", "c") },
                        { limit: 10 },
                        authorization,
                    ),
            ),
        ).toEqual([]);
        await expect(
            Authorization.run(
                authorizer,
                database,
                () => alice,
                (authorization) =>
                    Invitation.list(
                        { object: node.reference("personal", "c") },
                        { limit: 101 },
                        authorization,
                    ),
            ),
        ).rejects.toMatchObject({
            code: "INVALID_CONTEXT",
            message: "invitation page limit must be 1 to 100",
        });
        await expect(
            Authorization.run(
                authorizer,
                database,
                () => carol,
                (authorization) =>
                    Invitation.accept(asked.relationship.object, asked.id, authorization),
            ),
        ).rejects.toMatchObject({
            code: "FORBIDDEN",
            message: "permission denied: share",
        });
        await expect(
            Authorization.run(
                authorizer,
                database,
                () => carol,
                (authorization) =>
                    Invitation.list(
                        { object: node.reference("personal", "c") },
                        { limit: 10 },
                        authorization,
                    ),
            ),
        ).rejects.toMatchObject({
            code: "FORBIDDEN",
            message: "permission denied: share",
        });
        await Authorization.run(
            authorizer,
            database,
            () => alice,
            (authorization) =>
                Invitation.accept(asked.relationship.object, asked.id, authorization),
        );
        await expectEditable(database, authorizer, node, carol, ["c"]);

        // offer access to an email address and bind it to whoever proves it
        const offered = await Authorization.run(
            authorizer,
            database,
            () => alice,
            (authorization) =>
                Invitation.create(
                    {
                        relationship: {
                            object: node.reference("personal", "a"),
                            relation: "editor",
                            subject: Contact.subject(Contact.email("carol@example.com")),
                        },
                    },
                    authorization,
                ),
        );
        await expect(
            Authorization.run(
                authorizer,
                database,
                () => dave,
                (authorization) =>
                    Invitation.create(
                        {
                            relationship: {
                                object: node.reference("personal", "a"),
                                relation: "editor",
                                subject: Contact.subject(Contact.email("dave@example.com")),
                            },
                        },
                        authorization,
                    ),
            ),
        ).rejects.toMatchObject({
            code: "FORBIDDEN",
            message: "permission denied: share",
        });
        await expect(
            Authorization.run(
                authorizer,
                database,
                () => dave,
                (authorization) =>
                    Invitation.accept(offered.relationship.object, offered.id, authorization),
            ),
        ).rejects.toMatchObject({
            code: "FORBIDDEN",
            message: "invitation is addressed to someone else",
        });
        const agent = principal.installation.reference("personal", "assistant");
        await expect(
            Authorization.run(
                authorizer,
                database,
                () => ({
                    ...carol,
                    subject: userSubject("carol"),
                    actors: [agent],
                    grant: { id: Identifier.create("grant"), scope: "carol", permissions: [] },
                }),
                (authorization) =>
                    Invitation.accept(offered.relationship.object, offered.id, authorization),
            ),
        ).rejects.toMatchObject({
            code: "FORBIDDEN",
            message: "invitation is addressed to someone else",
        });
        await Authorization.run(
            authorizer,
            database,
            () => carol,
            (authorization) =>
                Invitation.accept(offered.relationship.object, offered.id, authorization),
        );
        await expectEditable(database, authorizer, node, carol, ["a", "c"]);

        // let the addressee refuse an offer and a grantor reject a request
        const refused = await Authorization.run(
            authorizer,
            database,
            () => alice,
            (authorization) =>
                Invitation.create(
                    {
                        relationship: {
                            object: node.reference("work", "d"),
                            relation: "editor",
                            subject: userSubject("bob"),
                        },
                    },
                    authorization,
                ),
        );
        await Authorization.run(
            authorizer,
            database,
            () => bob,
            (authorization) =>
                Invitation.withdraw(refused.relationship.object, refused.id, authorization),
        );
        const rejected = await Authorization.run(
            authorizer,
            database,
            () => dave,
            (authorization) =>
                Invitation.create(
                    {
                        relationship: {
                            object: node.reference("work", "d"),
                            relation: "editor",
                            subject: userSubject("dave"),
                        },
                    },
                    authorization,
                ),
        );
        await expect(
            Authorization.run(
                authorizer,
                database,
                () => carol,
                (authorization) =>
                    Invitation.withdraw(rejected.relationship.object, rejected.id, authorization),
            ),
        ).rejects.toMatchObject({
            code: "FORBIDDEN",
            message: "permission denied: share",
        });
        await Authorization.run(
            authorizer,
            database,
            () => alice,
            (authorization) =>
                Invitation.withdraw(rejected.relationship.object, rejected.id, authorization),
        );
        await expectEditable(database, authorizer, node, dave, []);
        expect(
            await Authorization.run(
                authorizer,
                database,
                () => alice,
                (authorization) =>
                    Invitation.list(
                        { object: node.reference("work", "d") },
                        { limit: 10 },
                        authorization,
                    ),
            ),
        ).toEqual([]);
    },
);

databaseTest(
    "apply grants only under their request, session and authentication conditions",
    async ({ fixture }) => {
        const { database, authorizer, alice, bob } = fixture;
        const grant = async (conditions: RelationshipCondition) =>
            Authorization.run(
                authorizer,
                database,
                () => alice,
                (authorization) =>
                    Relationship.grant(
                        {
                            object: node.reference("personal", "c"),
                            relation: "viewer",
                            subject: principal.user.reference("universe", "carol"),
                            conditions,
                        },
                        authorization,
                    ),
            );
        const carol: AccessContext = {
            ...bob,
            subject: principal.user.reference("universe", "carol"),
            groups: [],
        };
        const readable = async (context: AccessContext) =>
            readableNodes(database, authorizer, context);

        // bind a grant to one request and one credential
        const once = await grant({ request: "request-1" });
        expect(await readable(carol)).toEqual([]);
        expect(await readable({ ...carol, request: "request-1" })).toEqual(["c"]);
        await authorizer.release(database, "request-1");
        expect(await readable({ ...carol, request: "request-1" })).toEqual([]);
        const bound = await grant({ credential: "session:1" });
        expect(await readable({ ...carol, credential: sessionCredential("2", carol.now) })).toEqual(
            [],
        );
        expect(await readable({ ...carol, credential: sessionCredential("1", carol.now) })).toEqual(
            ["c"],
        );
        await Authorization.run(
            authorizer,
            database,
            () => alice,
            (authorization) => Relationship.revoke(bound.object, bound.id, authorization),
        );

        // require strong and recent authentication
        await grant({ assurance: 2, maxAge: 300 });
        expect(await readable(carol)).toEqual([]);
        expect(
            await readable({
                ...carol,
                credential: {
                    ...sessionCredential("1", carol.now),
                    assurance: { level: 1, authenticatedAt: 1000 },
                },
            }),
        ).toEqual([]);
        expect(
            await readable({
                ...carol,
                credential: {
                    ...sessionCredential("1", carol.now),
                    assurance: { level: 2, authenticatedAt: 800 },
                },
            }),
        ).toEqual(["c"]);
        expect(
            await readable({
                ...carol,
                credential: {
                    ...sessionCredential("1", carol.now),
                    assurance: { level: 3, authenticatedAt: 600 },
                },
            }),
        ).toEqual([]);

        // remove the consumed request grant
        expect(
            await database
                .select()
                .from(accessRelationship)
                .where(eq(accessRelationship.id, schema.identifier("relationship").parse(once.id))),
        ).toEqual([]);
    },
);

databaseTest(
    "narrow a delegate to what its subject may and its grant gives, until the grant is revoked or lapses",
    async ({ fixture }) => {
        const { database, authorizer, alice, bob } = fixture;
        const assistant = principal.installation.reference("personal", "assistant");

        // store alice's and bob's own scopes, where their grants live
        await database.insert(Scope.table).values(
            ["alice", "bob"].map((id) =>
                Table.complete(Scope.table, {
                    scope: id,
                    parent: Scope.universe.id,
                    partition: Scope.universe.id,
                    packageId: ACCESS_PACKAGE_ID,
                    type: principal.user.name,
                    ancestors: [],
                }),
            ),
        );

        // grant the assistant editing a and b for each of them until 2000, refusing others' authority and sets
        const grant = (context: AccessContext, scope: string, delegate: Subject = assistant) =>
            Authorization.run(
                authorizer,
                database,
                () => context,
                (authorization) =>
                    Grant.create(
                        scopeOf(scope),
                        {
                            delegate,
                            permissions: [editing("a"), editing("b")],
                            expiresAt: 2000,
                        },
                        authorization,
                    ),
            );
        const [forAlice, forBob] = [await grant(alice, "alice"), await grant(bob, "bob")];
        const refusals = await Promise.all([
            outcome(grant(bob, "alice")),
            outcome(grant(alice, "alice", { ...userSubject("carol"), relation: "member" })),
        ]);

        // allow the assistant what its subject may and the grant gives, and its subject's whole authority when impersonated
        const delegated = (context: AccessContext, granted: typeof forAlice) => ({
            ...context,
            subject: context.subject,
            actors: [assistant],
            grant: Grant.claim(granted),
        });
        await expectEditable(database, authorizer, node, delegated(alice, forAlice), ["a", "b"]);
        await expectEditable(database, authorizer, node, delegated(bob, forBob), ["b"]);
        await expectEditable(
            database,
            authorizer,
            node,
            { ...alice, subject: userSubject("alice"), actors: [userSubject("support")] },
            ["a", "b", "c"],
        );

        // refuse the grant to another delegate, once lapsed, and once revoked
        const resolve = (context: AccessContext) =>
            outcome(authorizer.resolve(Snapshot.live(database), "personal", context));
        const elsewhere = await resolve({
            ...delegated(alice, forAlice),
            grant: Grant.claim(forBob),
        });
        const lapsed = await resolve({ ...delegated(alice, forAlice), now: 2000 });
        await Authorization.run(
            authorizer,
            database,
            () => alice,
            (authorization) => Grant.revoke(scopeOf("alice"), forAlice.id, authorization),
        );
        expect({
            delegateScopes: [forAlice.delegateScope, forBob.delegateScope],
            refusals,
            elsewhere,
            lapsed,
            revoked: await resolve(delegated(alice, forAlice)),
            remaining: await resolve(delegated(bob, forBob)),
        }).toEqual({
            delegateScopes: ["personal", "personal"],
            refusals: [
                "FORBIDDEN: a principal can grant only its own authority",
                "FORBIDDEN: a grant needs one other principal as delegate",
            ],
            elsewhere: `UNAUTHORIZED: grant ${forBob.id} belongs to another delegate`,
            lapsed: `UNAUTHORIZED: grant ${forAlice.id} expired`,
            revoked: `UNAUTHORIZED: grant ${forAlice.id} is revoked`,
            remaining: "resolved",
        });
    },
);

test("follow a scope relation of several scope types to the nearest scope only, and refuse one of a plain type", () => {
    // relate pins to the account or space they live in
    const pin = pinPolicy([account, space]);
    const authorizer = new Authorizer([...policies, pin], mappings);
    const reading = {
        kind: "through",
        relation: "scope",
        permission: "read",
        transitive: false,
    } as const;

    // take the space below its account, the account alone, and refuse a group as a scope
    expect([
        authorizer
            .enclosingTypes(pin, reading, [chainLink(space), chainLink(account)])
            .map((type) => type.type),
        authorizer.enclosingTypes(pin, reading, [chainLink(account)]).map((type) => type.type),
    ]).toEqual([["space"], ["account"]]);
    expect(() => new Authorizer([...policies, pinPolicy([account, group])], mappings)).toThrow(
        new AccessError(
            "INVALID_DECLARATION",
            "pin.scope is a scope relation: it accepts scope types only",
        ),
    );
});

databaseTest("remove a space's delegated grants when the space goes", async ({ fixture }) => {
    const { database, authorizer, alice } = fixture;

    // store alice's own scope, where her grants live
    await database.insert(Scope.table).values(
        Table.complete(Scope.table, {
            scope: "alice",
            parent: Scope.universe.id,
            partition: Scope.universe.id,
            packageId: ACCESS_PACKAGE_ID,
            type: principal.user.name,
            ancestors: [],
        }),
    );

    // grant editing to an installation of each of two spaces
    const grant = (spaceId: string) =>
        Authorization.run(
            authorizer,
            database,
            () => alice,
            (authorization) =>
                Grant.create(
                    scopeOf("alice"),
                    {
                        delegate: principal.installation.reference(spaceId, "assistant"),
                        permissions: [editing("a")],
                        expiresAt: 2000,
                    },
                    authorization,
                ),
        );
    await grant("space-gone");
    await grant("space-remaining");

    // remove the first space
    await Transaction.run(database, (transaction) =>
        authorizer.cascade(transaction, principal.space.reference(Scope.universe.id, "space-gone")),
    );

    // leave only the grant to the remaining space's installation
    const remaining = await database
        .select({ delegateScope: accessGrant.delegateScope })
        .from(accessGrant);
    expect(remaining).toEqual([{ delegateScope: "space-remaining" }]);
});

/** Give editing one node of the personal scope. */
function editing(objectId: string) {
    return { ...node.permission("edit"), scope: "personal", objectId };
}

/** Reference a user's own scope. */
function scopeOf(id: string) {
    return principal.user.reference(Scope.universe.id, id);
}

/** Read how a promise settles: resolved, or its access error's code and message. */
async function outcome(promise: Promise<unknown>): Promise<string> {
    try {
        await promise;

        return "resolved";
    } catch (error) {
        if (!(error instanceof AccessError)) {
            throw error;
        }

        return `${error.code}: ${error.message}`;
    }
}

databaseTest("update inherited permissions when notes move or disappear", async ({ fixture }) => {
    const { database, authorizer, alice, bob } = fixture;

    // share the root and its descendants while retaining the direct grant on b
    await Authorization.run(
        authorizer,
        database,
        () => alice,
        (authorization) =>
            Relationship.grant(
                {
                    object: node.reference("personal", "a"),
                    relation: "subtree-editor",
                    subject: userSubject("bob"),
                },
                authorization,
            ),
    );
    await expectEditable(database, authorizer, node, bob, ["a", "b", "c"]);

    // detach b: its direct grant remains, but c loses inherited root access
    await database.update(item).set({ parent: null }).where(eq(item.id, "b"));
    await expectEditable(database, authorizer, node, bob, ["a", "b"]);

    // reattach b and restore access through the same parent relationship
    await database.update(item).set({ parent: "a" }).where(eq(item.id, "b"));
    await expectEditable(database, authorizer, node, bob, ["a", "b", "c"]);

    // remove the leaf and restrict edits to the remaining records
    await database.delete(item).where(eq(item.id, "c"));
    await expectEditable(database, authorizer, node, bob, ["a", "b"]);
});

databaseTest(
    "end a caller's access at the moment a relationship its listings read expires",
    async ({ fixture }) => {
        const { database, authorizer, alice, bob } = fixture;

        // let bob view c until 1500
        await Authorization.run(
            authorizer,
            database,
            () => alice,
            (authorization) =>
                Relationship.grant(
                    {
                        object: node.reference("personal", "c"),
                        relation: "viewer",
                        subject: userSubject("bob"),
                        expiresAt: 1500,
                    },
                    authorization,
                ),
        );

        // list b and c until the expiry, and only b from then on
        const listed = async (now: number) => {
            const access = await authorizer.resolve(Snapshot.live(database), "personal", {
                ...bob,
                now,
            });
            const read = await database
                .select({ id: item.id })
                .from(item)
                .where(authorizer.where(node.permission("read"), access))
                .orderBy(item.id);

            return { until: access.until, read: read.map((row) => row.id) };
        };
        expect([await listed(1000), await listed(1500)]).toEqual([
            { until: 1500, read: ["b", "c"] },
            { until: undefined, read: ["b"] },
        ]);
    },
);

databaseTest("evaluate more rows than one query may bind", async ({ fixture }) => {
    const { database, authorizer, alice } = fixture;

    // present thousands of copies of alice's notes, beyond one query's parameter limit
    const copies = Array.from({ length: 4000 }, (_, position) => ({
        ...nth(rows, position % 3),
        id: `copy-${position}`,
    }));
    const access = await authorizer.resolve(Snapshot.live(database), "personal", {
        ...alice,
        attributes: {
            "first-row": 1,
            "last-row": 3,
            "first-column": 1,
            "last-column": 1,
        },
    });
    const permitted = await authorizer.checkRows(
        Snapshot.live(database),
        cell.permission("edit"),
        access,
        copies,
    );

    // permit editing every copy of the two unlocked cells alice owns
    expect(permitted.permitted.size).toBe(copies.filter((copy) => copy.locked === 0).length);
});

/** Read the nodes a context may read. */
async function readableNodes(
    database: DatabaseConnection,
    authorizer: Authorizer,
    context: AccessContext,
): Promise<string[]> {
    const readable = (
        await database
            .select({ id: item.id })
            .from(item)
            .where(
                authorizer.where(
                    node.permission("read"),
                    await authorizer.resolve(Snapshot.live(database), "personal", context),
                ),
            )
            .orderBy(item.id)
    ).map((row) => row.id);

    return readable;
}

/** Require the nodes of a type a context may edit to be exactly the expected ones. */
async function expectEditable(
    database: DatabaseConnection,
    authorizer: Authorizer,
    type: Policy,
    context: AccessContext,
    expected: readonly string[],
): Promise<void> {
    const selected = await database
        .select({ id: item.id })
        .from(item)
        .where(
            authorizer.where(
                type.permission("edit"),
                await authorizer.resolve(Snapshot.live(database), "personal", context),
            ),
        )
        .orderBy(item.id);
    expect(selected.map((row) => row.id)).toEqual(expected);
}

/** Declare pins reading through the scope types they live in. */
function pinPolicy(scopes: readonly Policy[]): Policy {
    return new Policy(module1.package, {
        name: "pin",
        relations: { scope: { subjects: [...scopes], grantedBy: null, isScope: true } },
        permissions: { read: through("scope", "read") },
    });
}

/** Reference a scope type as a chain link has it. */
function chainLink(type: Policy) {
    return { packageId: type.definition.packageId, type: type.name };
}
