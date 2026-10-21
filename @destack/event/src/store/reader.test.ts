import { TEST_DIALECTS } from "@destack/db/test";
import { Package } from "@destack/package";
import { schema } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { EventFixture, testKind } from "../test/index.ts";
import { EventReader } from "./reader.ts";

/** How long the readers hold unread events: one minute. */
const HOLD = 60_000;

/** The package declaring the test's events. */
const meters = Package.parse({
    id: "package-019f5530-8000-7000-8000-000000000311",
    name: "@acme/meters",
    version: "2026.10.0",
});

/** Readings of a meter. */
const reading = testKind({
    package: meters,
    name: "reading",
    keys: schema.object({ meter: schema.string() }),
    data: schema.object({}),
    delivery: "at-most-once",
    policy: { flush: { maxAge: HOLD, maxRows: 100 }, retention: HOLD },
});

test.for(TEST_DIALECTS)(
    "place each key of a reader family once, list the placed keys in order, and move each key apart on %s",
    async (dialect) => {
        const fixture = await EventFixture.open(dialect, [reading]);
        onTestFinished(() => fixture[Symbol.asyncDispose]());
        const { database } = fixture.database;
        const now = fixture.clock.now();
        const usage = new EventReader("usage", HOLD);

        // place b, then a twice, beside another family's key
        await usage.at("b").place(now, database);
        await usage.at("a").place(now, database);
        await usage.at("a").place(now, database);
        await new EventReader("endpoint", HOLD).at("c").place(now, database);

        // move a alone, leaving b at the start
        await usage.at("a").move(0, 7, now, database);
        expect({
            keys: await usage.keys(database),
            places: [await usage.at("a").after(database), await usage.at("b").after(database)],
        }).toEqual({ keys: ["a", "b"], places: [7, 0] });
    },
);

test.for(TEST_DIALECTS)(
    "read a claimed page again unchanged after later appends, keep the first claim, and end it when the reader moves on %s",
    async (dialect) => {
        const fixture = await EventFixture.open(dialect, [reading]);
        onTestFinished(() => fixture[Symbol.asyncDispose]());
        const { database } = fixture.database;
        const { store } = fixture;
        const now = fixture.clock.now();
        const metering = new EventReader("metering", HOLD).at("space-1");
        const append = async (id: string) =>
            store.append(reading, [
                { scope: "space-1", id, time: now * 1000, keys: { meter: "requests" }, data: {} },
            ]);
        const filter = { scopes: ["space-1"] } as const;

        // claim the page holding the first two readings, then append a third
        await metering.place(now, database);
        await append("reading-1");
        await append("reading-2");
        const after = await metering.after(database);
        const open = await store.changes(reading, filter, after);
        const claim = await metering.claim(after, open.sequence, now, database);
        const again = await metering.claim(after, open.sequence + 1, now, database);
        await append("reading-3");

        // read the claimed page alone, then move past it and end the claim
        const kept = await metering.claimed(database);
        const claimed = await store.range(reading, filter, after, open.sequence);
        await metering.move(after, open.sequence, now, database);
        expect({
            claim,
            again,
            kept,
            claimed: claimed.map((event) => event.id),
            unclaimed: await metering.claimed(database),
            next: (await store.changes(reading, filter, open.sequence)).events.map(
                (event) => event.id,
            ),
        }).toEqual({
            claim: { through: open.sequence, claimedAt: now },
            again: { through: open.sequence, claimedAt: now },
            kept: { through: open.sequence, claimedAt: now },
            claimed: ["reading-1", "reading-2"],
            unclaimed: undefined,
            next: ["reading-3"],
        });
    },
);
