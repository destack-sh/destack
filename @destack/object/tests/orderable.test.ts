import type { Environment } from "@destack/environment";
import { testSimulation } from "../src/test/index.ts";
import { principal, relation } from "@destack/access";
import { defineDatabase } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { present, schema } from "@destack/schema";
import { RequestId, Scope } from "@destack/sync";
import { testCallKey } from "@destack/object/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { defineObject, field, FractionalIndex } from "../src/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { userContext, users } from "./fixture/user.ts";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { TestObjectStore } from "../src/test/index.ts";

/** The space with the items and cards. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000d1");

/** Items in a tree, ordered among the siblings under each parent. */
const item = defineObject(
    {
        name: "item",
        plural: "items",
        scope: space,
        nested: { in: "self", optional: true, receive: "write", move: "write" },
        orderable: true,
        fields: {
            owner: field.reference(principal.user).caller(),
            name: field.string(),
        },
        permissions: { read: relation("owner"), write: relation("owner") },
    },
    (method) => ({
        create: method.create("write"),
        update: method.update("write"),
    }),
);

/** Cards ordered within the column their status refers to. */
const card = defineObject(
    {
        name: "card",
        plural: "cards",
        scope: space,
        orderable: { within: "status" },
        fields: {
            owner: field.reference(principal.user).caller(),
            name: field.string(),
            status: field.enum(["todo", "done"]),
        },
        permissions: { read: relation("owner"), write: relation("owner") },
    },
    (method) => ({
        create: method.create("write"),
        update: method.update("write"),
    }),
);

/** Serve items and cards over a fresh database of each dialect. */
async function serve(world: Environment) {
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "order",
            ...tablesOf({
                packages: [objectPackage, { objects: { item, card }, tables: [] }],
                follows: [],
                tables: [],
            }),
        }),
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);

    return await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([item, card]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: { package: item.package },
    });
}

test("append objects after their last sibling under each parent, list them in order and move one between two others", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const store = await serve(world);
    const context = userContext(users.frank, spaceId, { environment: world });
    const create = (name: string, parentId?: string) =>
        store
            .mutate(context.access(), {
                id: RequestId.create(store.environment),
                calls: [
                    item.call("create", {
                        spaceId,
                        name,
                        ...(parentId === undefined ? {} : { parentId }),
                    }),
                ],
            })
            .then(([written]) => written);

    // append two roots and three children of the first
    const first = await create("first");
    await create("second");
    const children = [
        await create("a", first.id),
        await create("b", first.id),
        await create("c", first.id),
    ];

    // move the last child between the first two
    const [alpha, beta, gamma] = children;
    await store
        .mutate(context.access(), {
            id: RequestId.create(store.environment),
            calls: [
                item.call("update", {
                    spaceId,
                    id: present(gamma, "the third child").id,
                    position: FractionalIndex.place(
                        alpha?.position,
                        beta?.position,
                        store.environment.random,
                    ),
                }),
            ],
        })
        .then(([written]) => written);

    // read each parent's children in order
    const listed = await store.query(context.access(), item.call("list", { spaceId }));
    const under = (parentId: string | null) =>
        listed.items.filter((row) => (row.parentId ?? null) === parentId).map((row) => row.name);

    expect([under(null), under(first.id)]).toEqual([
        ["first", "second"],
        ["a", "c", "b"],
    ]);
});

test("append objects after the last one with the same value of their ordering field", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const store = await serve(world);
    const context = userContext(users.frank, spaceId, { environment: world });
    const create = (name: string, status: "todo" | "done") =>
        store
            .mutate(context.access(), {
                id: RequestId.create(store.environment),
                calls: [card.call("create", { spaceId, name, status })],
            })
            .then(([written]) => written);

    // fill two columns in turns
    await create("write", "todo");
    await create("ship", "done");
    await create("review", "todo");
    await create("celebrate", "done");
    const listed = await store.query(context.access(), card.call("list", { spaceId }));
    const column = (status: string) =>
        listed.items.filter((row) => row.status === status).map((row) => row.name);

    expect([column("todo"), column("done")]).toEqual([
        ["write", "review"],
        ["ship", "celebrate"],
    ]);
});
