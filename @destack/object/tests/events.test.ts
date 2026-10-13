import { Caller, none, principal, relation } from "@destack/access";
import { defineDatabase } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { EventTime } from "@destack/event";
import { schema, Clock } from "@destack/schema";
import { RequestId } from "@destack/runtime/request";
import { testCallKey } from "@destack/runtime/test";
import { callTables, testEvents } from "../src/test/index.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { defineObject, field } from "../src/index.ts";
import { ObjectStore } from "../src/server/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { userContext } from "./fixture/user.ts";

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
    permissions: { read: relation("visitor"), record: none() },
}).methods((method) => ({
    list: method.list("read"),
    get: method.get("read"),
    record: method.createMany("record", {
        isInternal: true,
        fields: ["visitorId", "page", "duration", "data"],
    }),
}));

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
            clock: Clock.system,
            principal: spacePrincipal(spaceId),
            objects: [visit],
            database: storage.database,
            callKey: testCallKey,
            events,
            origin: { package: visit.package },
        });

        // record two visits of alice a minute apart and one of bob, as the space itself
        const visited = (id: string, visitor: string, page: string, minute: number) => ({
            id,
            time: START + minute * MINUTE,
            visitorId: visitor,
            page,
            duration: 1000 * (minute + 1),
            data: { referrer: null },
        });
        const recorded = await store.mutate(Caller.of(spacePrincipal(spaceId), Date.now()), {
            id: RequestId.create(Date.now()),
            calls: [
                visit.call("record", {
                    spaceId,
                    items: [
                        visited("visit-1", "alice", "home", 0),
                        visited("visit-2", "alice", "about", 1),
                        visited("visit-3", "bob", "home", 0),
                    ],
                }),
            ],
        });

        // list, get and fold as alice
        const alice = userContext("alice", spaceId).caller();
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
            folded: await fold({ visitorId: "alice" }),
            unfiltered: await refusal(fold({})),
            foreign: await refusal(fold({ visitorId: "bob" })),
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
