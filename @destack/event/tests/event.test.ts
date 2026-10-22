import { TestDatabase } from "@destack/db/test";
import { Filter, sql } from "@destack/db";
import type { JsonCondition } from "@destack/db";
import { Package } from "@destack/package";
import { schema } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { eventSegment } from "../src/archive/catalog.ts";
import { type EventKind, Histogram, type Series } from "../src/index.ts";
import { EventFixture, testKind } from "../src/test/index.ts";

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

/** A retention setting, which the test's reader reads two days for space-1 and a day elsewhere. */
const KEEP = { name: "keep" };

/** The description of a space's meter readings, stored exactly once. */
const READING = {
    package: meters,
    name: "reading",
    keys: schema.object({
        meter: schema.string(),
        resource: schema.string(),
        value: schema.number(),
    }),
    data: schema.object({ unit: schema.string(), note: schema.string().exactOptional() }),
    delivery: "exactly-once",
} as const;

/** Readings of meters, flushed at three rows and retained a day. */
const reading = testKind({
    ...READING,
    policy: { flush: { maxAge: 60 * MINUTE, maxRows: 3 }, retention: DAY },
});

/** Calls of a space, locked, the caller's address sealed. */
const call = testKind({
    package: meters,
    name: "call",
    keys: schema.object({ actor: schema.string(), method: schema.string() }),
    data: schema.object({
        address: schema.sensitive(schema.string(), "personal").exactOptional(),
        token: schema.sensitive(schema.string()).exactOptional(),
    }),
    delivery: "exactly-once",
    policy: { flush: { maxAge: 60 * MINUTE, maxRows: 2 }, retention: 365 * DAY },
    isLocked: true,
    subject: ["actor"],
});

/** Log lines of a space: stored at most once, their severity unknown for some. */
const line = testKind({
    package: meters,
    name: "line",
    keys: schema.object({ severity: schema.number().int().nullable() }),
    data: schema.object({ message: schema.string() }),
    delivery: "at-most-once",
    policy: { flush: { maxAge: 60 * MINUTE, maxRows: 100 }, retention: DAY },
});

/** The driver's failure writing a log line once its table is gone, by dialect. */
const MISSING_LINE_TABLE = {
    sqlite: { name: "SQLiteError", message: "no such table: acme__meters__event_line" },
    postgresql: {
        name: "PostgresError",
        message: 'relation "acme__meters__event_line" does not exist',
    },
};

/** Open events of some kinds, closed as the test finishes. */
async function open(kinds: readonly EventKind[] = [reading]) {
    const fixture = await EventFixture.open(kinds);
    onTestFinished(() => fixture[Symbol.asyncDispose]());
    fixture.clock.set(START);

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

test("page a scope's events a condition selects in time order, appending one again changing nothing and refusing one with other contents", async () => {
    const { store } = await open();

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
    await expect(store.append(reading, [measured("a", 1, "db.bytes", 99)])).rejects.toMatchObject({
        code: "CONFLICT",
        message: `event a of ${reading.key} was stored with other contents`,
    });

    // read the scope's database readings one per page, then every reading of at least 20
    const where = Filter.parse('meter = "db.bytes"');
    const first = await store.query(reading, { scopes: ["space-1"], where }, { limit: 1 });
    const second = await store.query(
        reading,
        { scopes: ["space-1"], where },
        first.cursor === undefined ? { limit: 1 } : { after: first.cursor, limit: 1 },
    );
    const large = await store.query(reading, {
        scopes: ["space-1"],
        where: Filter.parse("value >= 20"),
    });
    expect([
        first.events.map((event) => [event.id, event.keys.value]),
        second.events.map((event) => event.id),
        second.cursor,
        large.events.map((event) => event.id),
    ]).toEqual([[["a", 10]], ["c"], undefined, ["b", "c"]]);
});

test("flush a scope's hot events at the row cap into segments that queries read like hot ones", async () => {
    const fixture = await open();
    const { store, database } = fixture;

    // append five readings and flush at the policy's three rows, an hour before any is due by age
    await store.append(
        reading,
        [1, 2, 3, 4, 5].map((minute) => measured(`r${String(minute)}`, minute, "db.bytes", minute)),
    );
    await fixture.settle();
    const hot = await database.database.select({ id: reading.table.id }).from(reading.table);
    const segments = await database.database.select().from(eventSegment);

    // the readings read back across the segment and the hot rows by time range and condition
    const ranged = await store.query(reading, {
        scopes: ["space-1"],
        where: { time: { gte: at(2), lt: at(5) } },
    });
    const selected = await store.query(reading, {
        scopes: ["space-1"],
        where: Filter.parse("value != 4"),
    });

    // read one event back by its identity alone, flushed or hot, and none for an unknown one
    const gotten = await Promise.all(
        ["r2", "r5", "r9"].map((id) => store.get(reading, { scopes: ["space-1"], id })),
    );
    expect([
        hot.map((row) => row.id),
        segments.map((segment) => [segment.rows, segment.keys["meter"]]),
        ranged.events.map((event) => event.id),
        selected.events.map((event) => event.id),
        gotten.map((event) => event?.id),
    ]).toEqual([
        ["r4", "r5"],
        [[3, ["db.bytes"]]],
        ["r2", "r3", "r4"],
        ["r1", "r2", "r3", "r5"],
        ["r2", "r5", undefined],
    ]);
});

test("read the events of several scopes, hot and flushed, in one time order", async () => {
    const fixture = await open();
    const { store } = fixture;

    // flush three readings of the space, leave one hot, and add two of the organisation
    const within = (id: string, minute: number) => ({
        ...measured(id, minute, "db.bytes", minute),
        scope: "space-2",
    });
    const organisation = (id: string, minute: number) => ({
        ...measured(id, minute, "db.bytes", minute),
        scope: "organisation-1",
    });
    await store.append(reading, [within("w1", 1), within("w2", 3), within("w3", 5)]);
    await fixture.settle();
    await store.append(reading, [within("w4", 6), organisation("o1", 2), organisation("o2", 4)]);

    // read only the organisation, and with the space within it
    const ids = async (scopes: readonly [string, ...string[]]) =>
        (await store.query(reading, { scopes })).events.map((event) => [event.scope, event.id]);
    expect([await ids(["organisation-1"]), await ids(["organisation-1", "space-2"])]).toEqual([
        [
            ["organisation-1", "o1"],
            ["organisation-1", "o2"],
        ],
        [
            ["space-2", "w1"],
            ["organisation-1", "o1"],
            ["space-2", "w2"],
            ["organisation-1", "o2"],
            ["space-2", "w3"],
            ["space-2", "w4"],
        ],
    ]);
});

/** Messages of a package, their text searched. */
const message = testKind(
    {
        package: meters,
        name: "message",
        keys: schema.object({ text: schema.string() }),
        data: schema.object({}),
        delivery: "exactly-once",
        policy: { flush: { maxAge: 60 * MINUTE, maxRows: 2 }, retention: DAY },
    },
    ["text"],
);

test("search a scope's hot and flushed events by word prefixes, its segments pruned by their grams", async () => {
    const fixture = await open([message]);
    const { store } = fixture;

    // flush two messages and leave one hot
    const said = (id: string, minute: number, text: string) => ({
        scope: "space-1",
        id,
        time: at(minute),
        keys: { text },
        data: {},
    });
    await store.append(message, [said("a", 1, "Request timed out"), said("b", 2, "Cache warmed")]);
    await fixture.settle();
    await store.append(message, [said("c", 3, "Retry timeout reached")]);

    // search both alone and beside a condition, and for words no segment has
    const found = async (where: JsonCondition) =>
        (await store.query(message, { scopes: ["space-1"], where })).events.map(
            (event) => event.id,
        );
    expect({
        timed: await found({ SEARCH: "time" }),
        timedOut: await found({ SEARCH: "timed out" }),
        recent: await found({ AND: [{ SEARCH: "time" }, { time: { gte: at(3) } }] }),
        cache: await found({ SEARCH: "cach" }),
        none: await found({ SEARCH: "zebra" }),
    }).toEqual({
        timed: ["a", "c"],
        timedOut: ["a"],
        recent: ["c"],
        cache: ["b"],
        none: [],
    });
});

test("sweep a scope's segment files without a catalog row once an hour old, daily, retaining catalogued and recent ones", async () => {
    const fixture = await open();
    const { store, bucket } = fixture;
    const directory = `events/${meters.id}/reading/space-1`;

    // flush a segment, then leave an uncatalogued file as a crashed write would
    await store.append(
        reading,
        [1, 2, 3].map((minute) => measured(`r${String(minute)}`, minute, "db.bytes", minute)),
    );
    await fixture.settle();
    await bucket.put(`${directory}/orphan.parquet`, "lost");

    // advance a day, leave a fresh uncatalogued file as a write in flight, then sweep
    fixture.clock.set(START + DAY);
    await bucket.put(`${directory}/inflight.parquet`, "writing");
    await fixture.settle();

    // retain the catalogued segment and the file in flight, deleting the orphan
    const catalogued = await fixture.database.database.select().from(eventSegment);
    const listed = await bucket.list({ prefix: `${directory}/` });
    expect(listed.files.map((file) => file.key).toSorted()).toEqual(
        [...catalogued.map((segment) => segment.file), `${directory}/inflight.parquet`].toSorted(),
    );
});

test("fold a measure per step and group across hot events in SQL and flushed ones", async () => {
    const fixture = await open();
    const { store } = fixture;

    // flush three readings and leave three hot, across two ten-minute steps
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
                { scopes: ["space-1"] },
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
        { scopes: ["space-1"], where: { time: { gte: at(0) } } },
        { fold: "count" },
    );
    const byScope = await store.series(
        reading,
        { scopes: ["space-1"] },
        { fold: "count", group: ["scope"] },
    );

    // count every event of the scope in one group, hot and flushed alike
    expect(byScope).toEqual([
        { group: { scope: "space-1" }, steps: [{ start: 0, value: 6, events: 6 }] },
    ]);
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
});

test("store a nullable whole-number key as an integer, compared and read back null", async () => {
    const { store } = await open([line]);

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
        scopes: ["space-1"],
        where: Filter.parse("severity >= 9"),
    });
    const every = await store.query(line, { scopes: ["space-1"] });
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
});

test("drop an at-most-once event whose write fails, capturing the failure", async () => {
    const fixture = await open([line]);
    const { store, database } = fixture;

    // append a log line once its table is gone
    await database.database.run(sql`DROP TABLE ${line.table}`);
    await store.append(line, [
        {
            scope: "space-1",
            id: "l",
            time: at(1),
            keys: { severity: 9 },
            data: { message: "hi" },
        },
    ]);

    // capture the failed write as the event package's exception
    expect(fixture.telemetry.take()).toEqual([
        { scope: "@destack/event", ...MISSING_LINE_TABLE[TestDatabase.target()] },
    ]);
});

test("store an exactly-once event only when its transaction commits, and an at-most-once one outside any", async () => {
    const { store, database } = await open([reading, line]);

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
        (await store.query(reading, { scopes: ["space-1"] })).events.map((event) => event.id),
        refused,
    ]).toEqual([
        ["kept"],
        `event kind ${line.key} is kept at most once and cannot join a caller's transaction`,
    ]);
});

test("seal a person's personal values at rest, drop secrets, and erase the values in hot and flushed events once the person is forgotten", async () => {
    const fixture = await open([call]);
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
    const rows = await database.database.select({ data: call.table.data }).from(call.table);
    await fixture.settle();
    await store.append(call, [made("e", 5, "ada")]);
    const before = await store.query(call, { scopes: ["space-1"] });

    // forget the person
    await store.forget(["ada"]);
    const after = await store.query(call, { scopes: ["space-1"] });
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
});

test("lock a locked kind's segments for its retention, chained by digest and never compacted", async () => {
    const fixture = await open([call]);
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
    const segments = await database.database.select().from(eventSegment).orderBy(eventSegment.from);
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
});

test("compact an unlocked kind's small segments and expire them past the retention their scope reads", async () => {
    // retain space-1's readings two days by their setting, every other scope's one
    const scoped = testKind({
        ...READING,
        policy: {
            flush: { maxAge: MINUTE, maxRows: 4 },
            retention: { setting: KEEP },
        },
    });
    const fixture = await EventFixture.open([scoped], async (setting, owner, scope) =>
        setting === KEEP && owner.id === meters.id && scope === "space-1" ? 2 * DAY : DAY,
    );
    onTestFinished(() => fixture[Symbol.asyncDispose]());
    fixture.clock.set(START);
    const { store, database } = fixture;

    // flush one reading at a time as each comes due by age
    for (const minute of [1, 2, 3]) {
        await store.append(scoped, [measured(`r${String(minute)}`, minute, "db.bytes", minute)]);
        fixture.clock.set(START + (minute + 1) * MINUTE);
        await fixture.settle();
    }
    const compacted = await database.database
        .select()
        .from(eventSegment)
        .orderBy(eventSegment.from);
    const read = await store.query(scoped, { scopes: ["space-1"] });

    // three days later every segment is past the scope's retention
    fixture.clock.set(START + 3 * DAY);
    await fixture.settle();
    expect([
        compacted.map((segment) => segment.rows),
        read.events.map((event) => event.id),
        (await database.database.select().from(eventSegment)).length,
        (await store.query(scoped, { scopes: ["space-1"] })).events,
    ]).toEqual([[2, 1], ["r1", "r2", "r3"], 0, []]);
});

test("refuse a condition with a key the kind lacks", async () => {
    const { store } = await open();

    await expect(
        store.query(reading, { scopes: ["space-1"], where: Filter.parse('owner = "x"') }),
    ).rejects.toThrow("condition has no field owner");
});

test("read a row of a kind's table as its event with the kind's keys, refusing a key outside them", () => {
    const event = {
        scope: "space-1",
        id: "reading-1",
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

test("refuse an event kind whose subject is not a text key or whose distribution is outside its data", () => {
    const declare =
        (keys: Readonly<Record<string, schema.Schema<string | number | null>>>, extra: object) =>
        () =>
            testKind({
                package: meters,
                name: "reading",
                keys: schema.object(keys),
                data: schema.object({ address: schema.string().exactOptional() }),
                delivery: "exactly-once",
                policy: { flush: { maxAge: MINUTE, maxRows: 10 }, retention: MINUTE },
                ...extra,
            });

    expect(declare({ actor: schema.number() }, { subject: ["actor"] })).toThrow(
        "event kind reading must have text subject keys",
    );
    expect(declare({}, { distribution: "histogram" })).toThrow(
        "event kind reading has no data entry histogram for its distribution",
    );
});

/** Timings of a space's operations, each recording a distribution of durations. */
const timing = testKind({
    package: meters,
    name: "timing",
    keys: schema.object({ operation: schema.string() }),
    data: schema.object({ histogram: Histogram.exactOptional() }),
    delivery: "exactly-once",
    policy: { flush: { maxAge: 60 * MINUTE, maxRows: 2 }, retention: DAY },
    distribution: "histogram",
});

/** A timing of an operation some minutes after the start, recording values in scale 0 buckets. */
function timed(id: string, minute: number, operation: string, histogram?: Histogram) {
    return {
        scope: "space-1",
        id,
        time: at(minute),
        keys: { operation },
        data: histogram === undefined ? {} : { histogram },
    };
}

/** A histogram at scale 0 of values in one bucket, where bucket i counts (2^i, 2^(i+1)]. */
function oneBucket(index: number, value: number, count = 1): Histogram {
    return {
        count,
        sum: value * count,
        min: value,
        max: value,
        scale: 0,
        zeroCount: 0,
        positive: { offset: index, counts: [count] },
        negative: { offset: 0, counts: [] },
    };
}

test("fold the distributions of a kind's events into quantiles per group, hot and flushed alike", async () => {
    // flush saves of 3, 12 and 12 and a load of 0, and leave a save of 5 and an empty one hot
    const fixture = await open([timing, reading]);
    const { store } = fixture;
    const zero = { ...oneBucket(0, 0), zeroCount: 1, positive: { offset: 0, counts: [] } };
    await store.append(timing, [
        timed("a", 1, "save", oneBucket(1, 3)),
        timed("b", 2, "save", oneBucket(3, 12, 2)),
        timed("d", 3, "load", zero),
    ]);
    await fixture.settle();
    await store.append(timing, [timed("c", 4, "save", oneBucket(2, 5)), timed("e", 5, "save")]);

    // fold the median and the 99th percentile per operation over one step
    const fold = async (name: "p50" | "p99") =>
        (
            await store.series(
                timing,
                { scopes: ["space-1"], where: { time: { gte: at(0) } } },
                { fold: name, group: ["operation"] },
            )
        )
            .toSorted((left, right) =>
                String(left.group["operation"]).localeCompare(String(right.group["operation"])),
            )
            .map((series) => [
                series.group["operation"],
                series.steps.map((step) => [step.start, step.value, step.events]),
            ]);

    // refuse a quantile of a kind without distributions, and one measuring a key
    const quantile = { scopes: ["space-1"] } as const;
    const unrecorded = store
        .series(reading, quantile, { fold: "p50" })
        .catch((error: unknown) => String(error));
    const keyed = store
        .series(timing, quantile, { fold: "p50", measure: "operation" })
        .catch((error: unknown) => String(error));

    // read each quantile as its bucket's geometric middle: 2^2.5 at rank 1.5, 2^3.5 at rank 2.97
    expect([await fold("p50"), await fold("p99"), await unrecorded, await keyed]).toEqual([
        [
            ["load", [[at(0), 0, 1]]],
            ["save", [[at(0), 2 ** 2.5, 3]]],
        ],
        [
            ["load", [[at(0), 0, 1]]],
            ["save", [[at(0), 2 ** 3.5, 3]]],
        ],
        `TypeError: a p50 series reads distributions, but ${reading.key} records none`,
        "TypeError: a p50 series reads distributions and cannot measure a key",
    ]);
});

/** Requests of a space: their status and their open-ended attributes, flushed every two. */
const request = testKind({
    package: meters,
    name: "request",
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
});

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

test("select, group and count the events of a map key's entries newest first, hot and flushed alike", async () => {
    // flush four requests into segments and leave one hot
    const fixture = await open([request]);
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
        scopes: ["space-1"] as const,
        where: Filter.parse('attributes.http.route = "/notes"', { isRelational: false }),
    };
    const first = await store.query(request, notes, { limit: 2, order: "descending" });
    const second = await store.query(request, notes, {
        limit: 2,
        order: "descending",
        ...(first.cursor === undefined ? {} : { after: first.cursor }),
    });

    // count requests per route and the distinct users per route, and refuse a kind listed twice
    const routes = await store.series(
        request,
        { scopes: ["space-1"] },
        {
            fold: "count",
            group: ["attributes.http.route"],
        },
    );
    const users = await store.series(
        request,
        { scopes: ["space-1"] },
        {
            measure: "attributes.user",
            fold: "unique",
            group: ["attributes.http.route"],
        },
    );
    const failed = await store.query(request, {
        scopes: ["space-1"],
        where: Filter.parse('attributes.http.route = "/notes" AND status >= 500', {
            isRelational: false,
        }),
    });
    const twice = await EventFixture.open([request, request]).then(
        () => "opened",
        (error: unknown) => String(error),
    );

    expect({
        pages: [first.events.map((event) => event.id), second.events.map((event) => event.id)],
        routes: byRoute(routes),
        users: byRoute(users),
        failed: failed.events.map((event) => event.id),
        found: store.kind(request.key) === request,
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
        twice: `TypeError: event kind ${request.key} is kept twice`,
    });
});
