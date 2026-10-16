import { users } from "./fixture/user.ts";
import { Transaction } from "@destack/db";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { Scope } from "@destack/sync";
import { AccessContext, Caller, principal } from "@destack/access";
import type { DatabaseConnection } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { Identifier, Clock, schema } from "@destack/schema";
import { RequestId } from "@destack/runtime/request";
import { testCallKey } from "@destack/runtime/test";
import { ObjectClient } from "../src/client/index.ts";
import { defineObject, field } from "../src/index.ts";
import { ObjectStore } from "../src/server/index.ts";
import { callTables, testEvents, serversOf } from "../src/test/index.ts";
import { serveNotes, spaceId } from "./fixture/device.ts";
import { note, notebook } from "./fixture/note.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";

/** A labelled pin of a space, whose label stays editable while the space moves. */
const pin = defineObject(
    {
        name: "pin",
        plural: "pins",
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
    // serve the space on its source and on the machine it moves to, and fence the source
    const source = await serveNotes("sqlite");
    const target = await serveNotes("sqlite");
    const moved = Identifier.create("machine");
    await Transaction.run(source.database, (transaction) =>
        Scope.fence(transaction, spaceId, moved, Date.now()),
    );

    // push through the source, reconnecting to the machine its refusal refers to
    const storage = await TestDatabase.create("sqlite", ObjectClient.tables({ notebook, note }));
    onTestFinished(() => storage.close());
    const machines: string[] = [];
    const client = await ObjectClient.open({
        clock: Clock.system,
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

test("admit the calls of moving permissions in a fenced scope and refuse every other call", async () => {
    // store a pin in a space, then fence the space toward another machine
    const storage = await TestDatabase.create("sqlite", [...pin.tables, ...callTables], {
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const { database } = storage;
    await openSpace(database, spaceId);
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([pin]),
        database,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: { package: pin.package },
    });
    const mutate = (call: ReturnType<typeof pin.call>) =>
        store.mutate(AccessContext.of(Caller.of(spacePrincipal(spaceId)), Date.now()), {
            id: RequestId.create(Date.now()),
            calls: [call],
        });
    const [created] = await mutate(pin.call("create", { spaceId, label: "draft" }));
    const { id } = schema.looseObject({ id: schema.string() }).parse(created);
    const moved = Identifier.create("machine");
    await Transaction.run(database, (transaction) =>
        Scope.fence(transaction, spaceId, moved, Date.now()),
    );

    // relabel the pin through its moving permission and refuse a new pin with the new machine
    await mutate(pin.call("update", { spaceId, id, label: "final" }));
    expect([
        await refusal(mutate(pin.call("create", { spaceId, label: "late" }))),
        (await database.select({ label: pin.table.label }).from(pin.table)).map((row) => row.label),
    ]).toEqual([["MISDIRECTED_REQUEST", `${spaceId} moves to ${moved}`], ["final"]]);
});

/** Read the titles of the notes in a database. */
async function titles(database: DatabaseConnection): Promise<string[]> {
    const rows = await database.select({ title: note.table.title }).from(note.table);

    return rows.map((row) => row.title);
}
