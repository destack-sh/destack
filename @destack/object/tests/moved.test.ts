import { testSimulation } from "../src/test/index.ts";
import { users } from "./fixture/user.ts";
import { Transaction } from "@destack/db";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { Scope, RequestId } from "@destack/sync";
import { AccessContext, Caller, principal } from "@destack/access";
import type { DatabaseConnection } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { Identifier, schema } from "@destack/schema";
import { testCallKey } from "@destack/object/test";
import { ObjectClient } from "../src/client/index.ts";
import { defineObject, field } from "../src/index.ts";
import { testEvents, serversOf } from "../src/test/index.ts";
import { serveNotes, spaceId } from "./fixture/device.ts";
import { note, notebook } from "./fixture/note.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { Simulation } from "@destack/environment/simulation";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { TestObjectStore } from "../src/test/index.ts";

/** A labelled marker of a space, whose label stays editable while the space moves. */
const marker = defineObject(
    {
        name: "marker",
        plural: "markers",
        scope: space,
        fields: { label: field.string() },
        shareable: {},
        moving: ["write"],
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
    const simulation = new Simulation({ seed: 1, start: Date.UTC(2026, 9, 1) });
    const source = await serveNotes({ simulation, node: "source" });
    const target = await serveNotes({ simulation, node: "target" });
    const { clock, environment } = source;
    const moved = Identifier.create("machine", world);
    await Transaction.run(source.database, (transaction) =>
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
        endpoint: source.endpoint(users.alice),
        reconnect: (machine) => {
            machines.push(machine);

            return target.endpoint(users.alice);
        },
    });
    const stopping = new AbortController();
    const errors: unknown[] = [];
    const running = client.run(stopping.signal, (error) => errors.push(error));
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });

    // confirm the note only on the new machine, without a reported failure
    await client.mutate(note).create({ title: "Ideas" }).confirmed;
    expect([
        machines,
        errors,
        await titles(source.database),
        await titles(target.database),
    ]).toEqual([[moved], [], [], ["Ideas"]]);
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
    await Transaction.run(database, (transaction) =>
        Scope.fence(transaction, spaceId, moved, world.clock.now()),
    );

    // relabel the marker through its moving permission and refuse a new marker with the new machine
    await mutate(marker.call("update", { spaceId, id, label: "final" }));
    expect([
        await refusal(mutate(marker.call("create", { spaceId, label: "late" }))),
        (await database.select({ label: marker.table.label }).from(marker.table)).map(
            (row) => row.label,
        ),
    ]).toEqual([["MISDIRECTED_REQUEST", "SCOPE_MOVED", `${spaceId} moved to ${moved}`], ["final"]]);
});

/** Read the titles of the notes in a database. */
async function titles(database: DatabaseConnection): Promise<string[]> {
    const rows = await database.select({ title: note.table.title }).from(note.table);

    return rows.map((row) => row.title);
}
