import { principal, relation } from "@destack/access";
import { journal } from "@destack/audit/stack";
import { defineDatabase } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { present, schema } from "@destack/schema";
import { RequestId } from "@destack/service/request";
import { testCallKey } from "@destack/service/test";
import { expect, onTestFinished, test } from "@destack/test";
import { defineObject, field, FractionalIndex } from "../src/index.ts";
import { ObjectServer } from "../src/server/index.ts";
import { openSpace, space } from "./fixture/space.ts";
import { userContext } from "./fixture/user.ts";

/** The space keeping the items and cards. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000d1");

/** Items in a tree, ordered among the siblings under each parent. */
const item = defineObject({
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
    methods: (method) => ({
        list: method.list("read"),
        create: method.create("write"),
        update: method.update("write"),
    }),
});

/** Cards ordered within the column their status names. */
const card = defineObject({
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
    methods: (method) => ({
        list: method.list("read"),
        create: method.create("write"),
        update: method.update("write"),
    }),
});

/** Serve items and cards over a fresh database of each dialect. */
async function serve(dialect: (typeof TEST_DIALECTS)[number]) {
    const storage = await TestDatabase.create(
        dialect,
        defineDatabase({ name: "order", tables: [...item.tables, ...card.tables, journal] }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);

    return new ObjectServer({
        objects: { item, card },
        database: storage.database,
        callKey: testCallKey,
        origin: { package: item.package, service: "test" },
    });
}

test.for(TEST_DIALECTS)(
    "append %s objects after their last sibling under each parent, list them in order and move one between two others",
    async (dialect) => {
        const server = await serve(dialect);
        const context = userContext("user-1", spaceId);
        const create = (name: string, parentId?: string) =>
            server.call(
                item,
                "create",
                {
                    spaceId,
                    requestId: RequestId.create(),
                    name,
                    ...(parentId === undefined ? {} : { parentId }),
                },
                context,
            );

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
        await server.call(
            item,
            "update",
            {
                spaceId,
                requestId: RequestId.create(),
                id: present(gamma, "the third child").id,
                position: FractionalIndex.place(alpha?.position, beta?.position),
            },
            context,
        );

        // read each parent's children in order
        const listed = await server.call(item, "list", { spaceId }, context);
        const under = (parentId: string | null) =>
            listed.items
                .filter((row) => (row.parentId ?? null) === parentId)
                .map((row) => row.name);

        expect([under(null), under(first.id)]).toEqual([
            ["first", "second"],
            ["a", "c", "b"],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "append %s objects after the last one holding the same value of their ordering field",
    async (dialect) => {
        const server = await serve(dialect);
        const context = userContext("user-1", spaceId);
        const create = (name: string, status: "todo" | "done") =>
            server.call(
                card,
                "create",
                { spaceId, requestId: RequestId.create(), name, status },
                context,
            );

        // fill two columns in turns
        await create("write", "todo");
        await create("ship", "done");
        await create("review", "todo");
        await create("celebrate", "done");
        const listed = await server.call(card, "list", { spaceId }, context);
        const column = (status: string) =>
            listed.items.filter((row) => row.status === status).map((row) => row.name);

        expect([column("todo"), column("done")]).toEqual([
            ["write", "review"],
            ["ship", "celebrate"],
        ]);
    },
);
