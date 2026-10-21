import { expect, test } from "@destack/test";
import { v7 } from "uuid";
import {
    defineTable,
    Snapshot,
    Transaction,
    eq,
    type DatabaseConnection,
    Table,
    text,
} from "@destack/db";
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
    relation,
    through,
    AccessContext,
    Caller,
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
import { AccessFixture } from "../test/access.ts";

/** The relationships of other subjects the stress test adds to a scope. */
const STRESS_RELATIONSHIPS = 2000;

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

    // challenge the stale caller for the elevation's authentication, and refuse one no authentication allows
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
                    grants: [{ id: Identifier.create("grant"), scope: "carol", permissions: [] }],
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
            grants: [Grant.claim(granted)],
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
            grants: [Grant.claim(forBob)],
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

databaseTest(
    "act for a delegator under its one grant to a delegate, replaced by a later grant, refused once lapsed and for a chain link without a grant",
    async ({ fixture }) => {
        const { database, authorizer, alice } = fixture;
        const assistant = principal.installation.reference("personal", "assistant");
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

        // grant the assistant editing a, then editing b until 2000 in its place
        const grant = (objectId: string) =>
            Authorization.run(
                authorizer,
                database,
                () => alice,
                (authorization) =>
                    Grant.create(
                        scopeOf("alice"),
                        { delegate: assistant, permissions: [editing(objectId)], expiresAt: 2000 },
                        authorization,
                    ),
            );
        await grant("a");
        const later = await grant("b");

        // act for alice under the one live grant, under none once it lapsed, and not through a sub-agent the assistant never granted
        const parties = { delegator: alice.subject, delegate: assistant };
        const caller = await Grant.act(Snapshot.live(database), parties, 1000);
        const grants = await database.select().from(accessGrant);
        const subagent = principal.installation.reference("personal", "subagent");
        const chain = { subject: alice.subject, actors: [assistant, subagent] };
        expect({
            grants: grants.map((row) => row.id),
            caller: [caller.subject, caller.actors, caller.grants],
            lapsed: await outcome(Grant.act(Snapshot.live(database), parties, 2000)),
            unlinked: await outcome(Grant.chain(Snapshot.live(database), chain, 1000)),
        }).toEqual({
            grants: [later.id],
            caller: [alice.subject, [assistant], [Grant.claim(later)]],
            lapsed: "FORBIDDEN: installation assistant acts for user alice under no grant",
            unlinked:
                "FORBIDDEN: installation subagent acts for installation assistant under no grant",
        });
    },
);

databaseTest(
    "give a chain what every live grant of it gives, approval-mode permissions only to a call applying an approved change",
    async ({ fixture }) => {
        // let alice's agent edit a and b, share once approved, and its sub-agent edit b and c
        const { database, alice } = fixture;
        const agent = principal.installation.reference("personal", "agent");
        const subagent = principal.installation.reference("personal", "subagent");
        const sharing = {
            ...node.permission("share"),
            scope: "personal",
            mode: "approval" as const,
        };
        const access = new AccessFixture(database);
        const toAgent = await access.replicateGrant(
            alice.subject,
            agent,
            [editing("a"), editing("b"), sharing],
            1,
        );
        const toSubagent = await access.replicateGrant(
            agent,
            subagent,
            [editing("b"), editing("c")],
            1,
        );
        const caller = (approval?: string) =>
            AccessContext.of(
                {
                    ...Caller.of(alice.subject),
                    actors: [agent, subagent],
                    grants: [
                        { id: toAgent, scope: alice.subject.id },
                        { id: toSubagent, scope: agent.id },
                    ],
                },
                1000,
                { approval },
            );
        const agentAlone = AccessContext.of(
            {
                ...Caller.of(alice.subject),
                actors: [agent],
                grants: [{ id: toAgent, scope: alice.subject.id }],
            },
            1000,
            { approval: "change-request-019f5530-8000-7000-8000-0000000000c5" },
        );

        // give the sub-agent what both give, and the agent its sharing once applying an approved change
        const snapshot = Snapshot.live(database);
        expect([
            await Grant.permissions(snapshot, caller()),
            await Grant.permissions(snapshot, agentAlone),
        ]).toEqual([
            [editing("b")],
            [editing("a"), editing("b"), { ...node.permission("share"), scope: "personal" }],
        ]);
    },
);

databaseTest(
    "resolve a caller in the same statements however many other subjects' relationships the scope has",
    async ({ fixture }) => {
        const { database, authorizer, bob } = fixture;
        const resolve = async () => {
            const before = database.driver.state.statements;
            const access = await authorizer.resolve(Snapshot.live(database), "personal", bob);

            return {
                statements: database.driver.state.statements - before,
                grants: [...access.grants.keys()],
            };
        };
        const few = await resolve();

        // relate many other subjects to notes of the scope, then resolve bob again
        await database.insert(accessRelationship).values(
            Array.from({ length: STRESS_RELATIONSHIPS }, (_, index) =>
                Relationship.encode(
                    {
                        id: schema.identifier("relationship").parse(`relationship-${v7()}`),
                        object: node.reference("personal", `stress-${index}`),
                        relation: "editor",
                        subject: userSubject(`stranger-${index}`),
                        createdAt: bob.now,
                        expiresAt: null,
                    },
                    "personal",
                ),
            ),
        );
        const many = await resolve();

        // read the same statements and grants either way
        expect(many).toEqual(few);
    },
);

test("resolve the policy of an identifier by its prefix, refusing a prefix of no type or of several", () => {
    // read the cell and entity policies by identifiers of theirs
    const authorizer = new Authorizer(policies, mappings);
    const found = [Identifier.create("cell"), Identifier.create("entity")];
    const unknown = Identifier.create("sloth");
    const shared = Identifier.create("group");

    // find each identifier's type, and refuse one of no registered type and one two packages share
    expect(found.map((id) => authorizer.policyOf(id))).toEqual([cell, entity]);
    expect(() => authorizer.policyOf(unknown)).toThrow(
        new AccessError("INVALID_DECLARATION", `no known type has identifiers like ${unknown}`),
    );
    expect(() => authorizer.policyOf(shared)).toThrow(
        new AccessError(
            "INVALID_DECLARATION",
            `several known types have identifiers like ${shared}`,
        ),
    );
});

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

test("refuse binding a role to a scope relation a field decides", () => {
    // declare workspaces with tuple members and a lead stored in a column
    const workspace = new Policy(module1.package, {
        name: "workspace",
        relations: { member: { subjects: [principal.user] }, lead: { subjects: [principal.user] } },
        permissions: { join: relation("member"), lead: relation("lead") },
        scope: true,
    });
    const table = defineTable("example_workspace", {
        id: text("id").primaryKey(),
        scope: text("scope").notNull(),
        lead: text("lead").notNull(),
    });
    const authorizer = new Authorizer(
        [workspace],
        [
            {
                policy: workspace,
                table,
                id: "id",
                scope: "scope",
                attributes: {},
                relations: { lead: { column: "lead", scope: "universe" } },
            },
        ],
    );

    // bind to the members, and refuse the lead
    const bind = (name: string) => () =>
        authorizer.validate(
            {
                object: workspace.reference("universe", "w"),
                role: "role-1",
                subject: { ...workspace.reference("universe", "w"), relation: name },
            },
            0,
        );
    expect(bind("member")).not.toThrow();
    expect(bind("lead")).toThrow(
        new AccessError(
            "FORBIDDEN",
            "a role cannot bind to workspace#lead: roles bind to subject sets and scope relations",
        ),
    );
});

databaseTest(
    "refuse sharing with a group outside the object's scope chain",
    async ({ fixture }) => {
        const { database, authorizer, alice } = fixture;
        const share = (scope: string) =>
            Authorization.run(
                authorizer,
                database,
                () => alice,
                (authorization) =>
                    Relationship.grant(
                        {
                            object: node.reference("personal", "c"),
                            relation: "viewer",
                            subject: { ...group.reference(scope, "g"), relation: "member" },
                        },
                        authorization,
                    ),
            );

        // share with a group of the node's scope, refusing another account's, also as a first subject
        await share("personal");
        const created = Authorization.run(
            authorizer,
            database,
            () => alice,
            (authorization) =>
                authorization.create(node.reference("personal", "z"), {
                    relationships: [
                        {
                            relation: "viewer",
                            subject: { ...group.reference("account-2", "g"), relation: "member" },
                        },
                    ],
                }),
        );
        const refused = new AccessError(
            "FORBIDDEN",
            "group#member in account-2 is outside the scope chain of personal",
        );
        await expect(share("account-2")).rejects.toEqual(refused);
        await expect(created).rejects.toEqual(refused);
    },
);

test("refuse universe subject sets that nest without end", () => {
    // declare teams kept at the universe whose members may be teams
    const team = new Policy(module1.package, {
        name: "crew",
        relations: { member: { subjects: [principal.user, "crew#member"] } },
        permissions: { join: relation("member") },
    });
    const table = defineTable("example_crew", { id: text("id").primaryKey() });
    const declare = () =>
        new Authorizer([team], [{ policy: team, table, id: "id", attributes: {}, relations: {} }]);

    // refuse the declaration, naming the cycle
    expect(declare).toThrow(
        new AccessError(
            "INVALID_DECLARATION",
            "universe subject sets nest without end: crew -> crew",
        ),
    );
});
