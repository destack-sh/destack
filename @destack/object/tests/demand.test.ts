import { accessTables, none, through } from "@destack/access";
import { asc, defineDatabase, lease } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { PackageHandle } from "@destack/runtime";
import { ControlLoop } from "@destack/runtime/control";
import { testCallKey } from "@destack/runtime/test";
import { Identifier, schema } from "@destack/schema";
import { Scope, type Subscription } from "@destack/sync";
import { expect, onTestFinished, test } from "@destack/test";
import { defineObject, defineShape, field, Follow, followKey } from "../src/index.ts";
import { defineServer, ObjectStore } from "../src/server/index.ts";
import { callTables, serversOf, testEvents } from "../src/test/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { VirtualClock } from "@destack/environment/simulation";

/** How long a test waits for a replica: 5 s, far above its milliseconds. */
const REPLICA_MILLISECONDS = 5000;

/** How long an acquired key stays followed in these tests: ten minutes. */
const TTL_MINUTES = 10;

/** The public space with the cards. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000031");

/** The space of the store following the cards on demand. */
const followerId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000032");

/** Cards of a space, read by anyone the space allows. */
const card = defineObject({
    name: "card",
    plural: "cards",
    scope: space,
    fields: { title: field.string() },
    permissions: { read: through("scope", "read"), write: none() },
});

/** The cards with one title. */
const titleShape = defineShape({
    name: "cards",
    parameters: schema.object({ title: schema.string() }),
    objects: [card],
    queries: ({ title }) => [card.query({ where: { title } })],
});

/** The follow of cards by title, each title followed ten minutes after its last acquisition. */
const byTitle = Follow.of(titleShape, { onDemand: { ttl: { minutes: TTL_MINUTES } } });

/** The follower's database: its own tables and the replicated cards. */
const followerDatabase = defineDatabase({
    name: "main",
    tables: [lease, followKey, ...accessTables],
    replicated: [card.table],
});

/** Serve the cards at home and a follower acquiring them by title on a clock the test moves. */
async function openStores(): Promise<{
    readonly follower: ObjectStore;
    readonly clock: VirtualClock;
    readonly titles: () => Promise<readonly string[]>;
    readonly keys: () => Promise<readonly string[]>;
}> {
    // create three cards at home in a public space
    const homeDatabase = await TestDatabase.create("sqlite", [...callTables, ...card.tables], {
        isMigrated: true,
    });
    const followerTest = await TestDatabase.create("sqlite", followerDatabase, {
        isMigrated: true,
    });
    onTestFinished(async () => {
        await Promise.all([homeDatabase.close(), followerTest.close()]);
    });
    await openSpace(homeDatabase.database, spaceId);
    for (const title of ["Milk", "Bread", "Eggs"]) {
        await homeDatabase.database.insert(card.table).values({
            id: Identifier.create("card"),
            scope: spaceId,
            title,
            createdAt: 1,
            updatedAt: 1,
        });
    }
    const clock = new VirtualClock(Date.UTC(2026, 9, 16));
    const home = new ObjectStore({
        chain: async () => undefined,
        clock,
        principal: spacePrincipal(spaceId),
        servers: serversOf([card]),
        database: homeDatabase.database,
        callKey: testCallKey,
        events: await testEvents(homeDatabase.database),
        origin: { package: card.package },
    });

    // follow the cards by title from the home as the follower's space, and run its replicas
    const decided = { subject: spacePrincipal(followerId), parent: Scope.universe.id };
    const publisher = {
        receive: () => Promise.reject(new Error("the follower sends no changes")),
        stream: (asked: Subscription, signal: AbortSignal) =>
            home.source.replicate(asked, asked.after, decided, signal),
    };
    const follower = new ObjectStore({
        chain: async () => ({ kind: "follower" }),
        clock,
        principal: spacePrincipal(followerId),
        servers: [defineServer(PackageHandle.of(card.package, {}), { follows: [byTitle] })],
        database: followerTest.database,
        origin: { package: card.package },
        publisher: () => publisher,
        uplink: publisher,
    });
    const controller = new AbortController();
    const loop = new ControlLoop(
        followerTest.database,
        follower.controllers().filter((each) => each.name === "replica"),
        {
            clock,
            report: (_controller, _key, error) => {
                throw error;
            },
        },
    );
    const running = loop.run(controller.signal);
    onTestFinished(async () => {
        controller.abort();
        await running;
    });

    // read the replicated titles and the followed keys
    const titles = async () =>
        (
            await followerTest.database
                .select({ title: card.table.title })
                .from(card.table)
                .orderBy(asc(card.table.title))
        ).map((row) => row.title);
    const keys = async () =>
        (
            await followerTest.database
                .select({ parameters: followKey.parameters })
                .from(followKey)
                .orderBy(asc(followKey.parameters))
        ).map((row) => row.parameters);

    return { follower, clock, titles, keys };
}

test("follow a shape's keys on demand: replicate only the acquired keys, renew a key past half its time, and evict lapsed keys on the next acquisition", async () => {
    const { follower, clock, titles, keys } = await openStores();
    const signal = AbortSignal.timeout(REPLICA_MILLISECONDS);

    // acquire one title, then a second, replicating only their cards
    await follower.acquire(byTitle, { key: { scope: spaceId, title: "Milk" } }, signal);
    const milk = await titles();
    await follower.acquire(byTitle, { key: { scope: spaceId, title: "Bread" } }, signal);
    const both = await titles();

    // leave a key acquired again within half its time unchanged
    const expiry = async () =>
        (await follower.database.select({ expiresAt: followKey.expiresAt }).from(followKey))
            .map((row) => row.expiresAt)
            .toSorted((left, right) => left - right);
    const before = await expiry();
    clock.advance(60_000);
    await follower.acquire(byTitle, { key: { scope: spaceId, title: "Milk" } }, signal);
    const renewedEarly = await expiry();

    // renew a key acquired again past half its time
    clock.advance((TTL_MINUTES / 2) * 60_000);
    await follower.acquire(byTitle, { key: { scope: spaceId, title: "Milk" } }, signal);
    const renewed = (await expiry()).map((at) => at - clock.now());

    // let both lapse, then acquire a third title, evicting the lapsed ones and their cards
    clock.advance(TTL_MINUTES * 60_000);
    await follower.acquire(byTitle, { key: { scope: spaceId, title: "Eggs" } }, signal);
    await follower.database.log.until(async () => (await titles()).length === 1, signal);

    // expect the replica to have exactly the keys followed at each step
    expect({
        milk,
        both,
        isUnchanged: renewedEarly.join() === before.join(),
        renewed,
        keys: await keys(),
        titles: await titles(),
    }).toEqual({
        milk: ["Milk"],
        both: ["Bread", "Milk"],
        isUnchanged: true,
        renewed: [(TTL_MINUTES - 6) * 60_000, TTL_MINUTES * 60_000],
        keys: ['{"title":"Eggs"}'],
        titles: ["Eggs"],
    });
});
