import type { Environment } from "@destack/environment";
import { testSimulation } from "../src/test/index.ts";
import { users } from "./fixture/user.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { defineDatabase } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { present, schema } from "@destack/schema";
import { subjectContext, testCallKey } from "@destack/object/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { RequestId, Scope, type Subject } from "@destack/sync";
import { relationship } from "../src/index.ts";
import { space, spacePrincipal } from "./fixture/space.ts";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { TestObjectStore } from "../src/test/index.ts";

/** The space whose access the checks decide. */
const SPACE_ID = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000041");

/** Alice, the owner of the space. */
const alice = principal.user.reference(Scope.universe.id, users.alice);
/** Bob, who reads the space. */
const bob = principal.user.reference(Scope.universe.id, users.bob);
/** Carol, who has no access. */
const carol = principal.user.reference(Scope.universe.id, users.carol);

test("check the caller's access under its grant, and another subject's for grantors alone", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const { storage, store, object } = await serveSpace(world);

    // check as alice, as bob, and as a token acting for alice under a grant to read the space
    const token = principal.client.reference(Scope.universe.id, "client-alice");
    const grantId = await new AccessFixture(storage.database, world).replicateGrant(alice, token, [
        { ...space.permission("read"), scope: SPACE_ID },
    ]);
    const delegated = { actors: [token], grants: [{ id: grantId, scope: users.alice }] };
    const check = async (
        caller: Subject,
        subject: Subject,
        permission: string,
        isDelegated = false,
    ) => {
        // answer the decision, or the refusal
        const pending = store.query(
            subjectContext(caller, SPACE_ID, {
                environment: world,
                ...(isDelegated ? delegated : {}),
            }).access(),
            relationship.call("check", { scope: SPACE_ID, subject, permission, object }),
        );
        const refused = await refusal(pending);

        return refused === "done" ? await pending : refused;
    };
    const checks = {
        aliceShare: await check(alice, alice, "share"),
        bobRead: await check(bob, bob, "read"),
        bobShare: await check(bob, bob, "share"),
        bobAsksAlice: await check(bob, alice, "read"),
        aliceAsksBob: await check(alice, bob, "read"),
        aliceAsksCarol: await check(alice, carol, "read"),
        tokenRead: await check(alice, alice, "read", true),
        tokenShare: await check(alice, alice, "share", true),
        tokenAsksBob: await check(alice, bob, "read", true),
        unknown: await check(alice, alice, "edit"),
    };

    // answer the caller's access, limit the token to its grant, and let only grantors ask for others
    const needsShare = [
        "FORBIDDEN",
        "CHECK_REFUSED",
        "checking another subject's access to space needs share",
    ];
    expect(checks).toEqual({
        aliceShare: { isAllowed: true },
        bobRead: { isAllowed: true },
        bobShare: { isAllowed: false },
        bobAsksAlice: needsShare,
        aliceAsksBob: { isAllowed: true },
        aliceAsksCarol: { isAllowed: false },
        tokenRead: { isAllowed: true },
        tokenShare: { isAllowed: false },
        tokenAsksBob: needsShare,
        unknown: ["BAD_REQUEST", "PERMISSION_UNKNOWN", "space has no permission edit"],
    });
});

test("list an object's relationships to whoever may read them, and revoke one only as a grantor of its relation", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // serve the universe, where the space's row and its relationships live
    const { store } = await serveSpace(world, Scope.universe.id);
    const list = (caller: Subject) =>
        store.query(
            subjectContext(caller, SPACE_ID, { environment: world }).access(),
            relationship.call("list", { scope: Scope.universe.id, where: { objectId: SPACE_ID } }),
        );
    const listed = async (caller: Subject) =>
        (await list(caller)).items
            .map((row) => [row.relation ?? "owner", row.subjectId])
            .toSorted((left, right) => left.join(" ").localeCompare(right.join(" ")));

    // list the space's relationships as alice, bob and carol
    const lists = {
        alice: await listed(alice),
        bob: await listed(bob),
        carol: await listed(carol),
    };

    // revoke bob's reading as carol, who cannot see it, as bob, who may not grant it, and as alice
    const reading = (await list(alice)).items.find((row) => row.relation === "reader");
    const revoke = (caller: Subject) =>
        refusal(
            store.mutate(subjectContext(caller, SPACE_ID, { environment: world }).access(), {
                id: RequestId.create(store.environment),
                calls: [
                    relationship.call("delete", {
                        scope: Scope.universe.id,
                        id: present(reading, "bob's reading").id,
                    }),
                ],
            }),
        );
    const both = [
        ["owner", users.alice],
        ["reader", users.bob],
    ];
    expect({
        lists,
        carol: await revoke(carol),
        bob: await revoke(bob),
        alice: await revoke(alice),
        after: await listed(alice),
    }).toEqual({
        lists: { alice: both, bob: both, carol: [] },
        carol: [
            "NOT_FOUND",
            "NOT_FOUND",
            `relationship ${present(reading, "bob's reading").id} not found`,
        ],
        bob: ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: share"],
        alice: "done",
        after: [["owner", users.alice]],
    });
});

test("touch a relationship into being once, keeping it on a second touch", async () => {
    // relate carol to alice's space as a reader by touch, twice, as alice
    const world = testSimulation().environment("test");
    const { store, object } = await serveSpace(world, Scope.universe.id);
    const touch = async () => {
        const [touched] = await store.mutate(
            subjectContext(alice, SPACE_ID, { environment: world }).access(),
            {
                id: RequestId.create(store.environment),
                calls: [
                    relationship.call("touch", {
                        scope: Scope.universe.id,
                        object,
                        relation: "reader",
                        subject: carol,
                    }),
                ],
            },
        );

        return schema.looseObject({ id: schema.string() }).parse(touched).id;
    };
    const first = await touch();
    const second = await touch();

    // keep the one relationship the first touch created
    const readers = await store.query(
        subjectContext(alice, SPACE_ID, { environment: world }).access(),
        relationship.call("list", {
            scope: Scope.universe.id,
            where: { objectId: SPACE_ID, relation: "reader", subjectId: carol.id },
        }),
    );
    expect([second, readers.items.map((row) => row.id)]).toEqual([first, [first]]);
});

/** Serve a private space of alice, read by bob, with its relationships, as one partition. */
async function serveSpace(world: Environment, partition?: string) {
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({
                packages: [objectPackage, { objects: { relationship }, tables: [] }],
                follows: [],
                tables: [space.table],
            }),
        }),
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());

    // create a private space of alice, read by bob
    const object = space.reference(Scope.universe.id, SPACE_ID);
    const fixture = new AccessFixture(storage.database, world);
    await fixture.replicateScope(object, space.policy);
    await fixture.replicateOwner(object, alice);
    await fixture.replicateRelationship(object, "reader", bob);
    await storage.database.transaction(
        async (transaction) =>
            await transaction
                .insert(space.table)
                .values({ id: SPACE_ID, scope: Scope.universe.id, createdAt: 1, updatedAt: 1 }),
    );
    const store = await TestObjectStore.at(space.reference(Scope.universe.id, SPACE_ID), {
        chain: async () => undefined,
        environment: world,
        ...(partition === undefined ? {} : { partitions: [partition] }),
        principal: spacePrincipal(SPACE_ID),
        servers: serversOf([relationship]),
        policies: [space],
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: { package: relationship.package },
    });

    return { storage, store, object };
}
