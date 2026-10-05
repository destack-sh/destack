import { aligned, schema } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import type { Subject } from "@destack/sync";
import { Snapshot, and, asc, eq, sql } from "@destack/db";
import { TEST_DIALECTS } from "@destack/db/test";
import {
    Relationship,
    accessRelationship,
    accessRole,
    accessRolePermission,
    anyone,
    Authorizer,
    principal,
    type AccessContext,
    type PermissionReference,
    type RelationshipCondition,
} from "../index.ts";
import {
    cell,
    entity,
    group,
    item,
    mappings,
    node,
    policies,
    team,
    teamTable,
} from "../test/fixture.ts";
import { openFixture } from "../test/database.ts";
import { GrantTree } from "./grant.ts";
import { relationship } from "../declare/principal.ts";

/** The number of random access models each dialect checks. */
const MODELS = 6;

/** The time every generated request happens at. */
const NOW = 1000;

/** Draw numbers from a seeded sequence. */
function random(seed: number) {
    let state = seed;

    return () => {
        state = (state + 0x6d2b79f5) | 0;
        let value = Math.imul(state ^ (state >>> 15), 1 | state);
        value = (value + Math.imul(value ^ (value >>> 7), 61 | value)) ^ value;

        return ((value ^ (value >>> 14)) >>> 0) / 4294967296;
    };
}

/** Reference a user in the universe. */
function user(id: string): Subject {
    return principal.user.reference("universe", id);
}

/** Keep a relationship's expiry after its creation, as the table requires. */
function lifetime(createdAt: number, expiresAt: number | null) {
    return {
        createdAt,
        expiresAt: expiresAt !== null && expiresAt <= createdAt ? null : expiresAt,
    };
}

test.for(TEST_DIALECTS)(
    "decide in memory exactly as the permission queries do, for objects and the relationships they share, for random access models on %s",
    async (dialect) => {
        const fixture = await openFixture(dialect);
        onTestFinished(() => fixture.close());
        const { database } = fixture;
        const snapshot = Snapshot.live(database);

        // cover roles on ancestors through the tree, as well as without it
        const authorizers = [
            fixture.authorizer,
            new Authorizer(
                policies,
                mappings.map((mapping) =>
                    mapping.policy === node ? { ...mapping, parent: "parent" } : mapping,
                ),
            ),
        ];

        // define a role that reads and edits nodes and reads cells across types
        const roleId = schema.identifier("role").parse("role-01996ab0-0000-7000-8000-000000000101");
        await database.insert(accessRole).values({
            id: roleId,
            createdAt: 1,
            updatedAt: 1,
            scope: "personal",
            name: "editor",
            description: "Edits nodes",
        });
        const granted = [node.permission("read"), node.permission("edit"), cell.permission("read")];
        for (const [position, permission] of granted.entries()) {
            await database.insert(accessRolePermission).values({
                id: schema
                    .identifier("role-permission")
                    .parse(`role-permission-01996ab0-0000-7000-8000-00000000020${position}`),
                roleId,
                scope: "personal",
                ...permission,
            });
        }
        await database.insert(teamTable).values({ id: "t1", scope: "personal", member: "bob" });

        // relate subjects through every relation shape and condition
        const subjects: Subject[] = [
            user("bob"),
            user("carol"),
            { ...principal.user.reference("*", "*") },
            { ...group.reference("personal", "g1"), relation: "member" },
            { ...team.reference("personal", "t1"), relation: "member" },
            anyone.reference("*", "*"),
        ];
        const conditions: RelationshipCondition[] = [
            {},
            {},
            { linkSecret: "c".repeat(64) },
            { assurance: 2 },
            { maxAge: 100 },
            { request: "request-1" },
            { session: "session-1" },
            { onBehalfOf: user("bob") },
        ];

        // call with the request attributes the policies' conditions read
        const attributes = {
            "first-row": 1,
            "last-row": 2,
            "first-column": 1,
            "last-column": 2,
            team: 1,
            phase: "edit",
        };

        // call as users, members, anonymous callers, presenters of link secrets and delegates
        const agent = principal.installation.reference("personal", "agent");
        const contexts: AccessContext[] = [
            { subjects: [user("alice")], now: NOW, attributes },
            { subjects: [user("bob")], now: NOW, attributes },
            { subjects: [user("carol")], now: NOW, attributes },
            { subjects: [], now: NOW, attributes },
            { subjects: [], now: NOW, attributes, linkSecrets: ["c".repeat(64)] },
            {
                subjects: [user("bob")],
                now: NOW,
                attributes,
                assurance: { level: 2, authenticatedAt: NOW - 50 },
                request: "request-1",
                session: "session-1",
            },
            {
                subjects: [user("bob")],
                subject: user("bob"),
                delegates: [{ subject: agent, authority: "lent" }],
                now: NOW,
                attributes,
            },
            {
                subjects: [user("bob")],
                now: NOW,
                attributes,
                permissions: [{ ...node.permission("read"), scope: "personal", objectId: "b" }],
            },
        ];
        const permissions: PermissionReference[] = [
            ...(["read", "edit", "share", "edit-descendant"] as const).map((name) =>
                node.permission(name),
            ),
            cell.permission("read"),
            cell.permission("edit"),
            entity.permission("edit"),
        ];

        // compare each random model's grants with the permission queries, counting the admissions
        let admissions = 0;
        let shared = 0;
        for (let model = 0; model < MODELS; model++) {
            const draw = random(model + 1);
            const pick = <Value>(values: readonly Value[]) =>
                aligned(values, Math.floor(draw() * values.length));
            await database.delete(accessRelationship);

            // relate random subjects to random nodes, groups and roles, skipping repeated tuples
            for (let position = 0; position < 10; position++) {
                const isBinding = draw() < 0.25;
                const isMembership = !isBinding && draw() < 0.2;
                await database
                    .insert(accessRelationship)
                    .values(
                        Relationship.encode(
                            {
                                id: `relationship-01996ab0-0000-7000-8000-0000000003${String(position).padStart(2, "0")}`,
                                object: isMembership
                                    ? group.reference("personal", "g1")
                                    : node.reference("personal", pick(["a", "b", "c"])),
                                ...(isBinding
                                    ? { role: roleId }
                                    : {
                                          relation: isMembership
                                              ? "member"
                                              : pick(["viewer", "editor", "subtree-editor"]),
                                      }),
                                subject: isMembership
                                    ? pick([user("bob"), user("carol"), agent])
                                    : pick(subjects),
                                ...lifetime(
                                    pick([1, 1, 1, NOW + 1]),
                                    pick([null, null, NOW, NOW + 1000]),
                                ),
                                ...(isMembership ? {} : { conditions: pick(conditions) }),
                            },
                            "personal",
                        ),
                    )
                    .onConflictDoNothing();
            }

            // admit through grants what the queries admit, for each caller at once, permission and authorizer
            for (const authorizer of authorizers) {
                const counts = await Promise.all(
                    contexts.map(async (context) => {
                        let count = 0;
                        const access = await authorizer.resolve(snapshot, "personal", context);
                        const rows = await database
                            .select()
                            .from(item)
                            .where(eq(item.scope, "personal"))
                            .orderBy(asc(item.id));
                        const reader = authorizer.reader(snapshot, access.scopes);
                        for (const permission of permissions) {
                            const queried = (
                                await database
                                    .select({ id: item.id })
                                    .from(item)
                                    .where(authorizer.where(permission, access))
                                    .orderBy(asc(item.id))
                            ).map((row) => row.id);
                            const { permitted } = await authorizer.checkRows(
                                snapshot,
                                permission,
                                access,
                                rows,
                                reader,
                            );
                            const admitted = rows
                                .filter((_row, position) => permitted.has(position))
                                .map((row) => row.id);

                            count += admitted.length;
                            expect({ model, context, permission, admitted }).toEqual({
                                model,
                                context,
                                permission,
                                admitted: queried,
                            });

                            // decide each object by key in memory as the listing query admits it, one node permission per model
                            if (dialect === "sqlite" && permission === permissions[model % 4]) {
                                const decided = [];
                                for (const row of rows) {
                                    const target = node.reference("personal", row.id);
                                    const decision = await authorizer.check(
                                        snapshot,
                                        permission,
                                        target,
                                        access,
                                        reader,
                                    );
                                    if (decision.isAllowed) {
                                        decided.push(row.id);
                                    }

                                    // decide a permission of another type from roles bound on or above the node
                                    const across = cell.permission("read");
                                    const crossed = await authorizer.check(
                                        snapshot,
                                        across,
                                        target,
                                        access,
                                        reader,
                                    );
                                    const permitting = aligned(
                                        await database.execute(
                                            sql`SELECT CASE WHEN ${authorizer.permits(across, target, access)} THEN 1 ELSE 0 END AS permitted`,
                                            schema.object({
                                                permitted: schema.union([
                                                    schema.number(),
                                                    schema.string(),
                                                    schema.bigint(),
                                                ]),
                                            }),
                                        ),
                                        0,
                                    );
                                    expect({
                                        model,
                                        context,
                                        row: row.id,
                                        across: crossed.isAllowed,
                                    }).toEqual({
                                        model,
                                        context,
                                        row: row.id,
                                        across: Number(permitting.permitted) === 1,
                                    });

                                    // keep each decision until the moment it names, as the query decides it just before then
                                    if (decision.until !== undefined) {
                                        const before = { ...context, now: decision.until - 1 };
                                        const later = await authorizer.resolve(
                                            snapshot,
                                            "personal",
                                            before,
                                        );
                                        const [earlier] = await database
                                            .select({ id: item.id })
                                            .from(item)
                                            .where(
                                                and(
                                                    eq(item.id, row.id),
                                                    authorizer.where(permission, later),
                                                ),
                                            );
                                        expect({
                                            model,
                                            context,
                                            permission,
                                            row: row.id,
                                            until: earlier !== undefined,
                                        }).toEqual({
                                            model,
                                            context,
                                            permission,
                                            row: row.id,
                                            until: decision.isAllowed,
                                        });
                                    }
                                }
                                expect({ model, context, permission, decided }).toEqual({
                                    model,
                                    context,
                                    permission,
                                    decided: queried,
                                });
                            }
                        }

                        // decide reading the relationships in memory as their query does, through what each shares
                        const read = relationship.permission("read");
                        const related = await database
                            .select()
                            .from(accessRelationship)
                            .orderBy(asc(accessRelationship.id));
                        const queriedRelationships = (
                            await database
                                .select({ id: accessRelationship.id })
                                .from(accessRelationship)
                                .where(authorizer.where(read, access, accessRelationship))
                                .orderBy(asc(accessRelationship.id))
                        ).map((row) => row.id);
                        const readable = await authorizer.checkRows(
                            snapshot,
                            read,
                            access,
                            related,
                            reader,
                        );
                        expect({
                            model,
                            context,
                            relationships: related
                                .filter((_row, position) => readable.permitted.has(position))
                                .map((row) => row.id),
                        }).toEqual({ model, context, relationships: queriedRelationships });

                        return { count, shared: readable.permitted.size };
                    }),
                );
                admissions += counts.reduce((total, counted) => total + counted.count, 0);
                shared += counts.reduce((total, counted) => total + counted.shared, 0);
            }
        }

        // admit enough rows that the models exercise the grants
        expect([admissions > 100, shared > 0]).toEqual([true, true]);
    },
);

test("read an intersection's grants as the superset its decisions change with", async () => {
    const fixture = await openFixture();
    onTestFinished(() => fixture.close());

    // read the owner each cell's edit permission reaches
    const { database, authorizer } = fixture;
    const rows = await database.select().from(item).orderBy(asc(item.id));
    const trees = await authorizer
        .reader(Snapshot.live(database), [])
        .trees(cell.permission("edit"), rows);
    expect(trees.map((tree) => GrantTree.flatten(tree).map((grant) => grant.subject.id))).toEqual([
        ["alice"],
        ["alice"],
        ["alice"],
        ["alice"],
    ]);
});

test("explain which grants admit a caller and why the others fail", async () => {
    const fixture = await openFixture();
    onTestFinished(() => fixture.close());
    const { database, authorizer } = fixture;

    // share a node with a link only its secret opens
    const linkSecret = "d".repeat(64);
    await database.insert(accessRelationship).values(
        Relationship.encode(
            {
                id: "relationship-01996ab0-0000-7000-8000-000000000401",
                object: node.reference("personal", "c"),
                relation: "viewer",
                subject: anyone.reference("*", "*"),
                createdAt: 1,
                expiresAt: null,
                conditions: { linkSecret },
            },
            "personal",
        ),
    );

    // explain reading the node for a visitor without and with the link secret
    const explain = async (context: AccessContext) => {
        const access = await authorizer.resolve(Snapshot.live(database), "personal", context);
        const explanation = await authorizer.explain(
            Snapshot.live(database),
            node.permission("read"),
            node.reference("personal", "c"),
            access,
        );

        return [
            explanation.isAllowed,
            explanation.authorities.map((authority) =>
                authority.grants.map((grant) => [
                    grant.path.join(" / "),
                    grant.failure ?? "permits",
                ]),
            ),
        ];
    };
    const visitor: AccessContext = { subjects: [], now: NOW, attributes: {} };
    expect(await explain(visitor)).toEqual([
        false,
        [
            [
                ["node read / node edit / field owner holding relation owner", "subject"],
                [
                    "node read / node edit / through parent to node b / node edit-descendant / field owner holding relation owner",
                    "subject",
                ],
                [
                    "node read / node edit / through parent to node a / node edit-descendant / field owner holding relation owner",
                    "subject",
                ],
                ["node read / relation viewer", "linkSecret"],
            ],
        ],
    ]);
    expect((await explain({ ...visitor, linkSecrets: [linkSecret] }))[0]).toBe(true);
});
