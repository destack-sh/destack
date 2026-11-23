import type { Environment } from "@destack/environment";
import { testSimulation } from "../src/test/index.ts";
import { principal, relation } from "@destack/access";
import { defineDatabase } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { nth, present, schema } from "@destack/schema";
import { RequestId, Scope } from "@destack/sync";
import { testCallKey } from "@destack/object/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { defineObject, field, type MethodInput } from "../src/index.ts";
import { openSpace, space, spacePrincipal } from "../src/test/space.ts";
import { userContext, users } from "./fixture/user.ts";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { TestObjectStore } from "../src/test/index.ts";
import { nested, orderable } from "../src/index.ts";
import { argument, plural, t } from "@destack/locale";

/** The space with the items and cards. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000d1");

/** The cards the model has. */
const CARD_COUNT = 8;

/** The moves made at random, about a hundred milliseconds' worth. */
const MOVE_COUNT = 40;

/** Items in a tree, ordered among the siblings under each parent. */
const item = defineObject(
    {
        name: "item",
        plural: "items",
        title: t`${plural(argument("count"), { one: "Item", other: "Items" })}`,
        scope: space,
        fields: {
            owner: field.reference(principal.user).caller(),
            name: field.string(),
        },
        access: {
            permissions: { read: relation("owner"), write: relation("owner") },
        },
        traits: [nested({ in: "self", optional: true, add: "write", move: "write" }), orderable()],
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
        title: t`${plural(argument("count"), { one: "Card", other: "Cards" })}`,
        scope: space,
        fields: {
            owner: field.reference(principal.user).caller(),
            name: field.string(),
            status: field.enum(["todo", "done"]),
        },
        access: {
            permissions: { read: relation("owner"), write: relation("owner") },
        },
        traits: [orderable({ within: "status", move: "write" })],
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

test("append objects after their last sibling under each parent, and move them in and between parents", async () => {
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
    const move = (input: MethodInput<typeof item, "move">) =>
        store.mutate(context.access(), {
            id: RequestId.create(store.environment),
            calls: [item.call("move", { spaceId, ...input })],
        });

    // append two roots and three children of the first
    const first = await create("first");
    const second = await create("second");
    const alpha = await create("a", first.id);
    const beta = await create("b", first.id);
    const gamma = await create("c", first.id);

    // read the children of each parent in order
    const order = async () => {
        const listed = await store.query(context.access(), item.call("list", { spaceId }));

        return [null, first.id, second.id].map((parentId) =>
            listed.items
                .filter((row) => (row.parentId ?? null) === parentId)
                .map((row) => row.name),
        );
    };

    // move the last child after the first, then first, then last
    await move({ id: gamma.id, after: alpha.id });
    const between = await order();
    await move({ id: gamma.id, after: null });
    const leading = await order();
    await move({ id: alpha.id });
    const trailing = await order();

    // move one under the other root, and another before it
    await move({ id: beta.id, parentId: second.id });
    await move({ id: alpha.id, parentId: second.id, after: null });

    expect([between, leading, trailing, await order()]).toEqual([
        [["first", "second"], ["a", "c", "b"], []],
        [["first", "second"], ["c", "a", "b"], []],
        [["first", "second"], ["c", "b", "a"], []],
        [["first", "second"], ["c"], ["a", "b"]],
    ]);

    // refuse an anchor under another parent, itself as the anchor, and two places at once
    const refused = [];
    for (const input of [
        { after: alpha.id },
        { after: gamma.id },
        { after: null, position: alpha.position },
    ]) {
        refused.push(await refusal(move({ id: gamma.id, ...input })));
    }
    expect(refused).toEqual([
        [
            "BAD_REQUEST",
            "MOVE_INVALID",
            `move of item is invalid: '${alpha.id}' is not its sibling`,
        ],
        [
            "BAD_REQUEST",
            "MOVE_INVALID",
            `move of item is invalid: '${gamma.id}' is not its sibling`,
        ],
        [
            "BAD_REQUEST",
            "MOVE_INVALID",
            "move of item is invalid: it takes either after or position",
        ],
    ]);
});

test("move objects between the values of their ordering field, and back where they were", async () => {
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

    // read both columns in order
    const order = async () => {
        const listed = await store.query(context.access(), card.call("list", { spaceId }));

        return ["todo", "done"].map((status) =>
            listed.items.filter((row) => row.status === status).map((row) => row.name),
        );
    };

    // fill two columns, then move a card into the other one, after its first card
    await create("write", "todo");
    const review = await create("review", "todo");
    await create("announce", "todo");
    const ship = await create("ship", "done");
    await create("celebrate", "done");
    const input = { spaceId, id: review.id, status: "done", after: ship.id } as const;
    const [moved] = await store.mutate(context.access(), {
        id: RequestId.create(store.environment),
        calls: [card.call("move", input)],
    });
    const after = await order();

    // undo the move by its inverse
    const undone = card.methods.move.inverse?.({
        object: card,
        name: "move",
        input,
        before: review,
        after: moved,
    });
    await store.mutate(context.access(), {
        id: RequestId.create(store.environment),
        calls: present(undone, "the calls undoing the move"),
    });

    // undo nothing once someone else moved it to another column
    const changed = card.methods.move.inverse?.({
        object: card,
        name: "move",
        input,
        before: review,
        after: moved,
        current: review,
    });
    expect(changed).toBeUndefined();

    expect([after, await order()]).toEqual([
        [
            ["write", "announce"],
            ["ship", "review", "celebrate"],
        ],
        [
            ["write", "review", "announce"],
            ["ship", "celebrate"],
        ],
    ]);
});

test("keep the order of a plain list through any moves, two into one place included", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const store = await serve(world);
    const context = userContext(users.frank, spaceId, { environment: world });
    const move = (input: MethodInput<typeof card, "move">) =>
        store.mutate(context.access(), {
            id: RequestId.create(store.environment),
            calls: [card.call("move", { spaceId, ...input })],
        });

    // fill one column, listing it in a model of both
    const columns: Record<"todo" | "done", string[]> = { todo: [], done: [] };
    for (let index = 0; index < CARD_COUNT; index += 1) {
        const [created] = await store.mutate(context.access(), {
            id: RequestId.create(store.environment),
            calls: [
                card.call("create", { spaceId, name: `Card ${String(index)}`, status: "todo" }),
            ],
        });
        columns.todo.push(created.id);
    }
    const ids = [...columns.todo];

    // move a card after another, first or last in either column, drawn at random
    for (let round = 0; round < MOVE_COUNT; round += 1) {
        const [drawn = 0, side = 0, place = 0] = world.random.bytes(3);
        const id = nth(ids, drawn % ids.length);
        const status = side % 2 === 0 ? "todo" : "done";
        columns.todo = columns.todo.filter((each) => each !== id);
        columns.done = columns.done.filter((each) => each !== id);
        const column = columns[status];
        const index = place % (column.length + 2);
        const after = index === 0 ? null : column[index - 1];
        await move({ id, status, ...(after === undefined ? {} : { after }) });
        column.splice(Math.min(index, column.length), 0, id);
    }

    // move two cards after the same one at once
    const [anchor, left, right] = [nth(ids, 0), nth(ids, 1), nth(ids, 2)];
    for (const status of ["todo", "done"] as const) {
        columns[status] = columns[status].filter((each) => each !== left && each !== right);
    }
    const status = columns.todo.includes(anchor) ? "todo" : "done";
    await Promise.all([
        move({ id: left, status, after: anchor }),
        move({ id: right, status, after: anchor }),
    ]);

    // read both columns by position, the two in either order
    const { items } = await store.query(
        context.access(),
        card.call("list", { spaceId, orderBy: { position: "asc" } }),
    );
    const read = (column: string) =>
        items.filter((row) => row.status === column).map((row) => row.id);
    const pair = read(status)
        .slice(columns[status].indexOf(anchor) + 1)
        .slice(0, 2);
    columns[status].splice(columns[status].indexOf(anchor) + 1, 0, ...pair);

    expect([pair.toSorted(), read("todo"), read("done")]).toEqual([
        [left, right].toSorted(),
        columns.todo,
        columns.done,
    ]);
    expect(new Set(items.map((row) => row.position)).size).toBe(CARD_COUNT);
});

test("refuse an ordering declaring a move its nesting declares", () => {
    expect(() =>
        defineObject({
            name: "leaf",
            plural: "leaves",
            title: t`${plural(argument("count"), { one: "Leaf", other: "Leaves" })}`,
            scope: space,
            access: { permissions: ["write"] },
            traits: [
                nested({ in: "self", optional: true, add: "write", move: "write" }),
                orderable({ move: "write" }),
            ],
        }),
    ).toThrow(
        new TypeError("object 'leaf' moves by its nesting, so its ordering cannot declare move"),
    );
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
