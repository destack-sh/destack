import { users } from "./fixture/user.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { AccessContext, Authorization, Caller, principal } from "@destack/access";
import { defineDatabase, Transaction } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";
import { subjectContext, testCallKey } from "@destack/runtime/test";
import { RequestId } from "@destack/runtime/request";
import { callTables, testEvents, serversOf } from "../src/test/index.ts";
import { ObjectStore } from "../src/server/index.ts";
import { defineObject, field } from "../src/index.ts";
import { space, spacePrincipal } from "./fixture/space.ts";
import { Clock } from "@destack/environment";

/** The space the feeds live in. */
const SPACE_ID = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000003");

/** Feeds, each read from its creation on, and their space's, read from the log's start. */
const feed = defineObject(
    {
        name: "feed",
        plural: "feeds",
        scope: space,
        fields: { name: field.string() },
        permissions: ["read", "write"],
        readers: {
            feed: { by: "id", start: "latest", hold: { minutes: 1 } },
            space: { by: "scope", start: "earliest", hold: { minutes: 1 } },
        },
    },
    (method) => ({
        create: method.create("write"),
        delete: method.delete("write"),
    }),
);

test("start a feed's reader at its creating commit and release it at its deletion, leaving its space's reader", async () => {
    // serve the space's feeds to its owner
    const storage = await TestDatabase.create(
        "sqlite",
        defineDatabase({ name: "main", tables: [...feed.tables, space.table, ...callTables] }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const { database } = storage;
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(SPACE_ID),
        servers: serversOf([feed]),
        policies: [space],
        database,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: { package: feed.package },
    });
    await database
        .insert(space.table)
        .values({ id: SPACE_ID, scope: "universe", createdAt: 1, updatedAt: 1 });
    const owner = principal.user.reference("universe", users.owner);
    const creating = await Transaction.open(database);
    await new Authorization(store.authorizer, creating, () =>
        AccessContext.of(Caller.of(owner), Date.now()),
    ).create(space.reference("universe", SPACE_ID), { owner });
    await creating.commit();
    const caller = subjectContext(owner, SPACE_ID).access();
    const mutate = (call: ReturnType<typeof feed.call>) =>
        store.mutate(caller, { id: RequestId.create(Date.now()), calls: [call] });

    // create two feeds, the first at the log's position then
    const { sequence } = await database.log.position();
    const [first] = await mutate(feed.call("create", { spaceId: SPACE_ID, name: "first" }));
    const [second] = await mutate(feed.call("create", { spaceId: SPACE_ID, name: "second" }));
    const firstId = schema.looseObject({ id: schema.string() }).parse(first).id;
    const secondId = schema.looseObject({ id: schema.string() }).parse(second).id;
    const created = {
        feeds: await feed.reader("feed").keys(database),
        first: await feed.reader("feed").at(firstId).after(database),
        spaces: await feed.reader("space").keys(database),
    };

    // delete the first feed
    await mutate(feed.call("delete", { spaceId: SPACE_ID, id: firstId }));
    const deleted = {
        feeds: await feed.reader("feed").keys(database),
        spaces: await feed.reader("space").keys(database),
    };

    // start each feed's slot at its creating commit, place the space's once, and release only the deleted feed's
    expect({ created, deleted }).toEqual({
        created: { feeds: [firstId, secondId].toSorted(), first: sequence, spaces: [SPACE_ID] },
        deleted: { feeds: [secondId], spaces: [SPACE_ID] },
    });
});

test("refuse a reader per scope that starts at the latest commit", () => {
    // declare a space reader that each feed would restart
    expect(() =>
        defineObject({
            name: "feed",
            plural: "feeds",
            scope: space,
            fields: { name: field.string() },
            permissions: ["read", "write"],
            readers: { space: { by: "scope", start: "latest", hold: { minutes: 1 } } },
        }).describe(),
    ).toThrow(
        new TypeError("reader space of feed starts each scope once: start it at the earliest"),
    );
});
