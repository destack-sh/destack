import { Filter } from "@destack/db";
import { TEST_DIALECTS } from "@destack/db/test";
import { Package } from "@destack/package";
import { schema } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { eventSegment } from "../src/archive/catalog.ts";
import { defineEventKind, type EventKind, type Series } from "../src/index.ts";
import { EventFixture, type EventFixtureOptions } from "../src/test/index.ts";

/** The package declaring the test kinds. */
const meters = Package.parse({
    id: "package-019f5530-8000-7000-8000-000000000301",
    name: "@acme/meters",
    version: "2026.10.0",
});

/** A minute, in milliseconds. */
const MINUTE = 60_000;

/** A day, in milliseconds. */
const DAY = 24 * 60 * MINUTE;

/** The start of the tests' time, in Unix milliseconds. */
const START = Date.UTC(2026, 9, 1);

/** Readings of a space's meters, kept exactly once and flushed every three. */
const reading = defineEventKind(
    {
        name: "reading",
        description: "A meter's reading in a space.",
        keys: schema.object({
            meter: schema.string(),
            resource: schema.string(),
            value: schema.number(),
        }),
        data: schema.object({ unit: schema.string(), note: schema.string().exactOptional() }),
        delivery: "exactly-once",
        policy: { flush: { maxAge: 60 * MINUTE, maxRows: 3 }, retention: DAY },
    },
    { package: meters },
);

/** Calls of a space, locked, copied upwards, the caller's address sealed. */
const call = defineEventKind(
    {
        name: "call",
        description: "A call made in a space.",
        keys: schema.object({ actor: schema.string(), method: schema.string() }),
        data: schema.object({
            address: schema.sensitive(schema.string(), "personal").exactOptional(),
            token: schema.sensitive(schema.string()).exactOptional(),
        }),
        delivery: "exactly-once",
        policy: { flush: { maxAge: 60 * MINUTE, maxRows: 2 }, retention: 365 * DAY },
        route: "enclosing",
        isLocked: true,
        subject: "actor",
    },
    { package: meters },
);

/** Log lines of a space: kept at most once, their severity unknown for some. */
const line = defineEventKind(
    {
        name: "line",
        description: "A log line of a space.",
        keys: schema.object({ severity: schema.number().int().nullable() }),
        data: schema.object({ message: schema.string() }),
        delivery: "at-most-once",
        policy: { flush: { maxAge: 60 * MINUTE, maxRows: 100 }, retention: DAY },
    },
    { package: meters },
);

/** Open events of some kinds, closed as the test finishes. */
async function open(
    dialect: (typeof TEST_DIALECTS)[number],
    kinds: readonly EventKind[] = [reading],
    options: EventFixtureOptions = {},
) {
    const fixture = await EventFixture.open(dialect, kinds, options);
    onTestFinished(() => fixture[Symbol.asyncDispose]());
    fixture.now = START;

    return fixture;
}

/** A reading of a meter some minutes after the start. */
function measured(id: string, minute: number, meter: string, value: number, note?: string) {
    return {
        scope: "space-1",
        id,
        time: (START + minute * MINUTE) * 1000,
        keys: { meter, resource: "/spaces/space-1", value },
        data: note === undefined ? { unit: "byte" } : { unit: "byte", note },
    };
}

/** The microseconds some minutes after the start. */
function at(minute: number): number {
    return (START + minute * MINUTE) * 1000;
}

test.for(TEST_DIALECTS)(
    "page a scope's %s events a condition selects in time order, appending one again changing nothing and refusing one with other contents",
    async (dialect) => {
        const { store } = await open(dialect);

        // append three readings, one of them twice and another scope's once
        await store.append(reading, [
            measured("c", 3, "db.bytes", 30),
            measured("a", 1, "db.bytes", 10),
            measured("b", 2, "bucket.bytes", 20),
        ]);
        await store.append(reading, [
            measured("a", 1, "db.bytes", 10),
            { ...measured("x", 1, "db.bytes", 1), scope: "space-2" },
        ]);

        // refuse a reading appended again with another value
        await expect(
            store.append(reading, [measured("a", 1, "db.bytes", 99)]),
        ).rejects.toMatchObject({
            code: "CONFLICT",
            message: `event a of ${reading.key} was stored with other contents`,
        });

        // read the scope's database readings one per page, then every reading of at least 20
        const where = Filter.parse('meter = "db.bytes"');
        const first = await store.query(reading, { scope: "space-1", where }, { limit: 1 });
        const second = await store.query(
            reading,
            { scope: "space-1", where },
            first.cursor === undefined ? { limit: 1 } : { after: first.cursor, limit: 1 },
        );
        const large = await store.query(reading, {
            scope: "space-1",
            where: Filter.parse("value >= 20"),
        });
        expect([
            first.events.map((event) => [event.id, event.keys.value]),
            second.events.map((event) => event.id),
            second.cursor,
            large.events.map((event) => event.id),
        ]).toEqual([[["a", 10]], ["c"], undefined, ["b", "c"]]);
    },
);

test.for(TEST_DIALECTS)(
    "flush a scope's %s hot events at the row cap into segments that queries read like hot ones",
    async (dialect) => {
        const fixture = await open(dialect);
        const { store, database } = fixture;

        // append five readings and flush at the policy's three rows, an hour before any is due by age
        await store.append(
            reading,
            [1, 2, 3, 4, 5].map((minute) =>
                measured(`r${String(minute)}`, minute, "db.bytes", minute),
            ),
        );
        await fixture.settle();
        const hot = await database.database.select().from(reading.table);
        const segments = await database.database.select().from(eventSegment);

        // the readings read back across the segment and the hot rows by time range and condition
        const ranged = await store.query(reading, { scope: "space-1", from: at(2), before: at(5) });
        const selected = await store.query(reading, {
            scope: "space-1",
            where: Filter.parse("value != 4"),
        });
        expect([
            hot.map((row) => row.id),
            segments.map((segment) => [segment.rows, segment.keys["meter"]]),
            ranged.events.map((event) => event.id),
            selected.events.map((event) => event.id),
        ]).toEqual([
            ["r4", "r5"],
            [[3, ["db.bytes"]]],
            ["r2", "r3", "r4"],
            ["r1", "r2", "r3", "r5"],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "search the %s data of hot and flushed events for text, filling each page",
    async (dialect) => {
        const fixture = await open(dialect);
        const { store } = fixture;

        // flush three readings, keep two hot, two of each side noting a timeout
        await store.append(reading, [
            measured("a", 1, "db.bytes", 1, "Timeout reading the size"),
            measured("b", 2, "db.bytes", 2, "read"),
            measured("c", 3, "db.bytes", 3, "a timeout"),
            measured("d", 4, "db.bytes", 4, "read"),
            measured("e", 5, "db.bytes", 5, "TIMEOUT again"),
        ]);
        await fixture.settle();

        // page through the timeouts one at a time, ignoring case
        const first = await store.query(
            reading,
            { scope: "space-1", text: "timeout" },
            { limit: 2 },
        );
        const second = await store.query(
            reading,
            { scope: "space-1", text: "timeout" },
            first.cursor === undefined ? { limit: 2 } : { after: first.cursor, limit: 2 },
        );
        expect([
            first.events.map((event) => event.id),
            second.events.map((event) => event.id),
            second.cursor,
        ]).toEqual([["a", "c"], ["e"], undefined]);
    },
);

test.for(TEST_DIALECTS)(
    "fold a %s measure per step and group across hot events in SQL and flushed ones",
    async (dialect) => {
        const fixture = await open(dialect);
        const { store } = fixture;

        // flush three readings and keep three hot, across two ten-minute steps
        await store.append(reading, [
            measured("a", 1, "db.bytes", 10),
            measured("b", 2, "bucket.bytes", 5),
            measured("c", 3, "db.bytes", 20),
        ]);
        await fixture.settle();
        await store.append(reading, [
            measured("d", 11, "bucket.bytes", 7),
            measured("e", 12, "db.bytes", 30),
            measured("f", 13, "db.bytes", 40),
        ]);

        // fold by meter in ten-minute steps, and count everything in one step
        const fold = async (name: "sum" | "average" | "min" | "max" | "last") =>
            (
                await store.series(
                    reading,
                    { scope: "space-1" },
                    { measure: "value", fold: name, group: ["meter"], step: 10 * MINUTE },
                )
            )
                .toSorted((left, right) =>
                    String(left.group["meter"]).localeCompare(String(right.group["meter"])),
                )
                .map((series) => [
                    series.group["meter"],
                    series.steps.map((step) => [step.start, step.value, step.events]),
                ]);
        const counted = await store.series(
            reading,
            { scope: "space-1", from: at(0) },
            { fold: "count" },
        );
        expect([
            await fold("sum"),
            await fold("average"),
            await fold("min"),
            await fold("max"),
            await fold("last"),
            counted,
        ]).toEqual([
            [
                [
                    "bucket.bytes",
                    [
                        [at(0), 5, 1],
                        [at(10), 7, 1],
                    ],
                ],
                [
                    "db.bytes",
                    [
                        [at(0), 30, 2],
                        [at(10), 70, 2],
                    ],
                ],
            ],
            [
                [
                    "bucket.bytes",
                    [
                        [at(0), 5, 1],
                        [at(10), 7, 1],
                    ],
                ],
                [
                    "db.bytes",
                    [
                        [at(0), 15, 2],
                        [at(10), 35, 2],
                    ],
                ],
            ],
            [
                [
                    "bucket.bytes",
                    [
                        [at(0), 5, 1],
                        [at(10), 7, 1],
                    ],
                ],
                [
                    "db.bytes",
                    [
                        [at(0), 10, 2],
                        [at(10), 30, 2],
                    ],
                ],
            ],
            [
                [
                    "bucket.bytes",
                    [
                        [at(0), 5, 1],
                        [at(10), 7, 1],
                    ],
                ],
                [
                    "db.bytes",
                    [
                        [at(0), 20, 2],
                        [at(10), 40, 2],
                    ],
                ],
            ],
            [
                [
                    "bucket.bytes",
                    [
                        [at(0), 5, 1],
                        [at(10), 7, 1],
                    ],
                ],
                [
                    "db.bytes",
                    [
                        [at(0), 20, 2],
                        [at(10), 40, 2],
                    ],
                ],
            ],
            [{ group: {}, steps: [{ start: at(0), value: 6, events: 6 }] }],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "follow a scope's %s events a condition selects as they commit, never repeating a flushed one",
    async (dialect) => {
        const fixture = await open(dialect);
        const { store } = fixture;

        // start following, then append, flush and append again
        const following = new AbortController();
        const seen: string[] = [];
        const tail = (async () => {
            for await (const { event } of store.tail(
                reading,
                { scope: "space-1", where: Filter.parse('meter = "db.bytes"') },
                following.signal,
            )) {
                seen.push(event.id);
                if (seen.length === 3) {
                    following.abort();
                }
            }
        })();
        await store.append(reading, [
            measured("a", 1, "db.bytes", 1),
            measured("b", 2, "bucket.bytes", 2),
            measured("c", 3, "db.bytes", 3),
        ]);
        await fixture.settle();
        await store.append(reading, [measured("d", 4, "db.bytes", 4)]);
        await tail;

        expect(seen).toEqual(["a", "c", "d"]);
    },
);

test.for(TEST_DIALECTS)(
    "keep a %s nullable whole-number key as an integer, compared and read back null",
    async (dialect) => {
        const { store } = await open(dialect, [line]);

        // append a line of known severity and one of unknown
        const written = (id: string, severity: number | null) => ({
            scope: "space-1",
            id,
            time: at(1),
            keys: { severity },
            data: { message: id },
        });
        await store.append(line, [written("known", 9), written("unknown", null)]);

        // read the severe lines, then every line's severity
        const severe = await store.query(line, {
            scope: "space-1",
            where: Filter.parse("severity >= 9"),
        });
        const every = await store.query(line, { scope: "space-1" });
        expect([
            severe.events.map((event) => event.id),
            every.events.map((event) => [event.id, event.keys.severity]),
        ]).toEqual([
            ["known"],
            [
                ["known", 9],
                ["unknown", null],
            ],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "keep a %s exactly-once event only when its transaction commits, and an at-most-once one outside any",
    async (dialect) => {
        const { store, database } = await open(dialect, [reading, line]);

        // append inside a transaction that fails, then inside one that commits
        await database.database
            .transaction(async (transaction) => {
                await store.append(reading, [measured("lost", 1, "db.bytes", 1)], transaction);
                throw new Error("the call failed");
            })
            .catch(() => undefined);
        await database.database.transaction(async (transaction) => {
            await store.append(reading, [measured("kept", 2, "db.bytes", 2)], transaction);
        });

        // an at-most-once kind refuses a caller's transaction
        const refused = await database.database
            .transaction((transaction) =>
                store.append(
                    line,
                    [
                        {
                            scope: "space-1",
                            id: "l",
                            time: at(1),
                            keys: { severity: 9 },
                            data: { message: "hi" },
                        },
                    ],
                    transaction,
                ),
            )
            .catch((error: unknown) => (error instanceof Error ? error.message : "thrown"));

        expect([
            (await store.query(reading, { scope: "space-1" })).events.map((event) => event.id),
            refused,
        ]).toEqual([
            ["kept"],
            `event kind ${line.key} is kept at most once, never in a caller's transaction`,
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "seal a %s person's personal values at rest, drop secrets, and erase the values in hot and flushed events once the person is forgotten",
    async (dialect) => {
        const receiver = await open(dialect, [call]);
        const fixture = await open(dialect, [call], {
            targets: () => Promise.resolve(["organisation-1"]),
            deliver: (kind, events) => receiver.store.receive(kind, events),
        });
        const { store, database } = fixture;

        // record three calls of one person and one of another, flushing them
        const made = (id: string, minute: number, actor: string) => ({
            scope: "space-1",
            id,
            time: at(minute),
            keys: { actor, method: "update" },
            data: { address: `${actor}@example.com`, token: "secret" },
        });
        await store.append(call, [made("a", 1, "ada"), made("b", 2, "ada"), made("c", 3, "ada")]);
        await database.database.transaction((transaction) =>
            store.append(call, [made("d", 4, "bo")], transaction),
        );
        const rows = await database.database.select().from(call.table);
        await fixture.settle();
        await store.append(call, [made("e", 5, "ada")]);
        const before = await store.query(call, { scope: "space-1" });

        // forget the person
        await store.forget("ada");
        const after = await store.query(call, { scope: "space-1" });
        expect([
            rows.length,
            rows.some((row) => JSON.stringify(row.data).includes("example.com")),
            rows.some((row) => JSON.stringify(row.data).includes("secret")),
            before.events.map((event) => event.data.address),
            after.events.map((event) => [event.id, event.data.address]),
        ]).toEqual([
            4,
            false,
            false,
            [
                "ada@example.com",
                "ada@example.com",
                "ada@example.com",
                "bo@example.com",
                "ada@example.com",
            ],
            [
                ["a", undefined],
                ["b", undefined],
                ["c", undefined],
                ["d", "bo@example.com"],
                ["e", undefined],
            ],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "route a copy of each %s event to the scope its kind names, once however often it is appended",
    async (dialect) => {
        const receiver = await open(dialect, [call]);
        const fixture = await open(dialect, [call], {
            targets: () => Promise.resolve(["organisation-1"]),
            deliver: (kind, events) => receiver.store.receive(kind, events),
        });
        const { store } = fixture;

        // record a call twice and deliver the copies
        const made = {
            scope: "space-1",
            id: "a",
            time: at(1),
            keys: { actor: "ada", method: "update" },
            data: {},
        };
        await store.append(call, [made]);
        await store.append(call, [made]);
        await fixture.settle();

        // the organisation keeps one copy, its source the space
        const copies = await receiver.store.query(call, { scope: "organisation-1" });
        const foreign = await receiver.store.query(call, {
            scope: "organisation-1",
            where: Filter.parse('source = "space-2"'),
        });
        expect([
            copies.events.map((event) => [event.scope, event.source, event.id, event.keys.actor]),
            foreign.events,
        ]).toEqual([[["organisation-1", "space-1", "a", "ada"]], []]);
    },
);

test.for(TEST_DIALECTS)(
    "lock a %s locked kind's segments for its retention, chained by digest and never compacted",
    async (dialect) => {
        const receiver = await open(dialect, [call]);
        const fixture = await open(dialect, [call], {
            targets: () => Promise.resolve(["organisation-1"]),
            deliver: (kind, events) => receiver.store.receive(kind, events),
        });
        const { store, database, bucket } = fixture;

        // flush two segments of two calls each
        await store.append(
            call,
            [1, 2, 3, 4].map((minute) => ({
                scope: "space-1",
                id: `c${String(minute)}`,
                time: at(minute),
                keys: { actor: "ada", method: "update" },
                data: {},
            })),
        );
        await fixture.settle();
        const segments = await database.database
            .select()
            .from(eventSegment)
            .orderBy(eventSegment.from);
        const [first, second] = segments;

        // deleting a segment's file is refused while it is locked
        const deleting = await bucket
            .delete([first?.file ?? ""])
            .then(() => "deleted")
            .catch((error: unknown) => (error instanceof Error ? error.name : "thrown"));
        expect([
            segments.length,
            second?.previous === first?.digest,
            first?.previous,
            deleting === "deleted",
            (await bucket.head(first?.file ?? ""))?.retainUntil?.getTime(),
        ]).toEqual([2, true, null, false, START + 365 * DAY]);
    },
);

test.for(TEST_DIALECTS)(
    "compact a %s unlocked kind's small segments and expire them past a scope's retention",
    async (dialect) => {
        const fixture = await open(dialect, [reading], {
            policy: async (_kind, scope) =>
                scope === "space-1"
                    ? { flush: { maxAge: MINUTE, maxRows: 4 }, retention: 2 * DAY }
                    : undefined,
        });
        const { store, database } = fixture;

        // flush one reading at a time as each comes due by age
        for (const minute of [1, 2, 3]) {
            await store.append(reading, [
                measured(`r${String(minute)}`, minute, "db.bytes", minute),
            ]);
            fixture.now = START + (minute + 1) * MINUTE;
            await fixture.settle();
        }
        const compacted = await database.database
            .select()
            .from(eventSegment)
            .orderBy(eventSegment.from);
        const read = await store.query(reading, { scope: "space-1" });

        // three days later every segment is past the scope's retention
        fixture.now = START + 3 * DAY;
        await fixture.settle();
        expect([
            compacted.map((segment) => segment.rows),
            read.events.map((event) => event.id),
            (await database.database.select().from(eventSegment)).length,
            (await store.query(reading, { scope: "space-1" })).events,
        ]).toEqual([[2, 1], ["r1", "r2", "r3"], 0, []]);
    },
);

test.for(TEST_DIALECTS)("refuse a %s condition naming a key the kind lacks", async (dialect) => {
    const { store } = await open(dialect);

    await expect(
        store.query(reading, { scope: "space-1", where: Filter.parse('owner = "x"') }),
    ).rejects.toThrow("condition names no field owner");
});

test("read a row of a kind's table as its event with the kind's keys, refusing a key outside them", () => {
    const event = {
        scope: "space-1",
        id: "reading-1",
        source: "space-1",
        time: START * 1000,
        keys: { meter: "power", resource: "machine-1", value: 42 },
        data: { unit: "W" },
    };
    const row = { ...reading.row(event), sequence: 1 };

    // read the row back with its keys typed by the kind
    const read = reading.event(row);
    expect([read, read.keys.value + 1]).toEqual([event, 43]);

    // refuse a key value the kind does not accept
    expect(() => reading.event({ ...row, value: null })).toThrow(schema.Error);
});

test("refuse an event kind with an invalid name, a reserved key or a subject that is not a text key", () => {
    const declare =
        (
            name: string,
            keys: Readonly<Record<string, schema.Schema<string | number | null>>>,
            extra: object = {},
        ) =>
        () =>
            defineEventKind(
                {
                    name,
                    description: "Invalid.",
                    keys: schema.object(keys),
                    data: schema.object({ address: schema.string().exactOptional() }),
                    delivery: "exactly-once",
                    policy: { flush: { maxAge: MINUTE, maxRows: 10 }, retention: MINUTE },
                    ...extra,
                },
                { package: meters },
            );

    expect(declare("Reading", {})).toThrow("event kind name must be kebab-case: Reading");
    expect(declare("reading", { time: schema.string() })).toThrow(
        "event kind reading names a key time every event has",
    );
    expect(declare("reading", { actor: schema.number() }, { subject: "actor" })).toThrow(
        "event kind reading names its subject by a text key",
    );
});

/** Requests of a space: their status and their open-ended attributes, flushed every two. */
const request = defineEventKind(
    {
        name: "request",
        description: "A request served in a space.",
        keys: schema.object({
            status: schema.number().int(),
            attributes: schema.record(
                schema.string(),
                schema.union([schema.string(), schema.number(), schema.boolean()]),
            ),
        }),
        data: schema.object({}),
        delivery: "exactly-once",
        policy: { flush: { maxAge: 60 * MINUTE, maxRows: 2 }, retention: DAY },
    },
    { package: meters },
);

/** Read each route's single step of a series grouped by route. */
function byRoute(series: readonly Series[]): Record<string, number | undefined> {
    return Object.fromEntries(
        series.map((entry) => [
            String(entry.group["attributes.http.route"]),
            entry.steps[0]?.value,
        ]),
    );
}

/** A request some minutes after the start, on a route by a user. */
function served(id: string, minute: number, route: string, user: string, status = 200) {
    return {
        scope: "space-1",
        id,
        time: at(minute),
        keys: { status, attributes: { "http.route": route, user } },
        data: {},
    };
}

test.for(TEST_DIALECTS)(
    "select, group and count the %s events of a map key's entries newest first, hot and flushed alike",
    async (dialect) => {
        // flush four requests into segments and keep one hot
        const fixture = await open(dialect, [request]);
        const { store } = fixture;
        await store.append(request, [
            served("a", 1, "/notes", "ada"),
            served("b", 2, "/tasks", "ada"),
            served("c", 3, "/notes", "bob", 500),
            served("d", 4, "/notes", "ada"),
        ]);
        await fixture.settle();
        await store.append(request, [served("e", 5, "/notes", "cy")]);

        // page the notes requests newest first, two at a time
        const notes = {
            scope: "space-1",
            where: Filter.parse('attributes.http.route = "/notes"', { isRelational: false }),
        };
        const first = await store.query(request, notes, { limit: 2, order: "descending" });
        const second = await store.query(request, notes, {
            limit: 2,
            order: "descending",
            ...(first.cursor === undefined ? {} : { after: first.cursor }),
        });

        // count requests per route and the distinct users per route, and refuse a name kept twice
        const routes = await store.series(
            request,
            { scope: "space-1" },
            {
                fold: "count",
                group: ["attributes.http.route"],
            },
        );
        const users = await store.series(
            request,
            { scope: "space-1" },
            {
                measure: "attributes.user",
                fold: "unique",
                group: ["attributes.http.route"],
            },
        );
        const failed = await store.query(request, {
            scope: "space-1",
            where: Filter.parse('attributes.http.route = "/notes" AND status >= 500', {
                isRelational: false,
            }),
        });
        const twice = await EventFixture.open(dialect, [request, request]).then(
            () => "opened",
            (error: unknown) => String(error),
        );

        expect({
            pages: [first.events.map((event) => event.id), second.events.map((event) => event.id)],
            routes: byRoute(routes),
            users: byRoute(users),
            failed: failed.events.map((event) => event.id),
            found: store.kind("request") === request,
            twice,
        }).toEqual({
            pages: [
                ["e", "d"],
                ["c", "a"],
            ],
            routes: { "/notes": 4, "/tasks": 1 },
            users: { "/notes": 3, "/tasks": 1 },
            failed: ["c"],
            found: true,
            twice: "TypeError: two event kinds named request share one database",
        });
    },
);
