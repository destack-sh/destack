import { expect, test } from "@destack/test";
import { PackageId } from "@destack/package/package";
import { AccessError } from "../src/error/index.ts";
import { openFixture } from "./database.ts";
import { eq, type DatabaseConnection } from "@destack/db";
import {
    compare,
    attribute,
    literal,
    permission,
    defineObject,
    relation,
    type Subject,
} from "../src/index.ts";
import {
    Access,
    AccessModel,
    AccessSnapshot,
    type AccessContext,
    type AccessObject,
    type Grant,
    type ObjectType,
    type AccessView,
} from "../src/index.ts";
import { AccessQuery, GrantStore } from "../src/database/index.ts";
import { accessGrant, accessToken } from "../src/stack/index.ts";
import { node, cell, entity, item, mappings, rows } from "./fixture.ts";

/** Provide an isolated database with notes and an explicit editor grant. */
const databaseTest = test.extend<{ fixture: Awaited<ReturnType<typeof openFixture>> }>({
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
    const { database, model, alice, bob, store } = fixture;

    // retain the prepared mapping when the caller changes its input records
    const selections = mappings.map((mapping) => ({
        ...mapping,
        subjects: { ...mapping.subjects },
        attributes: { ...mapping.attributes },
    }));
    const query = new AccessQuery(model, selections);
    selections[0].subjects.owner = { column: "parent", kind: "user", authority: "other" };
    selections[1].attributes.row = "column";

    expect(
        await database
            .select({ id: item.id })
            .from(item)
            .where(query.where(node.permission("edit"), "personal", bob))
            .orderBy(item.id),
    ).toEqual([{ id: "b" }]);
    await expect(
        store.grant(
            {
                object: node.ref("personal", "b"),
                relation: "editor",
                subject: alice.subjects[0],
            },
            bob,
        ),
    ).rejects.toMatchObject({ code: "FORBIDDEN", message: "sharing permission denied" });

    // compare every example's SQL selection with complete memory decisions
    const contexts = [
        bob,
        {
            ...alice,
            attributes: { "first-row": 1, "last-row": 3, "first-column": 1, "last-column": 1 },
        },
        { ...bob, attributes: { team: 1, phase: "edit" } },
    ];
    const expected = [["b"], ["a", "b"], ["a"]];
    const persisted = await database.select().from(accessGrant);
    const grants: Grant[] = persisted.map((grant) => ({
        ...grant,
        object: {
            packageId: grant.packageId,
            type: grant.type,
            scope: grant.scope,
            id: grant.objectId,
        },
        subject: { kind: "user", authority: grant.subjectAuthority, id: grant.subjectId },
    }));
    for (const [position, type] of [node, cell, entity].entries()) {
        const objects: AccessObject[] = rows.map((row) => ({
            reference: type.ref(row.scope, row.id),
            attributes: {
                row: row.row,
                column: row.column,
                locked: row.locked,
                team: row.team,
                protected: row.protected,
            },
            subjects: { owner: [{ kind: "user", authority: "global", id: row.owner }] },
            objects: { parent: row.parent === null ? [] : [type.ref(row.scope, row.parent)] },
        }));
        const access = new Access(model, new AccessSnapshot(1, objects, grants));
        const context = contexts[position];
        const memory = objects
            .filter(
                (object) =>
                    object.reference.scope === "personal" &&
                    access.check(type.permission("edit"), object.reference, context),
            )
            .map((object) => object.reference.id);
        const sql = await database
            .select({ id: item.id })
            .from(item)
            .where(query.where(type.permission("edit"), "personal", context))
            .orderBy(item.id);
        expect({ memory, sql: sql.map((row) => row.id) }).toEqual({
            memory: expected[position],
            sql: expected[position],
        });
    }
});

databaseTest("share notes through expiring and revocable credentials", async ({ fixture }) => {
    const { database, model, query, alice, changes, store } = fixture;
    // exercise anonymous public access, bearer edit access, expiry and revocation
    await store.grant(
        {
            object: node.ref("personal", "a"),
            relation: "viewer",
            subject: { kind: "everyone" },
        },
        alice,
    );
    const anonymous = { ...alice, subjects: [] };
    expect(
        await database
            .select({ id: item.id })
            .from(item)
            .where(query.where(node.permission("read"), "personal", anonymous)),
    ).toEqual([{ id: "a" }]);
    const token = await store.createToken(
        { object: node.ref("personal", "b"), relation: "subtree-editor", expiresAt: 2000 },
        alice,
    );
    const subject = await store.authenticateToken(token.secret, 1000);
    const visitor = { ...anonymous, subjects: [subject] };
    await compareDecisions(database, model, node, visitor, ["b", "c"]);
    await compareDecisions(database, model, node, { ...visitor, now: 2000 }, []);
    expect(
        await database
            .select({ id: item.id })
            .from(item)
            .where(query.where(node.permission("edit"), "personal", visitor))
            .orderBy(item.id),
    ).toEqual([{ id: "b" }, { id: "c" }]);
    expect(
        await database
            .select({ id: item.id })
            .from(item)
            .where(query.where(node.permission("edit"), "personal", { ...visitor, now: 2000 })),
    ).toEqual([]);
    // direct bearer relationships must enforce the same lifetime as bearer grants
    const direct = defineObject({
        packageId: PackageId.parse("package-01996ab0-0000-7000-8000-000000000001"),
        name: "token",
        attributes: {},
        relations: { bearer: { kind: "subject", subjects: ["share-token"] } },
        permissions: { read: relation("bearer") },
    });
    const directModel = new AccessModel([direct]);
    const directQuery = new AccessQuery(directModel, [
        {
            type: direct,
            table: accessToken,
            id: "id",
            scope: "scope",
            attributes: {},
            objects: {},
            subjects: { bearer: { column: "id", kind: "share-token", authority: "personal" } },
        },
    ]);

    // compare complete SQL and memory decisions before expiry and after revocation
    for (const state of ["active", "expired", "revoked"] as const) {
        if (state === "revoked") {
            await store.revokeToken(token.id, alice);
        }
        const current = { ...visitor, now: state === "expired" ? 2000 : 1000 };
        const tokens = await database.select().from(accessToken);
        const reference = direct.ref("personal", token.id);
        const snapshot = new AccessSnapshot(
            state,
            [
                {
                    reference,
                    attributes: {},
                    subjects: { bearer: [subject] },
                    objects: {},
                },
            ],
            [],
            tokens,
        );
        const memory = new Access(directModel, snapshot).check(
            direct.permission("read"),
            reference,
            current,
        );
        const selected = await database
            .select({ id: accessToken.id })
            .from(accessToken)
            .where(directQuery.where(direct.permission("read"), "personal", current));
        expect({ memory, selected }).toEqual({
            memory: state === "active",
            selected: state === "active" ? [{ id: token.id }] : [],
        });
    }
    await compareDecisions(database, model, node, visitor, []);
    expect(
        await database
            .select({ id: item.id })
            .from(item)
            .where(query.where(node.permission("edit"), "personal", visitor)),
    ).toEqual([]);
    await expect(store.authenticateToken(token.secret, 1000)).rejects.toMatchObject({
        code: "FORBIDDEN",
        message: "invalid or expired share credential",
    });

    // reject a transaction when recording its security change fails
    const failing = new GrantStore(database, query, async () => {
        throw new Error("audit unavailable");
    });
    await expect(
        failing.createToken({ object: node.ref("personal", "a"), relation: "viewer" }, alice),
    ).rejects.toThrow("audit unavailable");
    expect((await database.select().from(accessToken)).length).toBe(1);
    expect(changes).toEqual(["grant", "grant", "create-token", "revoke-token"]);
});

databaseTest("constrain delegated access with mandatory policies", async ({ fixture }) => {
    const { database, model, query, alice, bob, store } = fixture;
    // constrain an agent to the intersection of user rights, workload rights and delegation
    const actor: Subject = { kind: "service-account", authority: "personal", id: "assistant" };
    await store.grant(
        { object: node.ref("personal", "b"), relation: "editor", subject: actor },
        alice,
    );
    const delegated: AccessContext = {
        ...bob,
        subject: bob.subjects[0],
        actor,
        delegations: [
            {
                id: "task",
                subject: bob.subjects[0],
                actor,
                createdAt: 1000,
                expiresAt: 2000,
                revokedAt: null,
                permissions: [{ ...node.permission("edit"), scope: "personal", objectId: "b" }],
            },
        ],
    };
    await compareDecisions(database, model, node, delegated, ["b"]);
    await compareDecisions(database, model, node, { ...delegated, now: 2000 }, []);
    expect(
        await database
            .select({ id: item.id })
            .from(item)
            .where(query.where(node.permission("edit"), "personal", delegated)),
    ).toEqual([{ id: "b" }]);
    expect(
        await database
            .select({ id: item.id })
            .from(item)
            .where(query.where(node.permission("share"), "personal", delegated)),
    ).toEqual([]);
    expect(
        await database
            .select({ id: item.id })
            .from(item)
            .where(query.where(node.permission("edit"), "personal", { ...delegated, now: 2000 })),
    ).toEqual([]);

    // apply mandatory restrictions after ordinary grants and delegation
    const restrictedModel = new AccessModel(
        [node, cell, entity],
        [
            {
                name: "freeze-edits",
                packageId: PackageId.parse("package-01996ab0-0000-7000-8000-000000000001"),
                type: "node",
                permissions: ["edit"],
                effect: "forbid",
                condition: compare("eq", attribute("context", "frozen", "boolean"), literal(true)),
            },
        ],
    );
    const restricted = new AccessQuery(restrictedModel, mappings);
    await compareDecisions(
        database,
        restrictedModel,
        node,
        { ...delegated, attributes: { frozen: true } },
        [],
    );
    await compareDecisions(
        database,
        restrictedModel,
        node,
        { ...delegated, attributes: { frozen: false } },
        ["b"],
    );
    expect(
        await database
            .select({ id: item.id })
            .from(item)
            .where(
                restricted.where(node.permission("edit"), "personal", {
                    ...delegated,
                    attributes: { frozen: true },
                }),
            ),
    ).toEqual([]);
    expect(
        await database
            .select({ id: item.id })
            .from(item)
            .where(
                restricted.where(node.permission("edit"), "personal", {
                    ...delegated,
                    attributes: { frozen: false },
                }),
            ),
    ).toEqual([{ id: "b" }]);

    // policies may reuse declared permissions without introducing a permission cycle
    const ownerPolicy = new AccessModel(
        [node, cell, entity],
        [
            {
                name: "owner-only",
                packageId: PackageId.parse("package-01996ab0-0000-7000-8000-000000000001"),
                type: "node",
                permissions: ["edit"],
                effect: "restrict",
                condition: permission("share"),
            },
        ],
    );
    await compareDecisions(database, ownerPolicy, node, bob, []);
    await compareDecisions(database, ownerPolicy, node, alice, ["a", "b", "c"]);

    // restricting edit to its ordinary declaration is valid and preserves both evaluators
    const editPolicy = new AccessModel(
        [node, cell, entity],
        [
            {
                name: "declared-edit",
                packageId: PackageId.parse("package-01996ab0-0000-7000-8000-000000000001"),
                type: "node",
                permissions: ["edit"],
                effect: "restrict",
                condition: permission("edit"),
            },
        ],
    );
    await compareDecisions(database, editPolicy, node, bob, ["b"]);
});

databaseTest("update inherited permissions when notes move or disappear", async ({ fixture }) => {
    const { database, model, alice, bob, store } = fixture;

    // share the root and its descendants while retaining the direct grant on b
    await store.grant(
        { object: node.ref("personal", "a"), relation: "subtree-editor", subject: bob.subjects[0] },
        alice,
    );
    await compareDecisions(database, model, node, bob, ["a", "b", "c"]);

    // detach b: its direct grant remains, but c loses inherited root access
    await database.update(item).set({ parent: null }).where(eq(item.id, "b"));
    await compareDecisions(database, model, node, bob, ["a", "b"]);

    // reattach b and restore access through the same parent relationship
    await database.update(item).set({ parent: "a" }).where(eq(item.id, "b"));
    await compareDecisions(database, model, node, bob, ["a", "b", "c"]);

    // remove the leaf and restrict both evaluators to the remaining protected records
    await database.delete(item).where(eq(item.id, "c"));
    await compareDecisions(database, model, node, bob, ["a", "b"]);
});

/** Reject an authoritative view that returns another object's credential or changes mid-read. */
test("reject inconsistent authorization views", () => {
    // provide an active bearer grant for one protected note
    const reference = node.ref("personal", "note");
    const subject: Subject = { kind: "share-token", authority: "personal", id: "token" };
    const object: AccessObject = {
        reference,
        attributes: {},
        subjects: { owner: [] },
        objects: { parent: [] },
    };
    const grant: Grant = {
        id: "grant",
        object: reference,
        relation: "editor",
        subject,
        createdAt: 1,
        expiresAt: null,
        revokedAt: null,
    };
    const context: AccessContext = { subjects: [subject], now: 2, attributes: {} };

    // require the returned token to match both the requested identifier and scope
    const view: AccessView & { revision: number } = {
        revision: 1,
        object: () => object,
        grants: () => [grant],
        token: () => ({
            id: "token",
            scope: "work",
            digest: "digest",
            createdAt: 1,
            expiresAt: null,
            revokedAt: null,
        }),
    };
    const access = new Access(new AccessModel([node]), view);
    expect(() => access.check(node.permission("edit"), reference, context)).toThrow(
        new AccessError("INVALID_CONTEXT", "read view returned a different share token"),
    );

    // reject a decision spanning two authoritative revisions
    view.token = () => {
        view.revision++;

        return {
            id: "token",
            scope: "personal",
            digest: "digest",
            createdAt: 1,
            expiresAt: null,
            revokedAt: null,
        };
    };
    expect(() => access.check(node.permission("edit"), reference, context)).toThrow(
        new AccessError("CONFLICT", "authorization records changed during evaluation"),
    );
});

/** Reuse one evaluator while a resident application's authoritative world changes. */
test("recheck live world state", () => {
    const object = {
        reference: entity.ref("world", "door"),
        attributes: { team: 1, protected: 0 },
        subjects: {},
        objects: {},
    };
    const world = new Map([[object.reference.id, object]]);
    const view: AccessView & { revision: number } = {
        revision: 1,
        object: (reference) => world.get(reference.id),
        grants: () => [],
        token: () => undefined,
    };
    const access = new Access(new AccessModel([entity]), view);
    const context: AccessContext = {
        subjects: [],
        now: 1000,
        attributes: { team: 1, phase: "edit" },
    };

    // apply phase, team and object restrictions without copying the world
    expect([
        access.check(entity.permission("edit"), object.reference, context),
        access.check(entity.permission("edit"), object.reference, {
            ...context,
            attributes: { team: 1, phase: "play" },
        }),
        access.check(entity.permission("edit"), object.reference, {
            ...context,
            attributes: { team: 2, phase: "edit" },
        }),
    ]).toEqual([true, false, false]);
    object.attributes.protected = 1;
    view.revision++;
    expect(access.explain(entity.permission("edit"), object.reference, context)).toEqual({
        permission: entity.permission("edit"),
        object: object.reference,
        allowed: false,
        grants: [],
        revision: 2,
    });

    // read the next committed revision through the same view and evaluator
    object.attributes.protected = 0;
    view.revision++;
    expect(access.explain(entity.permission("edit"), object.reference, context)).toEqual({
        permission: entity.permission("edit"),
        object: object.reference,
        allowed: true,
        grants: [],
        revision: 3,
    });
});

/** Compare all persisted object decisions with the complete SQL result. */
async function compareDecisions(
    database: DatabaseConnection,
    model: AccessModel,
    type: ObjectType,
    context: AccessContext,
    expected: readonly string[],
): Promise<void> {
    // read authoritative application records and sharing state from the same test database
    const rows = await database.select().from(item).orderBy(item.id);
    const persisted = await database.select().from(accessGrant);
    const tokens = await database.select().from(accessToken);
    const objects = rows.map((row): AccessObject => ({
        reference: type.ref(row.scope, row.id),
        attributes: {
            row: row.row,
            column: row.column,
            locked: row.locked,
            team: row.team,
            protected: row.protected,
        },
        subjects: { owner: [{ kind: "user", authority: "global", id: row.owner }] },
        objects: { parent: row.parent === null ? [] : [type.ref(row.scope, row.parent)] },
    }));
    const grants = persisted.map((grant): Grant => ({
        ...grant,
        object: {
            packageId: grant.packageId,
            type: grant.type,
            scope: grant.scope,
            id: grant.objectId,
        },
        subject:
            grant.subjectKind === "everyone"
                ? { kind: "everyone" }
                : {
                      kind: grant.subjectKind,
                      authority: grant.subjectAuthority,
                      id: grant.subjectId,
                  },
    }));

    // compare separate evaluators against an explicit expected set
    const access = new Access(model, new AccessSnapshot(1, objects, grants, tokens));
    const memory = objects
        .filter(
            (object) =>
                object.reference.scope === "personal" &&
                access.check(type.permission("edit"), object.reference, context),
        )
        .map((object) => object.reference.id);
    const query = new AccessQuery(model, mappings);
    const selected = await database
        .select({ id: item.id })
        .from(item)
        .where(query.where(type.permission("edit"), "personal", context))
        .orderBy(item.id);
    expect({ memory, sql: selected.map((row) => row.id) }).toEqual({
        memory: expected,
        sql: expected,
    });
}
