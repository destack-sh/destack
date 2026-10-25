import { testSimulation } from "../src/test/index.ts";
import { principal, relation, through } from "@destack/access";
import { Change, defineDatabase } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";
import { ObjectReference, RequestId, Scope } from "@destack/sync";
import { refusal, expect, onTestFinished, test } from "@destack/test";
import { defineObject, field, tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { serversOf, testCallKey, testEvents } from "../src/test/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { userContext, users } from "./fixture/user.ts";
import { TestObjectStore } from "../src/test/index.ts";

/** The time the store runs at: 2026-10-24, in Unix milliseconds. */
const START = Date.UTC(2026, 9, 24);

/** The space the boards live in. */
const SPACE_ID = "space-01996ab0-0000-7000-8000-000000000301";

/** A board its author writes. */
const board = defineObject(
    {
        name: "board",
        plural: "boards",
        scope: space,
        fields: { author: field.subject().caller(), title: field.string() },
        permissions: { read: through("space", "read"), write: relation("author") },
    },
    (method) => ({ create: method.create("read", { fields: ["title"] }) }),
);

/** A card on a board, which the board's writers create. */
const card = defineObject(
    {
        name: "card",
        plural: "cards",
        scope: space,
        nested: { in: board, receive: "write" },
        fields: { label: field.string() },
        permissions: { read: through("parent", "read"), write: through("parent", "write") },
    },
    (method) => ({ create: method.create("write", { fields: ["label"] }) }),
);

/** A pinboard of the space, whose pin call inserts a card on a board. */
const pinboard = defineObject(
    {
        name: "pinboard",
        plural: "pinboards",
        scope: space,
        fields: {},
        permissions: { read: through("space", "read") },
    },
    (method) => ({
        /** Pin a card to a board, answering the card's identifier. */
        pin: method
            .mutation({
                permission: "read",
                target: "scope",
                input: schema.object({
                    boardId: schema.identifier("board"),
                    label: schema.string(),
                }),
                output: schema.object({ id: schema.string() }),
            })
            .handle(async (context) => {
                const inserted = await context.insert(card, {
                    parentId: context.input.boardId,
                    label: context.input.label,
                });

                return { id: inserted.id };
            }),
    }),
);

test("insert an object of the calling package's type from another method, as its create method writes it, refusing a parent the caller cannot write", async () => {
    // run in a world of its own
    const world = testSimulation(START).environment("test");

    // serve the boards and cards in one space at a fixed time
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({
                packages: [objectPackage, { objects: { board, card, pinboard }, tables: [] }],
                follows: [],
                tables: [space.table],
            }),
        }),
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const store = await TestObjectStore.at(space.reference(Scope.universe.id, SPACE_ID), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(SPACE_ID),
        servers: serversOf([board, card, pinboard]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: { package: board.package },
    });
    await openSpace(storage.database, world, SPACE_ID);
    const alice = userContext(users.alice, SPACE_ID, { environment: world }).access();
    const bob = userContext(users.bob, SPACE_ID, { environment: world }).access();

    // create alice's board, then pin a card to it as alice
    const [created] = await store.mutate(alice, {
        id: RequestId.create(store.environment),
        calls: [board.call("create", { spaceId: SPACE_ID, title: "Launch" })],
    });
    const boardId = schema.looseObject({ id: schema.identifier("board") }).parse(created).id;
    const before = (await storage.database.log.position()).sequence;
    const [pinned] = await store.mutate(alice, {
        id: RequestId.create(store.environment),
        calls: [pinboard.call("pin", { spaceId: SPACE_ID, boardId, label: "Ship" })],
    });

    // refuse bob, who reads the board yet cannot write it
    const refused = await refusal(
        store.mutate(bob, {
            id: RequestId.create(store.environment),
            calls: [pinboard.call("pin", { spaceId: SPACE_ID, boardId, label: "Mine" })],
        }),
    );

    // write the card stamped as a created one, and log its insert
    const rows = await storage.database.select().from(card.table);
    const { changes } = await storage.database.log.read({ tables: [card.table], after: before });
    const logged = changes.map((change) => ({
        operation: change.operation,
        key: change.key,
        image: Change.image(change),
        changedAt: change.changedAt,
    }));
    const cardId = schema.looseObject({ id: schema.string() }).parse(pinned).id;
    const author = principal.user.reference("universe", users.alice);
    const row = {
        id: cardId,
        scope: SPACE_ID,
        parentId: boardId,
        label: "Ship",
        revision: 1,
        tags: {},
        detachedAt: null,
        managerInstallationId: null,
        managerName: null,
        managerPackageId: null,
        createdAt: START,
        updatedAt: START,
        ...ObjectReference.columns("createdBy", author),
        ...ObjectReference.columns("updatedBy", author),
    };
    expect({ rows, logged, refused }).toEqual({
        rows: [row],
        logged: [{ operation: "insert", key: { id: cardId }, image: row, changedAt: START }],
        refused: ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: write"],
    });
});
