import { schema } from "@destack/schema";
import { MemoryBlobStorage } from "@destack/resource";
import { Transaction } from "@destack/db";
import { testSimulation } from "../src/test/index.ts";
import { users } from "./fixture/user.ts";
import { ResourceContext } from "@destack/resource/context";
import { MemoryBuild } from "@destack/package/test";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { TestDatabase } from "@destack/db/test";
import { createClient } from "@destack/object/client";
import { RequestId, Scope } from "@destack/sync";
import { testCallKey, testKeys } from "@destack/object/test";
import { testEvents } from "../src/test/index.ts";
import type { InstalledServerContext } from "@destack/object/workload";
import { implementPackage, ObjectStore } from "../src/server/index.ts";
import { spaceId } from "./fixture/device.ts";
import { notebook, note, notesDatabase, notesPackage } from "./fixture/note.ts";
import { openSpace } from "./fixture/space.ts";
import { Environment } from "@destack/environment";
import { TestObjectStore } from "../src/test/index.ts";

/** Build a call's space and a new request. */
function request(world: Environment) {
    return { spaceId, requestId: RequestId.create(world) };
}

/** The refusal of a method in the capped space, stating the way out of the cap. */
function refused(method: string): readonly [string, string, string] {
    return [
        "PAYMENT_REQUIRED",
        "STORAGE_CAPPED",
        `storage of scope ${spaceId} is capped: ${method} is refused until data is deleted or the plan changes`,
    ];
}

/** Serve the notes of a space as an installation storing them, with a client acting as alice. */
async function serveInstalledNotes(world: Environment) {
    // store the space's notes
    const storage = await TestDatabase.create(notesDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, world, spaceId);

    // serve them as the notes installation, which sees nothing beyond its database here
    const server: InstalledServerContext = {
        origin: "installed",
        installation: schema
            .identifier("installation")
            .parse("installation-01996ab0-0000-7000-8000-000000000003"),
        name: "notes",
        scope: spaceId,
        build: await MemoryBuild.write(new Map(), { package: note.package }, world.crypto),
        environment: world,
        resources: new ResourceContext(),
        fetch: (sent) => Promise.reject(new Error(`the notes reach no ${sent.url}`)),
        url: "https://notes--home.test",
        publisher: {
            request: async () => {},
            stream: () => {
                throw new TypeError("the fixture streams no replicas");
            },
            receive: () => Promise.reject(new TypeError("the fixture receives no changes")),
            call: () => Promise.reject(new TypeError("the fixture answers no queries")),
        },
        settings: {},
        capabilities: [],
        keys: await testKeys(world),
        publisherAt: () => {
            throw new TypeError("the fixture reaches no installation");
        },
        at: () => {
            throw new TypeError("the fixture fetches no address");
        },
    };
    const store = new ObjectStore({
        chain: async () => undefined,
        environment: world,
        blobs: new MemoryBlobStorage(world.crypto),
        server,
        servers: [implementPackage(notesPackage, { objects: [notebook, note] })],
        database,
        callKey: testCallKey,
        events: await testEvents(database, world),
        origin: { package: note.package },
    });
    const served = TestObjectStore.serve(store);
    onTestFinished(() => served[Symbol.asyncDispose]());

    return {
        environment: world,
        database,
        alice: createClient(notesPackage, served.as(users.alice).endpoint(notesPackage), world),
    };
}

test("refuse an installation's writes in a space whose storage is capped, allowing reads and deletes, until the cap lifts", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // create a notebook with two notes
    const { database, alice } = await serveInstalledNotes(world);
    const book = await alice.notebook.create({ ...request(world), name: "Travel" });
    const packing = await alice.note.create({ ...request(world), title: "Packing" });
    const tickets = await alice.note.create({ ...request(world), title: "Tickets" });

    // cap the space's storage, as the replica of its scope row arrives from its machine
    await Transaction.run(database, (transaction) =>
        Scope.cap(transaction, spaceId, world.clock.now()),
    );
    const created = await refusal(alice.note.create({ ...request(world), title: "Hotels" }));
    const updated = await refusal(
        alice.notebook.update({ ...request(world), id: book.id, name: "Trips" }),
    );

    // list the notes, and move one to the trash, which frees storage
    const listed = await alice.note.list({ spaceId });
    await alice.note.delete({ ...request(world), id: packing.id });
    const restored = await refusal(alice.note.restore({ ...request(world), id: packing.id }));

    // lift the cap, writing again
    await Transaction.run(database, (transaction) => Scope.cap(transaction, spaceId, null));
    const hotels = await alice.note.create({ ...request(world), title: "Hotels" });

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
