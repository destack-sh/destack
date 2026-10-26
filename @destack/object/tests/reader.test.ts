import { testSimulation } from "../src/test/index.ts";
import { defineObject, field, ObjectDescription } from "../src/index.ts";
import { users } from "./fixture/user.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { AccessContext, Authorization, Caller, none, principal } from "@destack/access";
import { defineDatabase, Transaction, type ReconcileController } from "@destack/db";
import { AccessFixture } from "@destack/access/test";
import { TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";
import { subjectContext, testCallKey } from "@destack/object/test";
import { RequestId, Scope } from "@destack/sync";
import { testEvents, serversOf } from "../src/test/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { EventTime } from "@destack/event";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { TestObjectStore } from "../src/test/index.ts";

/** The space the feeds live in. */
const SPACE_ID = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000003");

/** Another space, whose ticks the first space's reader passes. */
const OTHER_SPACE_ID = schema
    .identifier("space")
    .parse("space-01996ab0-0000-7000-8000-000000000004");

/** The time the ticks start at, in Unix milliseconds. */
const START = 1_800_000_000_000;

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
    // run in a world of its own
    const world = testSimulation().environment("test");

    // serve the space's feeds to its owner
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({
                packages: [objectPackage, { objects: { feed }, tables: [] }],
                follows: [],
                tables: [space.table],
            }),
        }),
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const { database } = storage;
    const store = await TestObjectStore.at(space.reference(Scope.universe.id, SPACE_ID), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(SPACE_ID),
        servers: serversOf([feed]),
        policies: [space],
        database,
        callKey: testCallKey,
        events: await testEvents(database, world),
        origin: { package: feed.package },
    });
    await database.transaction(
        async (transaction) =>
            await transaction
                .insert(space.table)
                .values({ id: SPACE_ID, scope: "universe", createdAt: 1, updatedAt: 1 }),
    );
    const owner = principal.user.reference("universe", users.owner);

    // bear the space at its home, its owner bound in the universe and replicated here
    const born = await Transaction.open(database);
    await new Authorization(
        store.authorizer,
        born,
        () => AccessContext.of(Caller.of(owner), world.clock.now()),
        world,
    ).receive(space.reference("universe", SPACE_ID), []);
    await born.commit();
    await new AccessFixture(database, world).replicateOwner(
        space.reference("universe", SPACE_ID),
        owner,
    );
    const caller = subjectContext(owner, SPACE_ID, { environment: world }).access();
    const mutate = (call: ReturnType<typeof feed.call>) =>
        store.mutate(caller, { id: RequestId.create(world), calls: [call] });

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

test("refuse a batch of a reader per row", () => {
    // declare a feed reader batching pages its rows never form
    expect(() =>
        ObjectDescription.of(
            defineObject({
                name: "feed",
                plural: "feeds",
                scope: space,
                fields: { name: field.string() },
                permissions: ["read", "write"],
                readers: {
                    feed: {
                        by: "id",
                        start: "latest",
                        hold: { minutes: 1 },
                        batch: { minutes: 1 },
                    },
                },
            }),
        ),
    ).toThrow(
        new TypeError(
            "reader feed of feed reads each row, so it cannot batch pages: only readers by scope can",
        ),
    );
});

test("refuse a reader per scope that starts at the latest commit", () => {
    // declare a space reader that each feed would restart
    expect(() =>
        ObjectDescription.of(
            defineObject({
                name: "feed",
                plural: "feeds",
                scope: space,
                fields: { name: field.string() },
                permissions: ["read", "write"],
                readers: { space: { by: "scope", start: "latest", hold: { minutes: 1 } } },
            }),
        ),
    ).toThrow(
        new TypeError("reader space of feed starts each scope once: start it at the earliest"),
    );
});

/** The milliseconds a reader lets a page's first event wait. */
const BATCH = 60_000;

/** Ticks of a space, each page of them taken by its reader a minute after its first tick. */
const tick = defineObject(
    {
        name: "tick",
        plural: "ticks",
        scope: space,
        storage: {
            kind: "events",
            flush: { maxAge: { hours: 1 }, maxRows: 1000 },
            retention: { days: 1 },
            delivery: "exactly-once",
        },
        readers: {
            count: { by: "scope", start: "earliest", hold: { days: 1 }, batch: { minutes: 1 } },
        },
        fields: {
            /** The tick's count. */
            count: field.number(),
            /** Nothing beside the count. */
            data: field.json(schema.object({})),
        },
        permissions: { read: none(), record: none() },
    },
    (method) => ({
        record: method.createMany("record", { visibility: "package", fields: ["count", "data"] }),
    }),
);

test("hand a scope's page of ticks to the handler once its first is a batch old, the same page after a failed attempt, the same page to two hosts at once, then the next page", async () => {
    // run in a world of its own, moving its time while the work waits
    const simulation = testSimulation(START);
    await simulation.drive(
        (async () => {
            const { clock } = simulation;
            const world = simulation.environment("test");

            // serve the space's ticks as the space itself
            const storage = await TestDatabase.create(
                defineDatabase({
                    name: "main",
                    ...tablesOf({
                        packages: [objectPackage, { objects: { space, tick }, tables: [] }],
                        follows: [],
                        tables: [],
                    }),
                }),
                { environment: world, isMigrated: true },
            );
            onTestFinished(() => storage.close());
            const { database } = storage;
            const store = await TestObjectStore.at(space.reference(Scope.universe.id, SPACE_ID), {
                chain: async () => undefined,
                environment: world,
                principal: spacePrincipal(SPACE_ID),
                servers: serversOf([tick]),
                database,
                callKey: testCallKey,
                events: await testEvents(database, world, [tick.eventKind()]),
                origin: { package: tick.package },
            });
            await openSpace(database, world, SPACE_ID);
            const record = async (id: string, spaceId: string = SPACE_ID) => {
                await store.mutate(
                    AccessContext.of(Caller.of(spacePrincipal(spaceId)), clock.now()),
                    {
                        id: RequestId.create(world),
                        calls: [
                            tick.call("record", {
                                spaceId,
                                items: [
                                    { id, time: EventTime.of(clock.now()), count: 1, data: {} },
                                ],
                            }),
                        ],
                    },
                );

                return (await database.log.position()).sequence;
            };

            // fail the first attempt, and let host two take the second page while host one has it
            const pages: {
                readonly host: string;
                readonly ids: string[];
                readonly after: number;
                readonly through: number;
                readonly claimedAt: number;
            }[] = [];
            let isFailing = true;
            const reconcile = (consumer: ReconcileController) =>
                consumer.reconcile(SPACE_ID, {
                    signal: new AbortController().signal,
                    changed: async () => undefined,
                });
            const two = tick.consume("count", async (page) => {
                pages.push({
                    host: "two",
                    ids: page.events.map((event) => event.id),
                    after: page.after,
                    through: page.through,
                    claimedAt: page.claimedAt,
                });
            })(store);
            const one = tick.consume("count", async (page) => {
                // record the page, fail the first, and run host two before committing the third
                pages.push({
                    host: "one",
                    ids: page.events.map((event) => event.id),
                    after: page.after,
                    through: page.through,
                    claimedAt: page.claimedAt,
                });
                if (isFailing) {
                    isFailing = false;
                    throw new Error("the counter is unreachable");
                } else if (page.events.length > 1) {
                    await reconcile(two);
                }
            })(store);

            // tick, wait half a batch, fail a batch in, tick again, then retry
            const first = await record("1");
            clock.set(START + BATCH / 2);
            const young = await reconcile(one);
            clock.set(START + BATCH);
            const failed = await reconcile(one).catch((error: unknown) => String(error));
            await record("2");
            const retried = await reconcile(one);

            // tick a batch later, then take the page on host one while host two takes it too
            clock.set(START + 2 * BATCH);
            const third = await record("3");
            clock.set(START + 3 * BATCH);
            const both = await reconcile(one);
            const idle = await reconcile(two);

            // tick in another space, then pass the page without the space's ticks and look again
            await openSpace(database, world, OTHER_SPACE_ID);
            const elsewhere = await record("4", OTHER_SPACE_ID);
            const passed = await reconcile(two);
            const rest = await reconcile(two);

            // take the failed page twice unchanged, and the next one on both hosts, host two moving past it
            const reader = tick.reader("count").at(SPACE_ID);
            expect({
                young,
                failed,
                retried,
                both,
                idle,
                passed,
                rest,
                pages,
                place: [await reader.after(database), await reader.claimed(database)],
            }).toEqual({
                young: BATCH / 2,
                failed: "Error: the counter is unreachable",
                retried: 0,
                both: 0,
                idle: undefined,
                passed: 0,
                rest: undefined,
                pages: [
                    { host: "one", ids: ["1"], after: 0, through: first, claimedAt: START + BATCH },
                    { host: "one", ids: ["1"], after: 0, through: first, claimedAt: START + BATCH },
                    {
                        host: "one",
                        ids: ["2", "3"],
                        after: first,
                        through: third,
                        claimedAt: START + 3 * BATCH,
                    },
                    {
                        host: "two",
                        ids: ["2", "3"],
                        after: first,
                        through: third,
                        claimedAt: START + 3 * BATCH,
                    },
                ],
                place: [elsewhere, undefined],
            });
        })(),
    );
});
