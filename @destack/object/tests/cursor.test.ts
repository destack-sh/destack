import { testSimulation } from "../src/test/index.ts";
import { defineObject, field, ObjectDescription } from "../src/index.ts";
import { users } from "./fixture/user.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { AccessContext, Authorization, Caller, none, principal } from "@destack/access";
import { type Cursor, defineDatabase, Transaction } from "@destack/db";
import { reconciliation } from "@destack/db/test";
import { TestAccess } from "@destack/access/test";
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

/** Another space, whose ticks the cursor hands over as their own page. */
const OTHER_SPACE_ID = schema
    .identifier("space")
    .parse("space-01996ab0-0000-7000-8000-000000000004");

/** The time the ticks start at, in Unix milliseconds. */
const START = 1_800_000_000_000;

/** The milliseconds a cursor lets a page's first event wait. */
const BATCH = 60_000;

/** Ticks of spaces, each page of them taken by a cursor. */
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

test("hand a claimed page of ticks to the read one scope at a time, the same page after a failed attempt and to two hosts at once", async () => {
    // run in a world of its own, moving its time while the work waits
    const simulation = testSimulation(START);
    await (async () => {
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
            await store.mutate(AccessContext.of(Caller.of(spacePrincipal(spaceId)), clock.now()), {
                id: RequestId.create(world),
                calls: [
                    tick.call("record", {
                        spaceId,
                        items: [{ id, time: EventTime.of(clock.now()), count: 1, data: {} }],
                    }),
                ],
            });

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
        const take = (cursor: Cursor) => cursor.take(reconciliation());
        const options = { hold: { days: 1 }, batch: { minutes: 1 } };
        const two = tick.cursor(
            "count",
            async (page) => {
                pages.push({
                    host: "two",
                    ids: page.events.map((event) => event.id),
                    after: page.after,
                    through: page.through,
                    claimedAt: page.claimedAt,
                });
            },
            options,
        )(store);
        const one = tick.cursor(
            "count",
            async (page) => {
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
                    await take(two);
                }
            },
            options,
        )(store);

        // tick, fail a batch in, tick again, then retry
        const first = await record("1");
        await simulation.jump(START + BATCH);
        const failed = await take(one).catch((error: unknown) => String(error));
        await record("2");
        await take(one);

        // tick a batch later, then take the page on host one while host two takes it too
        await simulation.jump(START + 2 * BATCH);
        const third = await record("3");
        await simulation.jump(START + 3 * BATCH);
        await take(one);
        await take(two);

        // tick in another space, then hand its tick over as that space's page
        await openSpace(database, world, OTHER_SPACE_ID);
        const elsewhere = await record("4", OTHER_SPACE_ID);
        await take(two);

        // take the failed page twice unchanged, the next one on both hosts, then the other space's, host two moving past it
        const name = two.name;
        expect({
            failed,
            pages,
            place: [await database.log.checkpoint(name), await database.log.claimed(name)],
        }).toEqual({
            failed: "Error: the counter is unreachable",
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
                {
                    host: "two",
                    ids: ["4"],
                    after: third,
                    through: elsewhere,
                    claimedAt: START + 3 * BATCH,
                },
            ],
            place: [elsewhere, undefined],
        });
    })();
});
