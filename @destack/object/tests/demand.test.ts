import { testSimulation } from "../src/test/index.ts";
import { CONTAINED, none, principal, through } from "@destack/access";
import { asc, defineDatabase, ControlLoop } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { PackageHandle } from "@destack/object";
import { testCallKey } from "@destack/object/test";
import { Identifier, schema } from "@destack/schema";
import { Scope, type Subscription } from "@destack/sync";
import { expect, onTestFinished, test } from "@destack/test";
import { defineObject, defineShape, field, Follow, followKey } from "../src/index.ts";
import { implementPackage, ObjectStore } from "../src/server/index.ts";
import { serversOf, subjectContext, testEvents } from "../src/test/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import type { Simulation } from "@destack/environment/simulation";
import { tablesOf } from "../src/stack/db.ts";
import objectPackage from "../src/package.ts";
import { TestObjectStore } from "../src/test/index.ts";

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

/** A machine of a space, whose row stands for it as a caller. */
const machine = defineObject({
    name: "machine",
    plural: "machines",
    scope: space,
    represents: principal.machine,
    fields: {},
    permissions: { read: through("scope", "read"), write: none() },
});

/** The row of one machine. */
const machineShape = defineShape({
    name: "machine",
    parameters: schema.object({ machine: schema.identifier("machine") }),
    objects: [machine],
    queries: ({ machine: id }) => [machine.query({ scopes: "every", where: { id } })],
});

/** The follow of a calling machine's row, acquired as it enters. */
const byMachine = Follow.of(machineShape, { onDemand: { ttl: { minutes: TTL_MINUTES } } });

/** The follower's database: its tables, the replicated cards and the callers' rows. */
const followerDatabase = defineDatabase({
    name: "main",
    ...tablesOf({ packages: [], follows: [byTitle, byMachine], tables: [] }),
});

/** Serve the cards at home and a follower acquiring them by title on a clock the test moves. */
async function openStores(): Promise<{
    readonly follower: ObjectStore;
    readonly simulation: Simulation;
    readonly titles: () => Promise<readonly string[]>;
    readonly keys: () => Promise<readonly string[]>;
    readonly machineId: Identifier<"machine">;
}> {
    // run in a world of its own, at a fixed time
    const simulation = testSimulation(Date.UTC(2026, 9, 16));
    const { clock } = simulation;
    const world = simulation.environment("test");

    // create three cards at home in a public space
    const homeDatabase = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { card, machine }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        { environment: world, isMigrated: true },
    );
    const followerTest = await TestDatabase.create(followerDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(async () => {
        await Promise.all([homeDatabase.close(), followerTest.close()]);
    });
    await openSpace(homeDatabase.database, world, spaceId);
    for (const title of ["Milk", "Bread", "Eggs"]) {
        await homeDatabase.database.transaction(
            async (transaction) =>
                await transaction.insert(card.table).values({
                    id: Identifier.create("card", world),
                    scope: spaceId,
                    title,
                    createdAt: 1,
                    updatedAt: 1,
                }),
        );
    }
    const machineId = Identifier.create("machine", world);
    await homeDatabase.database.transaction(
        async (transaction) =>
            await transaction
                .insert(machine.table)
                .values({ id: machineId, scope: spaceId, createdAt: 1, updatedAt: 1 }),
    );
    const home = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([card, machine]),
        database: homeDatabase.database,
        callKey: testCallKey,
        events: await testEvents(homeDatabase.database, world),
        origin: { package: card.package },
    });

    // follow the cards by title from the home as the follower's space, and run its replicas
    const decided = { subject: spacePrincipal(followerId), parent: Scope.universe.id };
    const publisher = {
        request: (asked: Subscription) => home.source.request(asked),
        receive: () => Promise.reject(new Error("the follower sends no changes")),
        call: () => Promise.reject(new Error("the follower sends no queries")),
        stream: (asked: Subscription, signal: AbortSignal) =>
            home.source.replicate(asked, asked.after, decided, signal),
    };
    const follower = await TestObjectStore.at(space.reference(Scope.universe.id, followerId), {
        chain: async () => ({ kind: "follower", parent: Scope.universe.id }),
        environment: world,
        principal: spacePrincipal(followerId),
        servers: [
            implementPackage(PackageHandle.of(card.package, {}), {
                follows: [byTitle],
                standing: [byMachine],
            }),
        ],
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

    return { follower, simulation, titles, keys, machineId };
}

test("follow a shape's keys on demand: replicate only the acquired keys, renew a key past half its time, and evict lapsed keys on the next acquisition", async () => {
    const { follower, simulation, titles, keys } = await openStores();
    const { clock } = simulation;
    const signal = () => clock.timeout(REPLICA_MILLISECONDS);

    // acquire one title, then a second, replicating only their cards
    await follower.acquire(byTitle, { key: { scope: spaceId, title: "Milk" } }, signal());
    const milk = await titles();
    await follower.acquire(byTitle, { key: { scope: spaceId, title: "Bread" } }, signal());
    const both = await titles();

    // leave a key acquired again within half its time unchanged
    const expiry = async () =>
        (await follower.database.select({ expiresAt: followKey.expiresAt }).from(followKey))
            .map((row) => row.expiresAt)
            .toSorted((left, right) => left - right);
    const before = await expiry();
    await simulation.jump(clock.now() + 60_000);
    await follower.acquire(byTitle, { key: { scope: spaceId, title: "Milk" } }, signal());
    const renewedEarly = await expiry();

    // renew a key acquired again past half its time
    await simulation.jump(clock.now() + (TTL_MINUTES / 2) * 60_000);
    await follower.acquire(byTitle, { key: { scope: spaceId, title: "Milk" } }, signal());
    const renewed = (await expiry()).map((at) => at - clock.now());

    // let both lapse, then acquire a third title, evicting the lapsed ones and their cards
    await simulation.jump(clock.now() + TTL_MINUTES * 60_000);
    await follower.acquire(byTitle, { key: { scope: spaceId, title: "Eggs" } }, signal());
    await follower.database.log.until(async () => (await titles()).length === 1, signal());

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

test("follow the row standing for a calling machine on demand as it enters, placing it inside its space, and nothing for a caller without a standing follow", async () => {
    const { follower, keys, machineId } = await openStores();
    const world = follower.environment;
    const caller = principal.machine.reference(spaceId, machineId);
    const user = principal.user.reference(Scope.universe.id, Identifier.create("user", world));

    // know the machine's space, as a replica of its chain does
    await openSpace(follower.database, world, spaceId);

    // enter as a user and as the machine, reading the scopes the machine's row places it inside
    const before = await follower.source.within(caller, follower.database);
    await follower.enter(subjectContext(user, undefined, { environment: world }), followerId);
    const afterUser = await keys();
    await follower.enter(subjectContext(caller, undefined, { environment: world }), followerId);
    const after = await follower.source.within(caller, follower.database);

    // expect the machine's row followed once it entered, placing it inside its space
    expect({ before, afterUser, keys: await keys(), after }).toEqual({
        before: [],
        afterUser: [],
        keys: [`{"machine":"${machineId}"}`],
        after: [{ ...space.reference(Scope.universe.id, spaceId), relation: CONTAINED }],
    });
});
