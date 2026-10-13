import { reconciliation, testCallKey } from "@destack/runtime/test";
import { callTables, testEvents } from "../src/test/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { accessTables, none, principal } from "@destack/access";
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
import { ObjectStore, Subscriber } from "../src/server/index.ts";
import { serveObjects } from "./fixture/device.ts";
import { spacePrincipal, unmoved } from "./fixture/space.ts";
import { userContext } from "./fixture/user.ts";

/** How long a commit may take to reach a copy: 5 s, far above the milliseconds it takes. */
const COPY_MILLISECONDS = 5000;

/** The person whose preferences live at their home. */
const personId = schema.identifier("person").parse("person-01996ab0-0000-7000-8000-000000000031");

/** The person's room, whose machine copies the person's preferences. */
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

/** Rooms within a person, whose copies keep the person's preferences. */
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
const refuseChanges = () => Promise.reject(new Error("the copy receives no changes"));

/** Serve alice's preferences at their home and a copy at her room's machine. */
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
    const copyStorage = await TestDatabase.create(
        "sqlite",
        defineDatabase({
            name: "main",
            tables: [...callTables, ...accessTables],
            copies: [person.table, room.table, preference.table],
        }),
        { isMigrated: true },
    );
    onTestFinished(async () => {
        await Promise.all([homeStorage.close(), copyStorage.close()]);
    });
    const alice = principal.user.reference(Scope.universe.id, "alice");
    const fixture = new AccessFixture(homeStorage.database);
    await fixture.copyScope(person.reference(Scope.universe.id, personId), "own");
    await fixture.copyScope(room.reference(personId, roomId));
    await fixture.copyOwner(person.reference(Scope.universe.id, personId), alice);
    await homeStorage.database
        .insert(person.table)
        .values({ id: personId, scope: Scope.universe.id, createdAt: 1, updatedAt: 1 });
    await homeStorage.database
        .insert(room.table)
        .values({ id: roomId, scope: personId, createdAt: 1, updatedAt: 1 });

    // serve the preferences at home
    const home = new ObjectStore({
        clock: Clock.system,
        principal: spacePrincipal(personId),
        objects: [preference],
        policies: [person, room],
        database: homeStorage.database,
        callKey: testCallKey,
        events: await testEvents(homeStorage.database),
        origin: { package: preference.package },
    });

    // copy the room's chain from home into the room's machine and serve the copy
    const feed = new Feed(
        homeStorage.database,
        [...accessTables, person.table, room.table, preference.table],
        {
            clock: Clock.system,
        },
    );
    const copy: ObjectStore = new ObjectStore({
        clock: Clock.system,
        principal: spacePrincipal(roomId),
        objects: [preference],
        policies: [person, room],
        database: copyStorage.database,
        subscriber: Subscriber.of(
            {
                receive: refuseChanges,
                stream: (subscription, signal) =>
                    feed.subscribe(
                        copy.source.replicaOf(subscription).queries,
                        subscription.after,
                        signal,
                    ),
            },
            () => copy.authorizer.chain(copyStorage.database, roomId, { kind: "follower" }),
        ),
        callKey: testCallKey,
        events: await testEvents(copyStorage.database),
        origin: { package: preference.package },
    });
    await followChain(copy, copyStorage.database, homeStorage.database);

    // reach each server as alice, recording the requests each receives
    const writes: string[] = [];
    const recorded = (name: string, server: { fetch(request: Request): Promise<Response> }) =>
        ({
            url: "https://objects.test",
            headers: { authorization: "Bearer alice" },
            fetch: (request: Request) => {
                writes.push(`${name} ${new URL(request.url).pathname}`);

                return server.fetch(request);
            },
        }) satisfies ClientOptions;
    const homeServer = serveObjects(home.implement(preferences, [preference], []), personId);
    const copyServer = serveObjects(copy.implement(preferences, [preference], []), roomId);

    return {
        home,
        homeDatabase: homeStorage.database,
        writes,
        homeEndpoint: recorded("home", homeServer.server),
        copyEndpoint: recorded("copy", copyServer.server),
    };
}

/** Follow a copy's chain from the room up to the universe, waiting for each scope's copy. */
async function followChain(
    copy: ObjectStore,
    database: TestDatabase["database"],
    home: TestDatabase["database"],
): Promise<void> {
    // reconcile each scope's copy, keeping it followed until the test ends
    const follower = present(
        copy.controllers().find((each) => each.name === "replica"),
        "the replica controller",
    );
    const controller = new AbortController();
    const following: Promise<unknown>[] = [];
    onTestFinished(async () => {
        controller.abort();
        await Promise.allSettled(following);
    });
    const head = await home.log.position();
    for (const scope of [roomId, personId, Scope.universe.id]) {
        const key = present(
            (await follower.list()).find((entry) => entry.split(" ")[1] === scope),
            `the replica key of ${scope}`,
        );
        following.push(follower.reconcile(key, reconciliation(controller.signal)));
        await Replica.wait(database, scope, head, controller.signal);
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
        caller: principal.user.reference(Scope.universe.id, "alice"),
        endpoint: served.copyEndpoint,
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
    const [created] = await home.mutate(userContext("alice", personId).caller(), {
        id: RequestId.create(Date.now()),
        calls: [preference.call("create", { scope: personId, name: "theme", value })],
    });

    return present(created, "the created theme").id;
}

/** Wait until a client's database meets a condition, failing after the copy's bound. */
async function waitFor(client: ObjectClient, condition: () => Promise<boolean>): Promise<void> {
    const isMet = await client.database.log.until(
        condition,
        AbortSignal.timeout(COPY_MILLISECONDS),
    );
    if (!isMet) {
        throw new Error("the client's copy never met the condition");
    }
}

/** Wait until the client's copy keeps no queued mutation, its home's commits streamed back. */
async function settled(client: ObjectClient): Promise<void> {
    await waitFor(client, async () => {
        const { pending, executed } = (await client.inspect()).mutations;

        return pending + executed === 0;
    });
}

/** Wait until the client's copy has a preference. */
async function copiedTo(client: ObjectClient, id: string): Promise<void> {
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

/** A preference's value as a client's copy keeps it. */
const Valued = schema.looseObject({ value: schema.string() });

test("send a write of an enclosing scope's row to its home, predicted on the room's copy and rebased once the home's commit streams back through it", async () => {
    // wait for alice's theme in her room's copy
    const served = await serveHomes();
    const id = await setTheme(served.home, "light");
    const { client, failures } = await openRoom(served);
    await copiedTo(client, id);

    // switch it to dark through the room's client
    const submitted = client.mutate(preference).update({ id, value: "dark" });
    await submitted.predicted;
    const predicted = Valued.parse(await client.row(client.database, preference, id)).value;
    await submitted.confirmed;
    await settled(client);
    const copied = Valued.parse(await client.row(client.database, preference, id)).value;

    // predict dark at once, push it to the person's home, and settle once the room's copy has it
    expect({
        predicted,
        copied,
        home: await homeTheme(served.homeDatabase, id),
        writes: served.writes.filter((write) => !write.endsWith("/replica/stream")),
        failures,
    }).toEqual({
        predicted: "dark",
        copied: "dark",
        home: "dark",
        writes: ["home /replica/push", "copy /replica/watermark"],
        failures: [],
    });
});

test("refuse a mutation changing rows of two homes", async () => {
    // change alice's theme at her person's home and create a preference in her room at once
    const served = await serveHomes();
    const id = await setTheme(served.home, "light");
    const { client } = await openRoom(served);
    await copiedTo(client, id);
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
    await copiedTo(client, id);
    await client.mutate(preference).update({ id, value: "dark" }).confirmed;
    await client.undo().confirmed;
    await settled(client);

    // run the undo at the person's home as well
    expect({
        home: await homeTheme(served.homeDatabase, id),
        copied: Valued.parse(await client.row(client.database, preference, id)).value,
        writes: served.writes.filter((write) => !write.endsWith("/replica/stream")),
        failures,
    }).toEqual({
        home: "light",
        copied: "light",
        writes: [
            "home /replica/push",
            "copy /replica/watermark",
            "home /replica/push",
            "copy /replica/watermark",
        ],
        failures: [],
    });
});
