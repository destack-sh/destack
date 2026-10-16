import { users } from "./fixture/user.ts";
import { ResourceContext } from "@destack/resource/context";
import { Transaction } from "@destack/db";
import { MemoryBuild } from "@destack/package/test";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { TestDatabase } from "@destack/db/test";
import { createClient } from "@destack/runtime/client";
import { RequestId } from "@destack/runtime/request";
import { testCallKey, testKeys } from "@destack/runtime/test";
import { testEvents } from "../src/test/index.ts";
import type { ServerContext } from "@destack/runtime/workload";
import { Scope } from "@destack/sync";
import { defineServer, ObjectStore } from "../src/server/index.ts";
import { serveObjects, spaceId } from "./fixture/device.ts";
import { notebook, note, notesDatabase, notesPackage } from "./fixture/note.ts";
import { openSpace } from "./fixture/space.ts";
import { Clock } from "@destack/environment";

/** Build a call's space and a new request. */
function request() {
    return { spaceId, requestId: RequestId.create(Date.now()) };
}

/** The refusal of a method in the capped space, stating the way out of the cap. */
function refused(method: string): readonly [string, string] {
    return [
        "QUOTA_EXCEEDED",
        `storage of scope ${spaceId} is capped: ${method} is refused until data is deleted or the plan changes`,
    ];
}

/** Serve the notes of a space as an installation storing them, with a client acting as alice. */
async function serveInstalledNotes() {
    // store the space's notes
    const storage = await TestDatabase.create("sqlite", notesDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, spaceId);

    // serve them as the notes installation, which reaches nothing beyond its own database here
    const server: ServerContext = {
        installation: "installation-01996ab0-0000-7000-8000-000000000003",
        name: "notes",
        space: spaceId,
        build: await MemoryBuild.write(new Map(), { package: note.package }),
        resources: new ResourceContext(),
        fetch: (sent) => Promise.reject(new Error(`the notes reach no ${sent.url}`)),
        url: "https://notes--home.test",
        publisher: {
            stream: () => {
                throw new TypeError("the fixture streams no replicas");
            },
            receive: () => Promise.reject(new TypeError("the fixture receives no changes")),
        },
        settings: {},
        keys: await testKeys(),
        publisherAt: () => {
            throw new TypeError("the fixture reaches no installation");
        },
        at: () => {
            throw new TypeError("the fixture fetches no address");
        },
    };
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        server,
        servers: [defineServer(notesPackage, { objects: [notebook, note] })],
        database,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: { package: note.package },
    });
    const served = serveObjects(store.package(notesPackage), spaceId);

    return { database, alice: createClient(notesPackage, served.endpoint(users.alice)) };
}

test("refuse an installation's writes in a space whose storage is capped, allowing reads and deletes, until the cap lifts", async () => {
    // create a notebook with two notes
    const { database, alice } = await serveInstalledNotes();
    const book = await alice.notebook.create({ ...request(), name: "Travel" });
    const packing = await alice.note.create({ ...request(), title: "Packing" });
    const tickets = await alice.note.create({ ...request(), title: "Tickets" });

    // cap the space's storage, as the replica of its scope row arrives from its machine
    await Transaction.run(database, (transaction) => Scope.cap(transaction, spaceId, Date.now()));
    const created = await refusal(alice.note.create({ ...request(), title: "Hotels" }));
    const updated = await refusal(
        alice.notebook.update({ ...request(), id: book.id, name: "Trips" }),
    );

    // list the notes, and move one to the trash, which frees storage
    const listed = await alice.note.list({ spaceId });
    await alice.note.delete({ ...request(), id: packing.id });
    const restored = await refusal(alice.note.restore({ ...request(), id: packing.id }));

    // lift the cap, writing again
    await Transaction.run(database, (transaction) => Scope.cap(transaction, spaceId, null));
    const hotels = await alice.note.create({ ...request(), title: "Hotels" });

    // state the refused method and the way out of the cap
    expect({
        created,
        updated,
        listed: listed.items.map((item) => item.title).toSorted(),
        restored,
        hotels: hotels.title,
        remaining: (await alice.note.list({ spaceId })).items.map((item) => item.id).toSorted(),
    }).toEqual({
        created: refused("note.create"),
        updated: refused("notebook.update"),
        listed: ["Packing", "Tickets"],
        restored: refused("note.restore"),
        hotels: "Hotels",
        remaining: [tickets.id, hotels.id].toSorted(),
    });
});
