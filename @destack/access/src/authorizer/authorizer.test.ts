import { expect, test } from "@destack/test";
import { eq, type DatabaseConnection } from "@destack/db";
import { identifier } from "@destack/schema";
import {
    accessRelationship,
    Authorization,
    Authorizer,
    Capability,
    Policy,
    principal,
    type AccessContext,
    type RelationshipCondition,
} from "../index.ts";
import { cell, entity, item, mappings, node, policies, rows } from "../test/fixture.ts";
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
    selections.find((mapping) => mapping.policy === node)!.relations.owner = {
        column: "parent",
        scope: "other",
    };
    selections.find((mapping) => mapping.policy === cell)!.attributes.row = "column";

    expect(
        await database
            .select({ id: item.id })
            .from(item)
            .where(
                query.where(
                    node.permission("edit"),
                    await query.resolve(database, "personal", bob),
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
    const contexts = [
        bob,
        {
            ...alice,
            attributes: {
                "first-row": 1,
                "last-row": 3,
                "first-column": 1,
                "last-column": 1,
            },
        },
        {
            ...bob,
            attributes: { team: 1, phase: "edit" },
            assurance: { level: 2, authenticatedAt: bob.now },
        },
    ];
    const expected = [["b"], ["a", "b"], ["a"]];
    for (const [position, type] of [node, cell, entity].entries()) {
        const context = contexts[position];
        const sql = await database
            .select({ id: item.id })
            .from(item)
            .where(
                authorizer.where(
                    type.permission("edit"),
                    await authorizer.resolve(database, "personal", context),
                ),
            )
            .orderBy(item.id);
        expect(sql.map((row) => row.id)).toEqual(expected[position]);
    }

    // apply the elevated edit permission only after recent strong authentication
    const stale = {
        ...contexts[2]!,
        assurance: { level: 2, authenticatedAt: bob.now - 3_600_000 },
    };
    expect(
        await database
            .select({ id: item.id })
            .from(item)
            .where(
                authorizer.where(
                    entity.permission("edit"),
                    await authorizer.resolve(database, "personal", stale),
                ),
            ),
    ).toEqual([]);
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
            subjects: [principal.user.reference("global", "carol")],
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
                            await authorizer.resolve(database, "personal", context),
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
        const visitor = { ...anonymous, capabilities: [await Capability.digest(shared.secret)] };
        await expectEditable(database, authorizer, node, visitor, ["b", "c"]);
        await expectEditable(database, authorizer, node, { ...visitor, now: 2000 }, []);
        const stranger = {
            ...anonymous,
            capabilities: [(await Capability.create()).digest],
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
        ).rejects.toMatchObject({ code: "CONFLICT", message: "relationship already exists" });
    },
);

databaseTest(
    "apply proposed relationships only once a grantor or the addressee accepts",
    async ({ fixture }) => {
        const { database, authorizer, alice, bob } = fixture;
        const carol: AccessContext = {
            ...alice,
            subjects: [principal.user.reference("global", "carol")],
            identifiers: ["email:carol@example.com"],
        };
        const dave: AccessContext = {
            ...alice,
            subjects: [principal.user.reference("global", "dave")],
        };

        // let carol ask for access without it applying until the owner accepts
        const asked = await new Authorization(authorizer, database, () => carol).propose({
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
                await new Authorization(authorizer, database, () => alice).proposals(
                    { object: node.reference("personal", "c") },
                    {
                        limit: 10,
                    },
                )
            ).map((proposal) => [proposal.id, proposal.purpose]),
        ).toEqual([[asked.id, "review the draft"]]);
        expect(
            (
                await new Authorization(authorizer, database, () => carol).addressed("personal", {
                    limit: 10,
                })
            ).map((proposal) => proposal.id),
        ).toEqual([asked.id]);

        // hide the proposal once it lapses, and refuse unbounded pages
        expect(
            await new Authorization(authorizer, database, () => ({
                ...carol,
                now: asked.expiresAt,
            })).addressed("personal", { limit: 10 }),
        ).toEqual([]);
        await expect(
            new Authorization(authorizer, database, () => carol).addressed("personal", {
                limit: 101,
            }),
        ).rejects.toMatchObject({
            code: "INVALID_CONTEXT",
            message: "proposal page limit must be 1 to 100",
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
            new Authorization(authorizer, database, () => carol).proposals(
                { object: node.reference("personal", "c") },
                { limit: 10 },
            ),
        ).rejects.toMatchObject({ code: "FORBIDDEN", message: "permission denied: share" });
        await new Authorization(authorizer, database, () => alice).accept(
            asked.relationship.object,
            asked.id,
        );
        await expectEditable(database, authorizer, node, carol, ["c"]);

        // offer access to an email address and bind it to whoever proves it
        const offered = await new Authorization(authorizer, database, () => alice).propose({
            relationship: { object: node.reference("personal", "a"), relation: "editor" },
            recipient: "email:carol@example.com",
        });
        await expect(
            new Authorization(authorizer, database, () => dave).propose({
                relationship: { object: node.reference("personal", "a"), relation: "editor" },
                recipient: "email:dave@example.com",
            }),
        ).rejects.toMatchObject({ code: "FORBIDDEN", message: "permission denied: share" });
        await expect(
            new Authorization(authorizer, database, () => dave).accept(
                offered.relationship.object,
                offered.id,
            ),
        ).rejects.toMatchObject({
            code: "FORBIDDEN",
            message: "proposal is addressed to someone else",
        });
        const agent = principal.installation.reference("personal", "assistant");
        await expect(
            new Authorization(authorizer, database, () => ({
                ...carol,
                subject: carol.subjects[0],
                delegates: [{ subject: agent, authority: "lent" }],
            })).accept(offered.relationship.object, offered.id),
        ).rejects.toMatchObject({
            code: "FORBIDDEN",
            message: "proposal is addressed to someone else",
        });
        await new Authorization(authorizer, database, () => carol).accept(
            offered.relationship.object,
            offered.id,
        );
        await expectEditable(database, authorizer, node, carol, ["a", "c"]);

        // let the addressee refuse an offer and a grantor reject a request
        const refused = await new Authorization(authorizer, database, () => alice).propose({
            relationship: {
                object: node.reference("work", "d"),
                relation: "editor",
                subject: userSubject("bob"),
            },
        });
        await new Authorization(authorizer, database, () => bob).decline(
            refused.relationship.object,
            refused.id,
        );
        const rejected = await new Authorization(authorizer, database, () => dave).propose({
            relationship: {
                object: node.reference("work", "d"),
                relation: "editor",
                subject: userSubject("dave"),
            },
        });
        await expect(
            new Authorization(authorizer, database, () => carol).decline(
                rejected.relationship.object,
                rejected.id,
            ),
        ).rejects.toMatchObject({
            code: "FORBIDDEN",
            message: "permission denied: share",
        });
        await new Authorization(authorizer, database, () => alice).decline(
            rejected.relationship.object,
            rejected.id,
        );
        await expectEditable(database, authorizer, node, dave, []);
        expect(
            await new Authorization(authorizer, database, () => dave).addressed("personal", {
                limit: 10,
            }),
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
                subject: principal.user.reference("global", "carol"),
                conditions,
            });
        const carol: AccessContext = {
            ...bob,
            subjects: [principal.user.reference("global", "carol")],
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
            await readable({ ...carol, assurance: { level: 1, authenticatedAt: 1000 } }),
        ).toEqual([]);
        expect(await readable({ ...carol, assurance: { level: 2, authenticatedAt: 800 } })).toEqual(
            ["c"],
        );
        expect(await readable({ ...carol, assurance: { level: 3, authenticatedAt: 600 } })).toEqual(
            [],
        );

        // remove the consumed request grant
        expect(
            await database
                .select()
                .from(accessRelationship)
                .where(eq(accessRelationship.id, identifier("relationship").parse(once.id))),
        ).toEqual([]);
    },
);

databaseTest("constrain delegated access and credential selections", async ({ fixture }) => {
    const { database, authorizer, alice, bob } = fixture;
    // constrain an agent to the intersection of its user's rights and what the user lent it
    const assistant = principal.installation.reference("personal", "assistant");
    const delegated: AccessContext = {
        ...alice,
        subject: alice.subjects[0],
        delegates: [{ subject: assistant, authority: "lent" }],
    };

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
        conditions: { onBehalfOf: alice.subjects[0]! },
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
    ).rejects.toMatchObject({ code: "FORBIDDEN", message: "a delegate must be one principal" });
    await expect(
        new Authorization(authorizer, database, () => bob).grant({
            ...lending,
            object: node.reference("personal", "b"),
            conditions: { onBehalfOf: bob.subjects[0]! },
        }),
    ).rejects.toMatchObject({ code: "FORBIDDEN", message: "permission denied: share" });
    await new Authorization(authorizer, database, () => alice).grant(lending);
    await expectEditable(database, authorizer, node, delegated, ["b"]);

    // act with alice's whole authority while impersonating her, and through what she lent her agent
    const impersonated: AccessContext = {
        ...alice,
        subject: alice.subjects[0],
        delegates: [{ subject: userSubject("support"), authority: "full" }],
    };
    await expectEditable(database, authorizer, node, alice, ["a", "b", "c"]);
    await expectEditable(database, authorizer, node, impersonated, ["a", "b", "c"]);
    await expectEditable(
        database,
        authorizer,
        node,
        { ...impersonated, delegates: [...impersonated.delegates!, ...delegated.delegates!] },
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
                    await authorizer.resolve(database, "personal", delegated),
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
                    await authorizer.resolve(database, "personal", delegated),
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
                    await authorizer.resolve(database, "personal", {
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
        ...rows[position % 3]!,
        id: `copy-${position}`,
    }));
    const access = await authorizer.resolve(database, "personal", {
        ...alice,
        attributes: { "first-row": 1, "last-row": 3, "first-column": 1, "last-column": 1 },
    });
    const permitted = await authorizer.checkRows(database, cell.permission("edit"), access, copies);

    // permit editing every copy of the two unlocked cells alice owns, which only statements decide
    expect(permitted.held.size).toBe(copies.filter((copy) => copy.locked === 0).length);
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
                    await authorizer.resolve(database, "personal", context),
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
                await authorizer.resolve(database, "personal", context),
            ),
        )
        .orderBy(item.id);
    expect(selected.map((row) => row.id)).toEqual(expected);
}
