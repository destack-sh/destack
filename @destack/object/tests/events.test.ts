import { testSimulation } from "../src/test/index.ts";
import { AccessContext, Caller, none, principal, relation, through } from "@destack/access";
import { defineDatabase } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { EventTime } from "@destack/event";
import { schema, type Identifier } from "@destack/schema";
import { RequestId, Scope } from "@destack/sync";
import { testCallKey } from "@destack/object/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { defineObject, EventView, field } from "../src/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { userContext, users } from "./fixture/user.ts";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { TestObjectStore } from "../src/test/index.ts";

/** The space the visits happen in. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000e1");

/** The first event's time, in Unix microseconds. */
const START = EventTime.of(1_800_000_000_000);

/** The microseconds in a minute, the series' step. */
const MINUTE = EventTime.of(60_000);

/** Page visits, each readable by its visitor. */
const visit = defineObject(
    {
        name: "visit",
        plural: "visits",
        scope: space,
        storage: {
            kind: "events",
            flush: { maxAge: { hours: 1 }, maxRows: 1000 },
            retention: { days: 30 },
            delivery: "exactly-once",
        },
        fields: {
            /** The visitor. */
            visitor: field.reference(principal.user),
            /** The page. */
            page: field.string(),
            /** The time spent on the page, in milliseconds. */
            duration: field.number(),
            /** The page the visitor came from, absent for a direct visit. */
            data: field.json(schema.object({ referrer: schema.string().nullable() })),
        },
        permissions: { read: relation("visitor"), record: none(), visit: relation("visitor") },
    },
    (method) => ({
        list: method.list("read"),
        get: method.get("read"),
        record: method.createMany("record", {
            isInternal: true,
            fields: ["visitorId", "page", "duration", "data"],
        }),
        visit: method.create("visit", { fields: ["visitorId", "page", "duration", "data"] }),
    }),
);

/** Pages of a space, each counting the visits its reader made to it. */
const page = defineObject(
    {
        name: "page",
        plural: "pages",
        scope: space,
        fields: { title: field.string() },
        permissions: { read: through("scope", "read") },
    },
    (method) => ({
        visits: method.query({
            permission: "read",
            target: "scope",
            input: schema.object({ title: schema.string() }),
            output: schema.object({ count: schema.number().int() }),
        }),
    }),
).implement({
    methods: {
        visits: async (call) => {
            // read the caller's visits of the page through the visits' listing
            const listed = await call.query(
                call.caller,
                visit.call("list", { spaceId: call.scope, where: { page: call.input.title } }),
            );

            return { count: listed.items.length };
        },
    },
});

test("list, get and fold the events a visitor may read, refusing a fold without a visitor filter", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create(
        defineDatabase({
            name: "visits",
            ...tablesOf({
                packages: [objectPackage, { objects: { space, visit }, tables: [] }],
                follows: [],
                tables: [],
            }),
        }),
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    const events = await testEvents(storage.database, world, [visit.eventKind()]);
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([visit]),
        database: storage.database,
        callKey: testCallKey,
        events,
        origin: { package: visit.package },
    });

    // record two visits of alice a minute apart and one of bob, as the space itself
    const visited = (id: string, visitor: Identifier<"user">, title: string, minute: number) => ({
        id,
        time: START + minute * MINUTE,
        visitorId: visitor,
        page: title,
        duration: 1000 * (minute + 1),
        data: { referrer: null },
    });
    const recorded = await store.mutate(
        AccessContext.of(Caller.of(spacePrincipal(spaceId)), world.clock.now()),
        {
            id: RequestId.create(world),
            calls: [
                visit.call("record", {
                    spaceId,
                    items: [
                        visited("visit-1", users.alice, "home", 0),
                        visited("visit-2", users.alice, "about", 1),
                        visited("visit-3", users.bob, "home", 0),
                    ],
                }),
            ],
        },
    );

    // list, get and fold as alice
    const alice = userContext(users.alice, spaceId, { environment: world }).access();
    const get = async (id: string) =>
        (await store.query(alice, visit.call("get", { spaceId, id }))).page;
    const fold = (where: object) =>
        store.query(
            alice,
            visit.call("series", {
                spaceId,
                where,
                fold: "sum",
                measure: "duration",
                group: ["page"],
                step: 60_000,
                from: START,
                before: START + 2 * MINUTE,
            }),
        );
    expect({
        recorded,
        listed: (
            await store.query(alice, visit.call("list", { spaceId, orderBy: { time: "asc" } }))
        ).items.map((item) => item.id),
        visited: await get("visit-2"),
        other: await refusal(get("visit-3")),
        folded: await fold({ visitorId: users.alice }),
        unfiltered: await refusal(fold({})),
        foreign: await refusal(fold({ visitorId: users.bob })),
    }).toEqual({
        recorded: [{ ids: ["visit-1", "visit-2", "visit-3"] }],
        listed: ["visit-1", "visit-2"],
        visited: "about",
        other: ["NOT_FOUND", "NOT_FOUND", "visit visit-3 not found"],
        folded: {
            series: [
                {
                    group: { page: "home" },
                    steps: [{ start: START, value: 1000, events: 1 }],
                },
                {
                    group: { page: "about" },
                    steps: [{ start: START + MINUTE, value: 2000, events: 1 }],
                },
            ],
        },
        unfiltered: ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: read"],
        foreign: ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: read"],
    });
});

test("append a visitor's visit publicly, refusing another's, and read visits from a durable method", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // store the visits beside the space's pages
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "visits",
            ...tablesOf({
                packages: [objectPackage, { objects: { space, visit, page }, tables: [] }],
                follows: [],
                tables: [],
            }),
        }),
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([visit, page]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world, [visit.eventKind()]),
        origin: { package: visit.package },
    });

    // append alice's visit as alice, and refuse bob appending one in her name
    const alice = userContext(users.alice, spaceId, { environment: world }).access();
    const bob = userContext(users.bob, spaceId, { environment: world }).access();
    const visited = (visitor: Identifier<"user">, id: string) =>
        visit.call("visit", {
            spaceId,
            id,
            time: START,
            visitorId: visitor,
            page: "home",
            duration: 1000,
            data: { referrer: null },
        });
    await store.mutate(alice, {
        id: RequestId.create(world),
        calls: [visited(users.alice, "visit-1")],
    });
    const refused = await refusal(
        store.mutate(bob, {
            id: RequestId.create(world),
            calls: [visited(users.alice, "visit-2")],
        }),
    );

    // count alice's visits of the page from a durable method reading them as her
    const counted = await store.query(alice, page.call("visits", { spaceId, title: "home" }));

    expect({ refused, counted }).toEqual({
        refused: ["NOT_FOUND", "NOT_FOUND", "visit not found"],
        counted: { count: 1 },
    });
});

test("read the events a space appended after a log sequence, in commit order", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // record a visit, then two more after the log's position
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "visits",
            ...tablesOf({
                packages: [objectPackage, { objects: { space, visit }, tables: [] }],
                follows: [],
                tables: [],
            }),
        }),
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    const events = await testEvents(storage.database, world, [visit.eventKind()]);
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([visit]),
        database: storage.database,
        callKey: testCallKey,
        events,
        origin: { package: visit.package },
    });
    const record = (ids: readonly string[]) =>
        store.mutate(AccessContext.of(Caller.of(spacePrincipal(spaceId)), world.clock.now()), {
            id: RequestId.create(world),
            calls: [
                visit.call("record", {
                    spaceId,
                    items: ids.map((id, minute) => ({
                        id,
                        time: START + minute * MINUTE,
                        visitorId: users.alice,
                        page: "home",
                        duration: 1000,
                        data: { referrer: null },
                    })),
                }),
            ],
        });
    await record(["visit-1"]);
    const { sequence: before } = await storage.database.log.position();
    await record(["visit-2", "visit-3"]);

    // read only the later visits, and nothing past the sequence the read arrived at
    const view = new EventView(visit.eventKind(), events);
    const later = await view.changes({ scopes: [spaceId] }, before);
    const drained = await view.changes({ scopes: [spaceId] }, later.sequence);
    expect({
        later: later.events.map((event) => event.id),
        drained: drained.events.length,
    }).toEqual({ later: ["visit-2", "visit-3"], drained: 0 });
});
