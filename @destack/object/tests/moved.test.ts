import { worldTelemetry } from "../src/test/world.ts";
import { testSimulation, testEvents, serversOf, TestObjectStore } from "../src/test/index.ts";
import { users } from "./fixture/user.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { Scope, RequestId } from "@destack/sync";
import { AccessContext, Caller, principal } from "@destack/access";
import type { DatabaseConnection } from "@destack/db/database";
import { TestDatabase } from "@destack/db/test";
import { Identifier, schema } from "@destack/schema";
import { testCallKey } from "@destack/object/test";
import { ObjectConnection } from "../src/client/index.ts";
import { ObjectClient } from "../src/device/index.ts";
import { defineObject, field, tablesOf, shareable } from "../src/index.ts";
import { serveNotes, spaceId } from "./fixture/device.ts";
import { note, notebook } from "../src/test/note.ts";
import { openSpace, space, spacePrincipal } from "../src/test/space.ts";
import { Simulation } from "@destack/environment/simulation";
import objectPackage from "../src/package.ts";
import { argument, plural, t } from "@destack/locale";
import { testWorldOptions } from "@destack/test/world";

/** The connection URL of the machine the space moves from. */
const SOURCE_URL = "wss://source.test/store/connect";

/** The connection URL of the machine the space moves to. */
const TARGET_URL = "wss://target.test/store/connect";

/** A labelled marker of a space, whose label stays editable while the space moves. */
const marker = defineObject(
    {
        name: "marker",
        plural: "markers",
        title: t`${plural(argument("count"), { one: "Marker", other: "Markers" })}`,
        scope: space,
        fields: { label: field.string() },
        access: { moving: ["write"] },
        traits: [shareable({})],
    },
    (method) => ({
        create: method.create("manage"),
        update: method.update("write", { fields: ["label"] }),
    }),
);

test("send a client's writes on to the machine serving a moved scope", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // serve the space on its source and on the machine it moves to, and fence the source
    await using simulation = new Simulation({
        world: testWorldOptions(),
        seed: 1,
        start: Date.UTC(2026, 9, 1),
    });
    const source = await serveNotes({ simulation, node: "source" });
    const target = await serveNotes({ simulation, node: "target" });
    const { clock, environment } = source;
    const moved = Identifier.create("machine", world);
    await source.database.commit((transaction) =>
        Scope.fence(transaction, spaceId, moved, clock.now()),
    );

    // push through the source, reconnecting to the machine its refusal refers to
    const storage = await TestDatabase.create(ObjectClient.tables({ notebook, note }), {
        environment,
        dialect: "sqlite",
    });
    onTestFinished(() => storage.close());
    const machines: string[] = [];
    const client = await ObjectClient.open({
        environment,
        database: storage.database,
        objects: { notebook, note },
        scope: spaceId,
        caller: principal.user.reference("universe", users.alice),
        remote: ObjectConnection.open({
            scope: spaceId,
            url: SOURCE_URL,
            authorize: source.authorize(users.alice),
            machine: (machine) => {
                machines.push(machine);

                return TARGET_URL;
            },
            websocket: (url, protocols) =>
                (url === TARGET_URL ? target : source).websocket(users.alice)(url, protocols),
            environment,
        }),
    });
    const stopping = new AbortController();
    const running = client.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });

    // confirm the note only on the new machine, without a captured failure
    await client.mutate(note).create({ title: "Ideas" }).confirmed;
    expect([
        machines,
        worldTelemetry(environment).exceptions(),
        await titles(source.database),
        await titles(target.database),
    ]).toEqual([[moved], [], [], ["Ideas"]]);
});

test("follow a moved scope to its machine on a socket of its own, leaving the origin's to other scopes", async () => {
    // serve the space on its source and on the machine it moves to, and fence the source
    await using simulation = new Simulation({
        world: testWorldOptions(),
        seed: 1,
        start: Date.UTC(2026, 9, 1),
    });
    const source = await serveNotes({ simulation, node: "source" });
    const target = await serveNotes({ simulation, node: "target" });
    const { clock, environment } = source;
    const world = testSimulation().environment("test");
    const moved = Identifier.create("machine", world);
    await source.database.commit((transaction) =>
        Scope.fence(transaction, spaceId, moved, clock.now()),
    );

    // connect the space and another scope of the origin over one socket, recording each opening
    const opened: string[] = [];
    const connection = ObjectConnection.open({
        scope: spaceId,
        url: SOURCE_URL,
        authorize: source.authorize(users.alice),
        machine: () => TARGET_URL,
        websocket: (url, protocols) => {
            opened.push(url);

            return (url === TARGET_URL ? target : source).websocket(users.alice)(url, protocols);
        },
        environment,
    });
    onTestFinished(() => connection.close());
    const other = connection.at(Identifier.create("space", world));

    // describe through the other scope, then push the moved space's note, then describe again
    await other.describe();
    const storage = await TestDatabase.create(ObjectClient.tables({ notebook, note }), {
        environment,
        dialect: "sqlite",
    });
    onTestFinished(() => storage.close());
    const client = await ObjectClient.open({
        environment,
        database: storage.database,
        objects: { notebook, note },
        scope: spaceId,
        caller: principal.user.reference("universe", users.alice),
        remote: connection,
    });
    const stopping = new AbortController();
    const running = client.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });
    await client.mutate(note).create({ title: "Ideas" }).confirmed;
    await other.describe();

    // expect one socket at each machine: the other scope stays on the origin's
    expect([opened, await titles(target.database)]).toEqual([[SOURCE_URL, TARGET_URL], ["Ideas"]]);
});

test("allow the calls of moving permissions in a fenced scope and refuse every other call", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // store a marker in a space, then fence the space toward another machine
    const storage = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { marker }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const { database } = storage;
    await openSpace(database, world, spaceId);
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([marker]),
        database,
        callKey: testCallKey,
        events: await testEvents(database, world),
        origin: { package: marker.package },
    });
    const mutate = (call: ReturnType<typeof marker.call>) =>
        store.mutate(AccessContext.of(Caller.of(spacePrincipal(spaceId)), world.clock.now()), {
            id: RequestId.create(world),
            calls: [call],
        });
    const [created] = await mutate(marker.call("create", { spaceId, label: "draft" }));
    const { id } = schema.looseObject({ id: schema.string() }).parse(created);
    const moved = Identifier.create("machine", world);
    await database.commit((transaction) =>
        Scope.fence(transaction, spaceId, moved, world.clock.now()),
    );

    // relabel the marker through its moving permission and refuse a new marker with the new machine
    await mutate(marker.call("update", { spaceId, id, label: "final" }));
    expect([
        await refusal(mutate(marker.call("create", { spaceId, label: "late" }))),
        (await database.select({ label: marker.table.label }).from(marker.table)).map(
            (row) => row.label,
        ),
    ]).toEqual([
        ["MISDIRECTED_REQUEST", "SCOPE_MOVED", `space '${spaceId}' moved to '${moved}'`],
        ["final"],
    ]);
});

/** Read the titles of the notes in a database. */
async function titles(database: DatabaseConnection): Promise<string[]> {
    const rows = await database.select({ title: note.table.title }).from(note.table);

    return rows.map((row) => row.title);
}
