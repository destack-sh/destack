import { expect, test } from "@destack/test";
import { Snapshot, eq, sql, type DatabaseConnection } from "@destack/db";
import { aligned, schema } from "@destack/schema";
import {
    AccessError,
    accessRelationship,
    Authorization,
    Contact,
    Authorizer,
    LinkSecret,
    resource,
    Policy,
    principal,
    relation,
    type AccessContext,
    type AccessExpression,
    type RelationshipCondition,
} from "../index.ts";
import { cell, entity, item, mappings, module4, node, policies, rows } from "../test/fixture.ts";
import { openFixture, userSubject } from "../test/database.ts";

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
        new Authorization(authorizer, database, () => bob).grant({
            object: node.reference("personal", "b"),
            relation: "editor",
            subject: userSubject("alice"),
        }),
    ).rejects.toMatchObject({
        code: "FORBIDDEN",
        message: "permission denied: share",
    });

    // select each example's editable rows for its caller
    const elevated = {
        ...bob,
        attributes: { team: 1, phase: "edit" },
        assurance: { level: 2, authenticatedAt: bob.now },
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
        assurance: { level: 2, authenticatedAt: bob.now - 3_600_000 },
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
            "authenticate again at the required assurance",
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
        await new Authorization(authorizer, database, () => alice).grant({
            object: node.reference("personal", "a"),
            relation: "viewer",
            subject: principal.user.reference("*", "*"),
        });
        const carol = {
            ...alice,
            subjects: [principal.user.reference("universe", "carol")],
        };
        const anonymous = { ...alice, subjects: [] };
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
        const shared = await new Authorization(authorizer, database, () => alice).link({
            object: node.reference("personal", "b"),
            relation: "subtree-editor",
            expiresAt: 2000,
        });

        // let anyone presenting the link edit the subtree until it expires
        const visitor = {
            ...anonymous,
            linkSecrets: [await LinkSecret.digest(shared.secret)],
        };
        await expectEditable(database, authorizer, node, visitor, ["b", "c"]);
        await expectEditable(database, authorizer, node, { ...visitor, now: 2000 }, []);
        const stranger = {
            ...anonymous,
            linkSecrets: [(await LinkSecret.create()).digest],
        };
        await expectEditable(database, authorizer, node, stranger, []);

        // revoke the link by deleting its relationship
        await new Authorization(authorizer, database, () => alice).revoke(
            node.reference("personal", "b"),
            shared.id,
        );
        await expectEditable(database, authorizer, node, visitor, []);

        // create a second link beside another, and refuse writing an existing relationship again
        const first = await new Authorization(authorizer, database, () => alice).link({
            object: node.reference("personal", "a"),
            relation: "viewer",
        });
        const second = await new Authorization(authorizer, database, () => alice).link({
            object: node.reference("personal", "a"),
            relation: "viewer",
        });
        expect(second.id).not.toBe(first.id);
        await expect(
            new Authorization(authorizer, database, () => alice).grant({
                object: node.reference("personal", "b"),
                relation: "editor",
                subject: userSubject("bob"),
            }),
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
            subjects: [principal.user.reference("universe", "carol")],
            contacts: [Contact.email("carol@example.com")],
        };
        const dave: AccessContext = {
            ...alice,
            subjects: [principal.user.reference("universe", "dave")],
        };

        // let carol ask for access without it applying until the owner accepts
        const asked = await new Authorization(authorizer, database, () => carol).invite({
            relationship: {
                object: node.reference("personal", "c"),
                relation: "editor",
                subject: userSubject("carol"),
            },
            purpose: "review the draft",
        });
        await expectEditable(database, authorizer, node, carol, []);
        expect(
            (
                await new Authorization(authorizer, database, () => alice).invitations(
                    { object: node.reference("personal", "c") },
                    {
                        limit: 10,
                    },
                )
            ).map((invitation) => [invitation.id, invitation.purpose]),
        ).toEqual([[asked.id, "review the draft"]]);

        // hide the lapsed invitation and refuse unbounded pages
        expect(
            await new Authorization(authorizer, database, () => ({
                ...alice,
                now: asked.expiresAt,
            })).invitations({ object: node.reference("personal", "c") }, { limit: 10 }),
        ).toEqual([]);
        await expect(
            new Authorization(authorizer, database, () => alice).invitations(
                { object: node.reference("personal", "c") },
                { limit: 101 },
            ),
        ).rejects.toMatchObject({
            code: "INVALID_CONTEXT",
            message: "invitation page limit must be 1 to 100",
        });
        await expect(
            new Authorization(authorizer, database, () => carol).accept(
                asked.relationship.object,
                asked.id,
            ),
        ).rejects.toMatchObject({
            code: "FORBIDDEN",
            message: "permission denied: share",
        });
        await expect(
            new Authorization(authorizer, database, () => carol).invitations(
                { object: node.reference("personal", "c") },
                { limit: 10 },
            ),
        ).rejects.toMatchObject({
            code: "FORBIDDEN",
            message: "permission denied: share",
        });
        await new Authorization(authorizer, database, () => alice).accept(
            asked.relationship.object,
            asked.id,
        );
        await expectEditable(database, authorizer, node, carol, ["c"]);

        // offer access to an email address and bind it to whoever proves it
        const offered = await new Authorization(authorizer, database, () => alice).invite({
            relationship: {
                object: node.reference("personal", "a"),
                relation: "editor",
                subject: Contact.subject(Contact.email("carol@example.com")),
            },
        });
        await expect(
            new Authorization(authorizer, database, () => dave).invite({
                relationship: {
                    object: node.reference("personal", "a"),
                    relation: "editor",
                    subject: Contact.subject(Contact.email("dave@example.com")),
                },
            }),
        ).rejects.toMatchObject({
            code: "FORBIDDEN",
            message: "permission denied: share",
        });
        await expect(
            new Authorization(authorizer, database, () => dave).accept(
                offered.relationship.object,
                offered.id,
            ),
        ).rejects.toMatchObject({
            code: "FORBIDDEN",
            message: "invitation is addressed to someone else",
        });
        const agent = principal.installation.reference("personal", "assistant");
        await expect(
            new Authorization(authorizer, database, () => ({
                ...carol,
                subject: userSubject("carol"),
                delegates: [{ subject: agent, authority: "lent" }],
            })).accept(offered.relationship.object, offered.id),
        ).rejects.toMatchObject({
            code: "FORBIDDEN",
            message: "invitation is addressed to someone else",
        });
        await new Authorization(authorizer, database, () => carol).accept(
            offered.relationship.object,
            offered.id,
        );
        await expectEditable(database, authorizer, node, carol, ["a", "c"]);

        // let the addressee refuse an offer and a grantor reject a request
        const refused = await new Authorization(authorizer, database, () => alice).invite({
            relationship: {
                object: node.reference("work", "d"),
                relation: "editor",
                subject: userSubject("bob"),
            },
        });
        await new Authorization(authorizer, database, () => bob).withdraw(
            refused.relationship.object,
            refused.id,
        );
        const rejected = await new Authorization(authorizer, database, () => dave).invite({
            relationship: {
                object: node.reference("work", "d"),
                relation: "editor",
                subject: userSubject("dave"),
            },
        });
        await expect(
            new Authorization(authorizer, database, () => carol).withdraw(
                rejected.relationship.object,
                rejected.id,
            ),
        ).rejects.toMatchObject({
            code: "FORBIDDEN",
            message: "permission denied: share",
        });
        await new Authorization(authorizer, database, () => alice).withdraw(
            rejected.relationship.object,
            rejected.id,
        );
        await expectEditable(database, authorizer, node, dave, []);
        expect(
            await new Authorization(authorizer, database, () => alice).invitations(
                { object: node.reference("work", "d") },
                { limit: 10 },
            ),
        ).toEqual([]);
    },
);

databaseTest(
    "apply grants only under their request, session and authentication conditions",
    async ({ fixture }) => {
        const { database, authorizer, alice, bob } = fixture;
        const grant = async (conditions: RelationshipCondition) =>
            new Authorization(authorizer, database, () => alice).grant({
                object: node.reference("personal", "c"),
                relation: "viewer",
                subject: principal.user.reference("universe", "carol"),
                conditions,
            });
        const carol: AccessContext = {
            ...bob,
            subjects: [principal.user.reference("universe", "carol")],
        };
        const readable = async (context: AccessContext) =>
            readableNodes(database, authorizer, context);

        // bind a grant to one request and one session
        const once = await grant({ request: "request-1" });
        expect(await readable(carol)).toEqual([]);
        expect(await readable({ ...carol, request: "request-1" })).toEqual(["c"]);
        await authorizer.release(database, "request-1");
        expect(await readable({ ...carol, request: "request-1" })).toEqual([]);
        const session = await grant({ session: "session-1" });
        expect(await readable({ ...carol, session: "session-2" })).toEqual([]);
        expect(await readable({ ...carol, session: "session-1" })).toEqual(["c"]);
        await new Authorization(authorizer, database, () => alice).revoke(
            session.object,
            session.id,
        );

        // require strong and recent authentication
        await grant({ assurance: 2, maxAge: 300 });
        expect(await readable(carol)).toEqual([]);
        expect(
            await readable({
                ...carol,
                assurance: { level: 1, authenticatedAt: 1000 },
            }),
        ).toEqual([]);
        expect(
            await readable({
                ...carol,
                assurance: { level: 2, authenticatedAt: 800 },
            }),
        ).toEqual(["c"]);
        expect(
            await readable({
                ...carol,
                assurance: { level: 3, authenticatedAt: 600 },
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

databaseTest("constrain delegated access and credential selections", async ({ fixture }) => {
    const { database, authorizer, alice, bob } = fixture;
    // constrain an agent to the intersection of its user's rights and what the user lent it
    const assistant = principal.installation.reference("personal", "assistant");
    const delegated = {
        ...alice,
        subject: userSubject("alice"),
        delegates: [{ subject: assistant, authority: "lent" }],
    } satisfies AccessContext;

    // ignore the agent's own grants while it acts for someone else
    await new Authorization(authorizer, database, () => alice).grant({
        object: node.reference("personal", "b"),
        relation: "editor",
        subject: assistant,
    });
    await expectEditable(database, authorizer, node, delegated, []);

    // let only the lending principal lend what it may grant, and only to one principal
    const lending = {
        object: node.reference("personal", "b"),
        relation: "editor",
        subject: assistant,
        expiresAt: 2000,
        conditions: { onBehalfOf: userSubject("alice") },
    };
    await expect(
        new Authorization(authorizer, database, () => bob).grant(lending),
    ).rejects.toMatchObject({
        code: "FORBIDDEN",
        message: "only a principal may lend its own authority",
    });
    await expect(
        new Authorization(authorizer, database, () => alice).grant({
            ...lending,
            subject: { ...userSubject("carol"), relation: "member" },
        }),
    ).rejects.toMatchObject({
        code: "FORBIDDEN",
        message: "a delegate must be one principal",
    });
    await expect(
        new Authorization(authorizer, database, () => bob).grant({
            ...lending,
            object: node.reference("personal", "b"),
            conditions: { onBehalfOf: userSubject("bob") },
        }),
    ).rejects.toMatchObject({
        code: "FORBIDDEN",
        message: "permission denied: share",
    });
    await new Authorization(authorizer, database, () => alice).grant(lending);
    await expectEditable(database, authorizer, node, delegated, ["b"]);

    // act with alice's whole authority while impersonating her, and through what she lent her agent
    const impersonated = {
        ...alice,
        subject: userSubject("alice"),
        delegates: [{ subject: userSubject("support"), authority: "full" }],
    } satisfies AccessContext;
    await expectEditable(database, authorizer, node, alice, ["a", "b", "c"]);
    await expectEditable(database, authorizer, node, impersonated, ["a", "b", "c"]);
    await expectEditable(
        database,
        authorizer,
        node,
        {
            ...impersonated,
            delegates: [...impersonated.delegates, ...delegated.delegates],
        },
        ["b"],
    );
    await expectEditable(database, authorizer, node, { ...delegated, now: 2000 }, []);
    expect(
        await database
            .select({ id: item.id })
            .from(item)
            .where(
                authorizer.where(
                    node.permission("edit"),
                    await authorizer.resolve(Snapshot.live(database), "personal", delegated),
                ),
            ),
    ).toEqual([{ id: "b" }]);
    expect(
        await database
            .select({ id: item.id })
            .from(item)
            .where(
                authorizer.where(
                    node.permission("share"),
                    await authorizer.resolve(Snapshot.live(database), "personal", delegated),
                ),
            ),
    ).toEqual([]);
    expect(
        await database
            .select({ id: item.id })
            .from(item)
            .where(
                authorizer.where(
                    node.permission("edit"),
                    await authorizer.resolve(Snapshot.live(database), "personal", {
                        ...delegated,
                        now: 2000,
                    }),
                ),
            ),
    ).toEqual([]);

    // restrict delegated access to the objects a credential selects
    const selected = {
        ...delegated,
        permissions: [{ ...node.permission("edit"), scope: "personal", objectId: "b" }],
    };
    await expectEditable(database, authorizer, node, selected, ["b"]);
    await expectEditable(database, authorizer, node, { ...selected, permissions: [] }, []);
    await expectEditable(
        database,
        authorizer,
        node,
        {
            ...selected,
            permissions: [
                {
                    ...node.permission("edit"),
                    scope: "other",
                    objectId: "b",
                },
            ],
        },
        [],
    );
});

databaseTest("update inherited permissions when notes move or disappear", async ({ fixture }) => {
    const { database, authorizer, alice, bob } = fixture;

    // share the root and its descendants while retaining the direct grant on b
    await new Authorization(authorizer, database, () => alice).grant({
        object: node.reference("personal", "a"),
        relation: "subtree-editor",
        subject: userSubject("bob"),
    });
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

databaseTest("evaluate more rows than one query may bind", async ({ fixture }) => {
    const { database, authorizer, alice } = fixture;

    // present thousands of copies of alice's notes, beyond one query's parameter limit
    const copies = Array.from({ length: 4000 }, (_, position) => ({
        ...aligned(rows, position % 3),
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

test("refuse policy conditions that follow relations when registering them", () => {
    // decide one permission by a condition over a relation
    const fenced = new Policy(module4.package, {
        name: "fenced",
        relations: { owner: { subjects: [principal.user] } },
        permissions: { read: relation("owner"), edit: resource({ owner: {} }) },
    });

    // refuse it before any decision reads it
    expect(() => new Authorizer([fenced], [])).toThrow(
        new AccessError("INVALID_DECLARATION", "policy conditions follow no relations: owner"),
    );
});

test.each<[string, AccessExpression, string]>([
    [
        "undeclared object attribute",
        resource({ size: { eq: 1 } }),
        "undeclared object attribute: size",
    ],
    [
        "undeclared request attribute",
        { kind: "context", condition: { size: { eq: 1 } } },
        "undeclared request attribute: size",
    ],
    [
        "undeclared placeholder",
        resource({ team: { eq: sql.placeholder("size") } }),
        "undeclared request attribute: size",
    ],
    [
        "request placeholder",
        { kind: "context", condition: { phase: { eq: sql.placeholder("phase") } } },
        "request conditions read no placeholders: phase",
    ],
    [
        "ordered text",
        resource({ title: { gt: "a" } }),
        "ordered comparisons require a number attribute: title",
    ],
    ["mistyped value", resource({ team: "one" }), "comparisons require a number value: team"],
    [
        "mistyped list",
        { kind: "context", condition: { phase: { in: ["edit", 1] } } },
        "listed values require string values: phase",
    ],
    ["numeric pattern", resource({ team: { like: "1%" } }), "patterns match text attributes"],
])("refuse a policy condition with an %s when registering it", (_case, edit, message) => {
    // decide one permission by the condition
    const checked = new Policy(module4.package, {
        name: "checked",
        attributes: { team: "number", title: "string" },
        context: { phase: "string" },
        relations: { owner: { subjects: [principal.user] } },
        permissions: { read: relation("owner"), edit },
    });

    // refuse it before any decision reads it
    expect(() => new Authorizer([checked], [])).toThrow(
        new AccessError("INVALID_DECLARATION", message),
    );
});
