import { principal } from "@destack/access";
import { ObjectTypeReference } from "@destack/sync";
import { Clock } from "@destack/schema";
import { Transaction } from "@destack/db";
import { MemoryBuild } from "@destack/package/test";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { TestDatabase } from "@destack/db/test";
import { createClient } from "@destack/runtime/client";
import { RequestId } from "@destack/runtime/request";
import { testCallKey } from "@destack/runtime/test";
import { testEvents } from "../src/test/index.ts";
import type { InstallationContext } from "@destack/runtime/workload";
import { Scope } from "@destack/sync";
import { ObjectStore } from "../src/server/index.ts";
import { serveObjects, spaceId } from "./fixture/device.ts";
import { notebook, note, notesDatabase, notesPackage } from "./fixture/note.ts";
import { openSpace, spacePrincipal } from "./fixture/space.ts";

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
        build: await MemoryBuild.write(new Map(), { package: note.package }),
        publisher: {
            stream: () => {
                throw new TypeError("the fixture streams no copies");
            },
            receive: () => Promise.reject(new TypeError("the fixture receives no changes")),
        },
        settings: {},
        publisherAt: () => {
            throw new TypeError("the fixture reaches no installation");
        },
        at: () => {
            throw new TypeError("the fixture fetches no address");
        },
        directory: {
            isHome: async () => false,
            locale: () => Promise.reject(new TypeError("the fixture knows no person")),
            address: () => Promise.reject(new TypeError("the fixture resolves no address")),
            messageKey: () => Promise.reject(new TypeError("the fixture seals no messages")),
        },
    };
    const store = new ObjectStore({
        clock: Clock.system,
        principal: principal.installation.reference(installation.scope, installation.id),
        objects: [notebook, note],
        database,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: { package: note.package },
        installed: new Map(
            [notebook, note].map((type) => [
                ObjectTypeReference.key(type.typeReference),
                installation,
            ]),
        ),
    });
    const served = serveObjects(
        store.implement(notesPackage, store.routed, store.controllers()),
        spaceId,
    );

    return { database, alice: createClient(notesPackage, served.endpoint("alice")) };
}

test("refuse an installation's writes in a space whose storage is capped, keeping reads and deletes, until the cap lifts", async () => {
    // keep a notebook with two notes
    const { database, alice } = await serveInstalledNotes();
    const book = await alice.notebook.create({ ...request(), name: "Travel" });
    const packing = await alice.note.create({ ...request(), title: "Packing" });
    const tickets = await alice.note.create({ ...request(), title: "Tickets" });

    // cap the space's storage, as the copy of its scope row arrives from its machine
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
