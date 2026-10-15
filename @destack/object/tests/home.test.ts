import { reconciliation, testCallKey } from "@destack/runtime/test";
import { callTables, testEvents, serversOf } from "../src/test/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { accessTables, Authorizer, none, principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { defineDatabase, eq } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { Clock, present, schema } from "@destack/schema";
import { definePackage } from "@destack/runtime";
import type { ClientOptions } from "@destack/runtime/client";
import { RequestId } from "@destack/runtime/request";
import { Feed, Replica, Scope } from "@destack/sync";
import { defineObject, field } from "../src/index.ts";
import { ObjectClient } from "../src/client/index.ts";
import { ObjectStore } from "../src/server/index.ts";
import { serveObjects } from "./fixture/device.ts";
import { spacePrincipal, unmoved } from "./fixture/space.ts";
import { userContext, users } from "./fixture/user.ts";

/** How long a commit may take to reach a replica: 5 s, far above the milliseconds it takes. */
const REPLICA_MILLISECONDS = 5000;

/** The person whose preferences live at their home. */
const personId = schema.identifier("person").parse("person-01996ab0-0000-7000-8000-000000000031");

/** The person's room, whose machine replicates the person's preferences. */
const roomId = schema.identifier("room").parse("room-01996ab0-0000-7000-8000-000000000032");

/** People, the scopes their preferences live in. */
const person = defineObject({
    name: "person",
    plural: "people",
    scope: "universe",
    isScope: true,
    partition: true,
    fields: {},
    permissions: { read: none(), write: none() },
}).methods();

/** Rooms within a person, whose replicas keep the person's preferences. */
const room = defineObject({
    name: "room",
    plural: "rooms",
    scope: person,
    isScope: true,
    fields: {},
    permissions: { read: none(), write: none() },
}).methods();

/** Preferences a person sets, handed down to their rooms. */
const preference = defineObject({
    name: "preference",
    plural: "preferences",
    scope: [person, room],
    inherited: {},
    fields: { name: field.string(), value: field.string() },
    permissions: { read: none(), write: none() },
}).methods((method) => ({
    create: method.create("write"),
    update: method.update("write", { fields: ["value"] }),
}));

/** The package serving the preferences. */
const preferences = definePackage({ objects: { preference } });

/** Refuse a change sent over the uplink, as the client sends its changes home itself. */
const refuseChanges = () => Promise.reject(new Error("the replica receives no changes"));

/** Serve alice's preferences at their home and a replica at her room's machine. */
async function serveHomes() {
    // keep alice's person and room at home, owned by her
    const homeStorage = await TestDatabase.create(
        "sqlite",
        defineDatabase({
            name: "main",
            tables: [...callTables, ...person.tables, ...room.tables, ...preference.tables],
        }),
        { isMigrated: true },
    );
    const replicateStorage = await TestDatabase.create(
        "sqlite",
        defineDatabase({
            name: "main",
            tables: [...callTables, ...accessTables],
            replicated: [person.table, room.table, preference.table],
        }),
        { isMigrated: true },
    );
    onTestFinished(async () => {
        await Promise.all([homeStorage.close(), replicateStorage.close()]);
    });
    const alice = principal.user.reference(Scope.universe.id, users.alice);
    const fixture = new AccessFixture(homeStorage.database);
    await fixture.replicateScope(person.reference(Scope.universe.id, personId), "own");
    await fixture.replicateScope(room.reference(personId, roomId));
    await fixture.bindOwner(person.reference(Scope.universe.id, personId), alice);
    await homeStorage.database
        .insert(person.table)
        .values({ id: personId, scope: Scope.universe.id, createdAt: 1, updatedAt: 1 });
    await homeStorage.database
        .insert(room.table)
        .values({ id: roomId, scope: personId, createdAt: 1, updatedAt: 1 });

    // serve the preferences at home
    const home = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(personId),
        servers: serversOf([preference]),
        policies: [person, room],
        database: homeStorage.database,
        callKey: testCallKey,
        events: await testEvents(homeStorage.database),
        origin: { package: preference.package },
    });

    // replicate the room's own chain from home into the room's machine and serve the replica
    const feed = new Feed(
        homeStorage.database,
        [...accessTables, person.table, room.table, preference.table],
        {
            clock: Clock.system,
        },
    );
    const replica: ObjectStore = new ObjectStore({
        chain: async () => ({ kind: "follower" }),
        clock: Clock.system,
        principal: spacePrincipal(roomId),
        servers: serversOf([preference]),
        policies: [person, room],
        database: replicateStorage.database,
        uplink: {
            receive: refuseChanges,
            stream: (subscription, signal) =>
                feed.subscribe(
                    replica.source.buildReplica(subscription).queries,
                    subscription.after,
                    signal,
                ),
        },
        callKey: testCallKey,
        events: await testEvents(replicateStorage.database),
        origin: { package: preference.package },
    });
    await followChain(replica, replicateStorage.database, homeStorage.database);

    // reach each server as alice, recording the requests each receives
    const writes: string[] = [];
    const recorded = (
        name: string,
        server: { readonly fetch: (request: Request) => Promise<Response> },
    ) =>
        ({
            url: "https://objects.test",
            headers: { authorization: `Bearer ${users.alice}` },
            fetch: (request: Request) => {
                writes.push(`${name} ${new URL(request.url).pathname}`);

                return server.fetch(request);
            },
        }) satisfies ClientOptions;
    const homeServer = serveObjects(home.implement(preferences, [preference], []), personId);
    const replicateServer = serveObjects(replica.implement(preferences, [preference], []), roomId);

    return {
        home,
        homeDatabase: homeStorage.database,
        writes,
        homeEndpoint: recorded("home", homeServer.server),
        replicateEndpoint: recorded("replica", replicateServer.server),
    };
}

/** Follow the replicas a room's store derives, the universe's rows and then its chain via them. */
async function followChain(
    replica: ObjectStore,
    database: TestDatabase["database"],
    home: TestDatabase["database"],
): Promise<void> {
    // reconcile each listed replica, keeping it followed until the test ends
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

    // follow the universe's rows first, then the chain via the scopes they bring, each to the home's head
    const head = await home.log.position();
    const followed = new Set<string>();
    for (const name of [roomId, Authorizer.viaReplica(roomId)]) {
        const key = present(
            (await follower.list()).find((entry) => entry === `${name} ${Scope.universe.id}`),
            `the replica key of ${name}`,
        );
        followed.add(key);
        following.push(follower.reconcile(key, reconciliation(controller.signal)));
        await Replica.wait(database, Scope.universe.id, head, controller.signal);
    }
}

/** Open alice's client of her room, reaching her person's home for its rows. */
async function openRoom(served: Awaited<ReturnType<typeof serveHomes>>) {
    // keep the room's preferences on the device
    const storage = await TestDatabase.create("sqlite", ObjectClient.tables({ preference }), {
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
        clock: Clock.system,
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
    const [created] = await home.mutate(userContext(users.alice, personId).caller(), {
        id: RequestId.create(Date.now()),
        calls: [preference.call("create", { scope: personId, name: "theme", value })],
    });

    return present(created, "the created theme").id;
}

/** Wait until a client's database meets a condition, failing after the replica's bound. */
async function waitFor(client: ObjectClient, condition: () => Promise<boolean>): Promise<void> {
    const isMet = await client.database.log.until(
        condition,
        AbortSignal.timeout(REPLICA_MILLISECONDS),
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

/** A preference's value as a client's replica keeps it. */
const Valued = schema.looseObject({ value: schema.string() });

test("send a write of an enclosing scope's row to its home, predicted on the room's replica and rebased once the home's commit streams back through it", async () => {
    // wait for alice's theme in her room's replica
    const served = await serveHomes();
    const id = await setTheme(served.home, "light");
    const { client, failures } = await openRoom(served);
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
        writes: served.writes.filter((write) => !write.endsWith("/replica/stream")),
        failures,
    }).toEqual({
        predicted: "dark",
        replicated: "dark",
        home: "dark",
        writes: ["home /replica/push", "replica /replica/watermark"],
        failures: [],
    });
});

test("refuse a mutation changing rows of two homes", async () => {
    // change alice's theme at her person's home and create a preference in her room at once
    const served = await serveHomes();
    const id = await setTheme(served.home, "light");
    const { client } = await openRoom(served);
    await replicatedTo(client, id);
    const submitted = client.mutation(async (mutation) => {
        await mutation.call(preference).update({ id, value: "dark" });
        await mutation.call(preference).create({ name: "font", value: "serif" });
    });

    await expect(submitted.predicted).rejects.toMatchObject({
        code: "BAD_REQUEST",
        message: `a mutation changes rows of one home, and its calls change rows of ${personId} and ${roomId}`,
    });
});

test("send the undo of a write of an enclosing scope's row to its home", async () => {
    // switch alice's theme to dark through her room's client, then undo it
    const served = await serveHomes();
    const id = await setTheme(served.home, "light");
    const { client, failures } = await openRoom(served);
    await replicatedTo(client, id);
    await client.mutate(preference).update({ id, value: "dark" }).confirmed;
    await client.undo().confirmed;
    await settled(client);

    // run the undo at the person's home as well
    expect({
        home: await homeTheme(served.homeDatabase, id),
        replicated: Valued.parse(await client.row(client.database, preference, id)).value,
        writes: served.writes.filter((write) => !write.endsWith("/replica/stream")),
        failures,
    }).toEqual({
        home: "light",
        replicated: "light",
        writes: [
            "home /replica/push",
            "replica /replica/watermark",
            "home /replica/push",
            "replica /replica/watermark",
        ],
        failures: [],
    });
});
