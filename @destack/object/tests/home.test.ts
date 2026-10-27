import { testSimulation } from "../src/test/index.ts";
import { testCallKey } from "@destack/object/test";
import { reconciliation, TestDatabase } from "@destack/db/test";
import { FeedPublisher, testEvents, serversOf } from "../src/test/index.ts";
import { expect, onTestFinished, test, refusal } from "@destack/test";
import { accessTables, none, principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { defineDatabase, eq, Snapshot } from "@destack/db";
import { present, schema } from "@destack/schema";
import { definePackage } from "@destack/object";
import { type ClientOptions, createClient } from "@destack/object/client";
import { RequestId, Feed, Replica, Scope } from "@destack/sync";
import { defineObject, defineShape, field, Follow } from "../src/index.ts";
import { ObjectClient } from "../src/client/index.ts";
import { ObjectStore } from "../src/server/index.ts";
import { Bookmark } from "../src/procedure/bookmark.ts";
import { TestBearer, TestObjectStore } from "../src/test/index.ts";
import { spacePrincipal, unmoved, space } from "./fixture/space.ts";
import { userContext, users } from "./fixture/user.ts";
import { Environment } from "@destack/environment";
import { tablesOf } from "../src/stack/db.ts";
import objectPackage from "../src/package.ts";

/** How long a commit may take to arrive at a replica: 5 s, far above the milliseconds it takes. */
const REPLICA_MILLISECONDS = 5000;

/** The person whose preferences live at their home. */
const personId = schema.identifier("person").parse("person-01996ab0-0000-7000-8000-000000000031");

/** The person's room, whose machine replicates the person's preferences. */
const roomId = schema.identifier("room").parse("room-01996ab0-0000-7000-8000-000000000032");

/** A person outside the room's chain. */
const otherPersonId = schema
    .identifier("person")
    .parse("person-01996ab0-0000-7000-8000-000000000033");

/** How long a wait may take once the home confirms: 2 s, far below the feed's 10 s heartbeat. */
const CONFIRM_MILLISECONDS = 2000;

/** People, the scopes their preferences live in. */
const person = defineObject({
    name: "person",
    plural: "people",
    scope: "universe",
    isScope: true,
    partition: true,
    fields: {},
    permissions: { read: none(), write: none() },
});

/** Rooms within a person, whose replicas store the person's preferences. */
const room = defineObject({
    name: "room",
    plural: "rooms",
    scope: person,
    isScope: true,
    fields: {},
    permissions: { read: none(), write: none() },
});

/** Preferences a person sets, handed down to their rooms. */
const preference = defineObject(
    {
        name: "preference",
        plural: "preferences",
        scope: [person, room],
        inherited: {},
        fields: { name: field.string(), value: field.string() },
        permissions: { read: none(), write: none() },
    },
    (method) => ({
        create: method.create("write"),
        update: method.update("write", { fields: ["value"] }),
    }),
);

/** The package serving the preferences. */
const preferences = definePackage({ objects: { preference } });

/** Refuse a change sent over the uplink, as the client sends its changes home itself. */
const refuseChanges = () => Promise.reject(new Error("the replica receives no changes"));

/** Refuse a query sent over the uplink, as the replica sends none. */
const refuseQueries = () => Promise.reject(new Error("the replica answers no queries"));

/** Serve alice's preferences at their home and a replica at her room's machine. */
async function serveHomes(world: Environment, options: { readonly isFollowing?: boolean } = {}) {
    // store alice's person and room at home, with her as owner
    const homeStorage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({
                packages: [objectPackage, { objects: { person, room, preference }, tables: [] }],
                follows: [],
                tables: [],
            }),
        }),
        { environment: world, isMigrated: true },
    );
    const replicateStorage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({ packages: [objectPackage], follows: [HOME_FOLLOW], tables: [] }),
        }),
        { environment: world, isMigrated: true },
    );
    onTestFinished(async () => {
        await Promise.all([homeStorage.close(), replicateStorage.close()]);
    });
    const alice = principal.user.reference(Scope.universe.id, users.alice);
    const fixture = new AccessFixture(homeStorage.database, world);
    await fixture.replicateScope(person.reference(Scope.universe.id, personId), person.policy);
    await fixture.replicateScope(room.reference(personId, roomId), room.policy);
    await fixture.replicateOwner(person.reference(Scope.universe.id, personId), alice);
    await homeStorage.database.transaction(
        async (transaction) =>
            await transaction
                .insert(person.table)
                .values({ id: personId, scope: Scope.universe.id, createdAt: 1, updatedAt: 1 }),
    );
    await homeStorage.database.transaction(
        async (transaction) =>
            await transaction
                .insert(room.table)
                .values({ id: roomId, scope: personId, createdAt: 1, updatedAt: 1 }),
    );

    // serve the preferences at home
    const home = await TestObjectStore.receive(space.reference(Scope.universe.id, personId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(personId),
        servers: serversOf([preference]),
        policies: [person, room],
        database: homeStorage.database,
        callKey: testCallKey,
        events: await testEvents(homeStorage.database, world),
        origin: { package: preference.package },
    });

    // replicate the room's scope chain from home into the room's machine and serve the replica
    const feed = new Feed(
        homeStorage.database,
        [...accessTables, person.table, room.table, preference.table],
        {},
    );
    const replica: ObjectStore = await TestObjectStore.at(
        space.reference(Scope.universe.id, roomId),
        {
            chain: async () => ({ kind: "follower", parent: Scope.universe.id }),
            environment: world,
            principal: spacePrincipal(roomId),
            servers: serversOf([preference], {
                follows: [HOME_FOLLOW],
            }),
            policies: [person, room],
            database: replicateStorage.database,
            uplink: {
                ...FeedPublisher.of(
                    feed,
                    (subscription) => replica.source.buildReplica(subscription).queries,
                ),
                receive: refuseChanges,
                call: refuseQueries,
            },
            callKey: testCallKey,
            events: await testEvents(replicateStorage.database, world),
            origin: { package: preference.package },
        },
    );
    const follow = () => followChain(replica, replicateStorage.database, homeStorage.database);
    if (options.isFollowing !== false) {
        await follow();
    }

    // call each server as alice, recording the requests each receives
    const writes: string[] = [];
    const recorded = (
        name: string,
        server: { readonly fetch: (request: Request) => Promise<Response> },
    ) =>
        ({
            url: "https://objects.test",
            headers: {
                authorization: TestBearer.header(principal.user.reference("universe", users.alice)),
            },
            fetch: (request: Request) => {
                writes.push(`${name} ${new URL(request.url).pathname}`);

                return server.fetch(request);
            },
        }) satisfies ClientOptions;
    const homeServed = TestObjectStore.serve(home);
    const replicaServed = TestObjectStore.serve(replica);
    onTestFinished(async () => {
        await replicaServed[Symbol.asyncDispose]();
        await homeServed[Symbol.asyncDispose]();
    });

    return {
        environment: world,
        home,
        replica,
        homeDatabase: homeStorage.database,
        follow,
        writes,
        homeEndpoint: recorded("home", homeServed.server(preferences)),
        replicateEndpoint: recorded("replica", replicaServed.server(preferences)),
    };
}

/** Follow the replicas a room's store derives: the universe's rows, then the chains via them. */
async function followChain(
    replica: ObjectStore,
    database: TestDatabase["database"],
    home: TestDatabase["database"],
): Promise<void> {
    // reconcile each listed replica, following it until the test ends
    const follower = present(
        replica.controllers().find((each) => each.name === "replica"),
        "the replica controller",
    );
    const controller = new AbortController();
    const following: Promise<unknown>[] = [];
    onTestFinished(async () => {
        controller.abort();
        await Promise.allSettled(following);
    });

    // follow the listed replicas until the scopes their rows bring list no more, each to the home's head
    const head = await home.log.position();
    const followed = new Set<string>();
    for (let isGrowing = true; isGrowing;) {
        const keys = (await follower.list(Snapshot.live(database))).filter(
            (entry) => !followed.has(entry),
        );
        for (const key of keys) {
            followed.add(key);
            following.push(follower.reconcile(key, reconciliation(controller.signal)));
        }
        for (const key of keys) {
            const [name, scope] = key.split(" ");
            const listed = { name: present(name, "a name"), scope: present(scope, "a scope") };
            await Replica.wait(database, listed.scope, head, controller.signal, [listed]);
        }
        isGrowing = keys.length > 0;
    }
}

/** Open alice's client of her room, calling her person's home for its rows. */
async function openRoom(world: Environment, served: Awaited<ReturnType<typeof serveHomes>>) {
    // replicate the room's preferences on the device
    const storage = await TestDatabase.create(ObjectClient.tables({ preference }), {
        environment: world,
        dialect: "sqlite",
        isMigrated: true,
    });
    const client = await ObjectClient.open({
        database: storage.database,
        objects: { preference },
        scope: roomId,
        caller: principal.user.reference(Scope.universe.id, users.alice),
        endpoint: served.replicateEndpoint,
        reconnect: unmoved,
        homes: () => served.homeEndpoint,
        environment: world,
    });
    const live = client.query.preference.findMany().subscribe();

    // follow and push until the test ends
    const controller = new AbortController();
    const failures: unknown[] = [];
    const loops = client.run(controller.signal, (error) => failures.push(error));
    onTestFinished(async () => {
        controller.abort();
        await loops;
        await client.close();
        await storage.close();
    });
    await live.ready;

    return { client, failures };
}

/** Write alice's theme at her person's home, answering its identifier. */
async function setTheme(home: ObjectStore, value: string): Promise<string> {
    const [created] = await home.mutate(
        userContext(users.alice, personId, { environment: home.environment }).access(),
        {
            id: RequestId.create(home.environment),
            calls: [preference.call("create", { scope: personId, name: "theme", value })],
        },
    );

    return present(created, "the created theme").id;
}

/** Wait until a client's database meets a condition, failing after the replica's bound. */
async function waitFor(client: ObjectClient, condition: () => Promise<boolean>): Promise<void> {
    const isMet = await client.database.log.until(
        condition,
        client.environment.clock.timeout(REPLICA_MILLISECONDS),
    );
    if (!isMet) {
        throw new Error("the client's replica never met the condition");
    }
}

/** Wait until the client's replica has no queued mutation, its home's commits streamed back. */
async function settled(client: ObjectClient): Promise<void> {
    await waitFor(client, async () => {
        const { pending, executed } = (await client.inspect()).mutations;

        return pending + executed === 0;
    });
}

/** Wait until the client's replica has a preference. */
async function replicatedTo(client: ObjectClient, id: string): Promise<void> {
    await waitFor(
        client,
        async () => (await client.row(client.database, preference, id)) !== undefined,
    );
}

/** Read the theme's value at home. */
async function homeTheme(database: TestDatabase["database"], id: string): Promise<string> {
    const [row] = await database
        .select({ value: preference.table.value })
        .from(preference.table)
        .where(eq(preference.table.id, preference.identifier(id)));

    return present(row, "the theme at home").value;
}

/** A preference's value as a client's replica stores it. */
const Valued = schema.looseObject({ value: schema.string() });

/** The people, their rooms and their preferences, which a room's machine reads. */
const HOME = defineShape({
    name: "home",
    parameters: schema.object({}),
    objects: [person, room, preference],
    queries: () => [
        person.query({}),
        room.query({ scopes: "every" }),
        preference.query({ scopes: "every" }),
    ],
});

/** The follow of the people, their rooms and their preferences at the universe. */
const HOME_FOLLOW = Follow.of(HOME, { keys: [{ key: { scope: Scope.universe.id } }] });

test("send a write of an enclosing scope's row to its home, predicted on the room's replica and rebased once the home's commit streams back through it", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // wait for alice's theme in her room's replica
    const served = await serveHomes(world);
    const id = await setTheme(served.home, "light");
    const { client, failures } = await openRoom(world, served);
    await replicatedTo(client, id);

    // switch it to dark through the room's client
    const submitted = client.mutate(preference).update({ id, value: "dark" });
    await submitted.predicted;
    const predicted = Valued.parse(await client.row(client.database, preference, id)).value;
    await submitted.confirmed;
    await settled(client);
    const replicated = Valued.parse(await client.row(client.database, preference, id)).value;

    // predict dark at once, push it to the person's home, and settle once the room's replica has it
    expect({
        predicted,
        replicated,
        home: await homeTheme(served.homeDatabase, id),
        writes: served.writes.filter((write) => !write.endsWith("/store/stream")),
        failures,
    }).toEqual({
        predicted: "dark",
        replicated: "dark",
        home: "dark",
        writes: ["home /store/push", "replica /store/watermark"],
        failures: [],
    });
});

test("push a row the room's replica writes after a client subscribed to the client's stream", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // subscribe alice's client of her room, then write her theme at her person's home
    const served = await serveHomes(world);
    const { client, failures } = await openRoom(world, served);
    const id = await setTheme(served.home, "light");

    // receive the theme through the room's replica into the client's replica
    await replicatedTo(client, id);
    expect({
        value: Valued.parse(await client.row(client.database, preference, id)).value,
        failures,
    }).toEqual({ value: "light", failures: [] });
});

test("push a row of the room's ancestor to a client that subscribed before the room knew its chain", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // subscribe alice's client of her room before the room follows its chain
    const served = await serveHomes(world, { isFollowing: false });
    const opening = openRoom(world, served);
    await served.follow();
    const { client, failures } = await opening;

    // write her theme at her person's home, an ancestor of the room
    const id = await setTheme(served.home, "light");

    // receive the theme through the room's stream once the room learned its chain
    await replicatedTo(client, id);
    expect({
        value: Valued.parse(await client.row(client.database, preference, id)).value,
        failures,
    }).toEqual({ value: "light", failures: [] });
});

test("wait through a replica for a home commit outside its rows, the home confirming the position at once", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // commit a person outside the room's chain at home
    const served = await serveHomes(world);
    await served.homeDatabase.transaction(
        async (transaction) =>
            await transaction.insert(person.table).values({
                id: otherPersonId,
                scope: Scope.universe.id,
                createdAt: 1,
                updatedAt: 1,
            }),
    );
    const head = await served.homeDatabase.log.position();

    // wait through the room's store for the home's head, far within the feed's heartbeat
    const isCaughtUp = await served.replica.wait(
        { scope: personId, ...head },
        world.clock.timeout(CONFIRM_MILLISECONDS),
    );

    // arrive at the position by the home's confirmation
    expect(isCaughtUp).toBe(true);
});

test("wait on a store's log for a watermark it wrote, in a scope another database homes", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // take the head of the room's store, whose room row a replica of the home stores
    const served = await serveHomes(world);
    const head = await served.replica.database.log.position();

    // arrive at it on the store's log, far within the feed's heartbeat
    expect(
        await served.replica.wait(
            { scope: roomId, ...head },
            world.clock.timeout(CONFIRM_MILLISECONDS),
        ),
    ).toBe(true);
});

test("enter a scope after its replica reaches a commit its chain's scope wrote at a store homing another", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // serve the person's preferences through a store of the universe over the home's database
    const served = await serveHomes(world);
    const universal = await TestObjectStore.at(Scope.universe, {
        chain: async () => undefined,
        environment: served.home.environment,
        principal: Scope.universe,
        partitions: [personId],
        servers: serversOf([preference]),
        policies: [person, room],
        database: served.homeDatabase,
        callKey: testCallKey,
        events: await testEvents(served.homeDatabase, world),
        origin: { package: preference.package },
    });
    await using universalServed = TestObjectStore.serve(universal);

    // write alice's theme in her person's scope, keeping the watermark the write answers
    const bookmark = new Bookmark();
    const client = createClient(
        preferences,
        {
            ...universalServed
                .as(principal.user.reference("universe", users.alice))
                .endpoint(preferences),
            bookmark,
        },
        world,
    );
    const { id } = await client.preference.create({
        scope: personId,
        requestId: RequestId.create(served.home.environment),
        name: "theme",
        value: "dark",
    });

    // enter the room with the write's watermark, finding the theme in its replica as soon as the entry returns
    await served.replica.enter(
        userContext(users.alice, roomId, { environment: world, bookmark }),
        roomId,
    );
    const replicated = await served.replica.database
        .select({ id: preference.table.id })
        .from(preference.table)
        .where(eq(preference.table.id, id));
    expect({ scopes: bookmark.watermarks.map((each) => each.scope), replicated }).toEqual({
        scopes: [personId],
        replicated: [{ id }],
    });
});

test("enter a scope at once past a watermark of a scope outside its chain", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // observe a position far ahead in a person's scope the room's chain lacks
    const served = await serveHomes(world);
    const head = await served.homeDatabase.log.position();
    const bookmark = new Bookmark();
    bookmark.observe({ scope: otherPersonId, ...head, sequence: head.sequence + 1000 });

    // enter the room without waiting for the other person's scope
    const entered = await served.replica
        .enter(userContext(users.alice, roomId, { environment: world, bookmark }), roomId)
        .then(() => "entered");
    expect(entered).toBe("entered");
});

test("refuse a request to confirm a position of a scope with no home or replica in the store", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // ask the home to confirm a position of a scope it never saw
    const served = await serveHomes(world);
    const head = await served.homeDatabase.log.position();
    const subscription = {
        name: "nowhere",
        publication: "nowhere",
        scope: "space-nowhere",
        target: roomId,
        parameters: {},
    };
    const refused = await refusal(
        served.home.source.request(
            { ...subscription, confirming: { scope: "space-nowhere", ...head } },
            undefined,
        ),
    );

    // refuse the unknown scope
    expect(refused).toEqual([
        "NOT_FOUND",
        "HOME_OR_REPLICA_MISSING",
        "space-nowhere has no home or replica here",
    ]);
});

test("refuse a mutation changing rows of two homes", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // change alice's theme at her person's home and create a preference in her room at once
    const served = await serveHomes(world);
    const id = await setTheme(served.home, "light");
    const { client } = await openRoom(world, served);
    await replicatedTo(client, id);
    const submitted = client.mutation(async (mutation) => {
        await mutation.call(preference).update({ id, value: "dark" });
        await mutation.call(preference).create({ name: "font", value: "serif" });
    });

    expect(await refusal(submitted.predicted)).toEqual([
        "BAD_REQUEST",
        "HOMES_MIXED",
        `a mutation must change rows of one home, but its calls change rows of ${personId} and ${roomId}`,
    ]);
});

test("send the undo of a write of an enclosing scope's row to its home", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // switch alice's theme to dark through her room's client, then undo it
    const served = await serveHomes(world);
    const id = await setTheme(served.home, "light");
    const { client, failures } = await openRoom(world, served);
    await replicatedTo(client, id);
    await client.mutate(preference).update({ id, value: "dark" }).confirmed;
    await client.undo().confirmed;
    await settled(client);

    // run the undo at the person's home as well
    expect({
        home: await homeTheme(served.homeDatabase, id),
        replicated: Valued.parse(await client.row(client.database, preference, id)).value,
        writes: served.writes.filter((write) => !write.endsWith("/store/stream")),
        failures,
    }).toEqual({
        home: "light",
        replicated: "light",
        writes: [
            "home /store/push",
            "replica /store/watermark",
            "home /store/push",
            "replica /store/watermark",
        ],
        failures: [],
    });
});
