import { Caller, none, principal, relation, through } from "@destack/access";
import { defineDatabase } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { EventTime } from "@destack/event";
import { schema, Clock, type Identifier } from "@destack/schema";
import { RequestId } from "@destack/runtime/request";
import { testCallKey } from "@destack/runtime/test";
import { callTables, testEvents, serversOf } from "../src/test/index.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { defineObject, EventView, field } from "../src/index.ts";
import { ObjectStore } from "../src/server/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { userContext, users } from "./fixture/user.ts";

/** The space the visits happen in. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000e1");

/** The first event's time, in Unix microseconds. */
const START = EventTime.of(1_800_000_000_000);

/** The microseconds in a minute, the series' step. */
const MINUTE = EventTime.of(60_000);

/** Page visits, each readable by its visitor. */
const visit = defineObject({
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
}).methods((method) => ({
    list: method.list("read"),
    get: method.get("read"),
    record: method.createMany("record", {
        isInternal: true,
        fields: ["visitorId", "page", "duration", "data"],
    }),
    visit: method.create("visit", { fields: ["visitorId", "page", "duration", "data"] }),
}));

/** Pages of a space, each counting the visits its reader made to it. */
const page = defineObject({
    name: "page",
    plural: "pages",
    scope: space,
    fields: { title: field.string() },
    permissions: { read: through("scope", "read") },
})
    .methods((method) => ({
        visits: method.query({
            permission: "read",
            target: "scope",
            input: schema.object({ title: schema.string() }),
            output: schema.object({ count: schema.number().int() }),
        }),
    }))
    .implement({
        methods: {
            visits: async (call) => {
                // read the caller's visits of the page through the visits' own listing
                const listed = await call.query(
                    call.caller,
                    visit.call("list", { spaceId: call.scope, where: { page: call.input.title } }),
                );

                return { count: listed.items.length };
            },
        },
    });

test.for(TEST_DIALECTS)(
    "list, get and fold the %s events a visitor may read, refusing a fold without a visitor filter",
    async (dialect) => {
        const storage = await TestDatabase.create(
            dialect,
            defineDatabase({
                name: "visits",
                tables: [...space.tables, ...visit.tables, ...callTables],
            }),
            { isMigrated: true },
        );
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);
        const events = await testEvents(storage.database, [visit.eventKind()]);
        const store = new ObjectStore({
            chain: async () => undefined,
            clock: Clock.system,
            principal: spacePrincipal(spaceId),
            servers: serversOf([visit]),
            database: storage.database,
            callKey: testCallKey,
            events,
            origin: { package: visit.package },
        });

        // record two visits of alice a minute apart and one of bob, as the space itself
        const visited = (
            id: string,
            visitor: Identifier<"user">,
            title: string,
            minute: number,
        ) => ({
            id,
            time: START + minute * MINUTE,
            visitorId: visitor,
            page: title,
            duration: 1000 * (minute + 1),
            data: { referrer: null },
        });
        const recorded = await store.mutate(Caller.of(spacePrincipal(spaceId), Date.now()), {
            id: RequestId.create(Date.now()),
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
        });

        // list, get and fold as alice
        const alice = userContext(users.alice, spaceId).caller();
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
            own: await get("visit-2"),
            other: await refusal(get("visit-3")),
            folded: await fold({ visitorId: users.alice }),
            unfiltered: await refusal(fold({})),
            foreign: await refusal(fold({ visitorId: users.bob })),
        }).toEqual({
            recorded: [{ ids: ["visit-1", "visit-2", "visit-3"] }],
            listed: ["visit-1", "visit-2"],
            own: "about",
            other: ["NOT_FOUND", "no visit visit-3"],
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
            unfiltered: ["FORBIDDEN", "permission denied: read"],
            foreign: ["FORBIDDEN", "permission denied: read"],
        });
    },
);

test("append a visitor's own visit publicly, refusing another's, and read visits from a durable method", async () => {
    // keep the visits beside the space's pages
    const storage = await TestDatabase.create(
        "sqlite",
        defineDatabase({
            name: "visits",
            tables: [...space.tables, ...visit.tables, ...page.tables, ...callTables],
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([visit, page]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, [visit.eventKind()]),
        origin: { package: visit.package },
    });

    // append alice's visit as alice, and refuse bob appending one in her name
    const alice = userContext(users.alice, spaceId).caller();
    const bob = userContext(users.bob, spaceId).caller();
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
        id: RequestId.create(Date.now()),
        calls: [visited(users.alice, "visit-1")],
    });
    const refused = await refusal(
        store.mutate(bob, {
            id: RequestId.create(Date.now()),
            calls: [visited(users.alice, "visit-2")],
        }),
    );

    // count alice's visits of the page from a durable method reading them as her
    const counted = await store.query(alice, page.call("visits", { spaceId, title: "home" }));

    expect({ refused, counted }).toEqual({
        refused: ["NOT_FOUND", "visit not found"],
        counted: { count: 1 },
    });
});

test("read the events a space appended after a log sequence, in commit order", async () => {
    // record a visit, then two more after the log's position
    const storage = await TestDatabase.create(
        "sqlite",
        defineDatabase({
            name: "visits",
            tables: [...space.tables, ...visit.tables, ...callTables],
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);
    const events = await testEvents(storage.database, [visit.eventKind()]);
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([visit]),
        database: storage.database,
        callKey: testCallKey,
        events,
        origin: { package: visit.package },
    });
    const record = (ids: readonly string[]) =>
        store.mutate(Caller.of(spacePrincipal(spaceId), Date.now()), {
            id: RequestId.create(Date.now()),
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

    // read only the later visits, and nothing past the sequence the read reached
    const view = new EventView(visit.eventKind(), events);
    const later = await view.changes({ scopes: [spaceId] }, before);
    const drained = await view.changes({ scopes: [spaceId] }, later.sequence);
    expect({
        later: later.events.map((event) => event.id),
        drained: drained.events.length,
    }).toEqual({ later: ["visit-2", "visit-3"], drained: 0 });
});
