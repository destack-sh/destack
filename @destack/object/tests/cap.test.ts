import { MemoryBuild } from "@destack/package/test";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { TestDatabase } from "@destack/db/test";
import { createClient } from "@destack/service/client";
import { RequestId } from "@destack/service/request";
import { testCallKey } from "@destack/service/test";
import type { InstallationContext } from "@destack/service/workload";
import { Scope } from "@destack/sync";
import { ObjectServer } from "../src/server/index.ts";
import { serveObjects, spaceId } from "./fixture/device.ts";
import { notebook, note, notesDatabase, notesService } from "./fixture/note.ts";
import { openSpace } from "./fixture/space.ts";

/** Name a call's space and a new request. */
function request() {
    return { spaceId, requestId: RequestId.create() };
}

/** The refusal of a method in the capped space, naming the way out of the cap. */
function refused(method: string): readonly [string, string] {
    return [
        "QUOTA_EXCEEDED",
        `storage of scope ${spaceId} is capped: ${method} is refused until data is deleted or the plan changes`,
    ];
}

/** Serve the notes of a space as an installation keeping them, with a client acting as alice. */
async function serveInstalledNotes() {
    // keep the space's notes
    const storage = await TestDatabase.create("sqlite", notesDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, spaceId);

    // serve them as the notes installation, which reaches nothing beyond its own database here
    const installation: InstallationContext = {
        id: "installation-01996ab0-0000-7000-8000-000000000003",
        scope: spaceId,
        build: (await MemoryBuild.write(new Map(), { package: note.package })).reader,
        publisher: {
            stream: () => {
                throw new TypeError("the fixture streams no copies");
            },
            receive: () => Promise.reject(new TypeError("the fixture receives no changes")),
        },
        publisherAt: () => {
            throw new TypeError("the fixture reaches no installation");
        },
        directory: {
            isHome: async () => false,
            locale: async () => undefined,
            address: () => Promise.reject(new TypeError("the fixture resolves no address")),
            messageKey: () => Promise.reject(new TypeError("the fixture seals no messages")),
        },
    };
    const server = new ObjectServer({
        objects: { notebook, note },
        database,
        callKey: testCallKey,
        origin: { package: note.package, service: notesService.name },
        installation,
    });
    const served = serveObjects(server.implement(notesService), spaceId);

    return { database, alice: createClient(notesService, served.endpoint("alice")) };
}

test("refuse an installation's writes in a space whose storage is capped, keeping reads and deletes, until the cap lifts", async () => {
    // keep a notebook with two notes
    const { database, alice } = await serveInstalledNotes();
    const book = await alice.notebook.create({ ...request(), name: "Travel" });
    const packing = await alice.note.create({ ...request(), title: "Packing" });
    const tickets = await alice.note.create({ ...request(), title: "Tickets" });

    // cap the space's storage, as the copy of its scope row arrives from its cell
    await Scope.cap(database, spaceId, Date.now());
    const created = await refusal(alice.note.create({ ...request(), title: "Hotels" }));
    const updated = await refusal(
        alice.notebook.update({ ...request(), id: book.id, name: "Trips" }),
    );

    // list the notes, and move one to the trash, which frees storage
    const listed = await alice.note.list({ spaceId });
    await alice.note.delete({ ...request(), id: packing.id });
    const restored = await refusal(alice.note.restore({ ...request(), id: packing.id }));

    // lift the cap, writing again
    await Scope.cap(database, spaceId, null);
    const hotels = await alice.note.create({ ...request(), title: "Hotels" });

    // name the refused method and the way out of the cap
    expect({
        created,
        updated,
        listed: listed.items.map((item) => item.title).toSorted(),
        restored,
        hotels: hotels.title,
        kept: (await alice.note.list({ spaceId })).items.map((item) => item.id).toSorted(),
    }).toEqual({
        created: refused("note.create"),
        updated: refused("notebook.update"),
        listed: ["Packing", "Tickets"],
        restored: refused("note.restore"),
        hotels: "Hotels",
        kept: [tickets.id, hotels.id].toSorted(),
    });
});
