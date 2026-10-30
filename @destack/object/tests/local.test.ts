import type { Clock } from "@destack/environment";
import { MemoryBlobStorage } from "@destack/resource";
import { testSimulation } from "../src/test/index.ts";
import { defineObject, device, field, Installation, ObjectDescription } from "../src/index.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { AccessContext, accessTables, Caller, CONTAINED, principal } from "@destack/access";
import { TestAccess } from "@destack/access/test";
import { RequestId, Scope, type Subject } from "@destack/sync";
import { TestDatabase } from "@destack/db/test";
import { broadcastChannel, serveDatabase, WasmClient } from "@destack/db/browser";
import {
    BrowserSimulation,
    type SimulatedBrowserEnvironment,
} from "@destack/environment/browser/simulation";
import { Identifier, nth } from "@destack/schema";
import { definePackage } from "@destack/object";
import { testCallKey } from "@destack/object/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { BrowserTab, type BrowserTabWorker } from "../src/browser/index.ts";
import { ObjectClient, type ObjectAccess } from "../src/client/index.ts";
import { ObjectStore, implementPackage } from "../src/server/index.ts";
import { serveNotes, spaceId } from "./fixture/device.ts";
import { spacePrincipal, unmoved, space } from "./fixture/space.ts";
import { draft, panel, sticky, user } from "./fixture/local.ts";
import { note, notebook } from "./fixture/note.ts";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { TestObjectStore } from "../src/test/index.ts";

test("read a local object one tab writes in another tab over the shared database, out of the outbox", async () => {
    // run in a world of its own
    const simulation = testSimulation();
    const world = simulation.environment("test");

    const { endpoint } = await serveNotes({ simulation, node: "browser" });
    const alice = Identifier.create("user", world);

    // open two tabs of one browser sharing one database worker
    const environment = new BrowserSimulation(simulation).environment("browser");
    const worker = databaseWorker(await WasmClient.memory(environment), environment);
    const open = () =>
        BrowserTab.open({
            environment,
            name: "notes",
            objects: { notebook, note, draft },
            scope: spaceId,
            caller: principal.user.reference("universe", alice),
            device: principal.device.reference("universe", Identifier.create("device", world)),
            endpoint: endpoint(alice),
            reconnect: unmoved,
            worker,
            report: (error) => {
                throw error;
            },
        });
    const first = await open();
    await first.ready;
    const second = await open();
    await second.ready;

    // write a draft in the first tab and read it in the second
    const written = first.client.mutate(draft).create({ text: "Packing list" });
    const created = await written.predicted;
    await written.confirmed;
    const drafts = second.client.query.draft.findMany().subscribe();
    await drafts.ready;

    // expect the draft in the second tab, its partition alice's, and no mutation queued
    expect({
        drafts: (await drafts.read()).map((row) => [row.id, row.scope, row.text]),
        mutations: (await first.client.inspect()).mutations,
    }).toEqual({
        drafts: [[created.id, alice, "Packing list"]],
        mutations: { pending: 0, executed: 0, rejected: 0 },
    });
    await drafts.close();
    await second.close();
    await first.close();
});

test("hide a user's local objects from another user on the same device, and the device's from another device", async () => {
    // run in a world of its own
    const simulation = testSimulation();
    const world = simulation.environment("test");

    const { endpoint, environment } = await serveNotes({ simulation });
    const [alice, bob] = [Identifier.create("user", world), Identifier.create("user", world)];
    const [laptop, phone] = [
        Identifier.create("device", world),
        Identifier.create("device", world),
    ];

    // open the users' object clients over one local database, as the laptop and as the phone
    const objects = { notebook, note, draft, panel };
    const storage = await TestDatabase.create([], { environment, dialect: "sqlite" });
    await ObjectClient.migrate(storage.database, objects);
    const openClient = (person: Identifier<"user">, on: string) =>
        ObjectClient.open({
            environment,
            database: storage.database,
            objects,
            scope: spaceId,
            caller: principal.user.reference("universe", person),
            device: principal.device.reference("universe", on),
            endpoint: endpoint(person),
            reconnect: unmoved,
            isMigrated: true,
        });
    const aliceLaptop = await openClient(alice, laptop);
    const bobLaptop = await openClient(bob, laptop);
    const alicePhone = await openClient(alice, phone);

    // write alice's draft and the laptop's panel width as alice on the laptop
    await aliceLaptop.mutate(draft).create({ text: "Alice's draft" }).confirmed;
    await aliceLaptop.mutate(panel).create({ width: 320 }).confirmed;

    // read both types as each user on each device
    const local = { draft, panel };
    const read = {
        aliceLaptop: await readLocal(aliceLaptop.of(local)),
        bobLaptop: await readLocal(bobLaptop.of(local)),
        alicePhone: await readLocal(alicePhone.of(local)),
    };
    expect(read).toEqual({
        aliceLaptop: { drafts: ["Alice's draft"], panels: [320] },
        bobLaptop: { drafts: [], panels: [320] },
        alicePhone: { drafts: ["Alice's draft"], panels: [] },
    });
    await storage.close();
});

test("refuse to serve local objects at another principal's store, and refuse local definitions beyond the device", async () => {
    // run in a world of its own
    const simulation = testSimulation();
    const world = simulation.environment("test");

    const { database, environment } = await serveNotes({ simulation });
    const local = { scope: device, storage: "local", fields: {}, permissions: [] } as const;
    const events = await testEvents(database, world);

    // refuse a server for a local type, and local types in the universe, in several scopes or with durable traits
    const draftPackage = definePackage({ objects: { draft } });
    const drafts = await Installation.base(
        space.reference(Scope.universe.id, spaceId),
        draftPackage,
        world.crypto,
    );
    expect([
        failure(() =>
            nth(
                new ObjectStore({
                    servers: [implementPackage(draftPackage, {}).at(drafts)],
                    chain: async () => undefined,
                    environment: environment,
                    blobs: new MemoryBlobStorage(environment.crypto),
                    origin: { package: draft.package },
                    principal: spacePrincipal(spaceId),
                    callKey: testCallKey,
                    events,
                    database,
                }).packages,
                0,
            ),
        ),
        failure(() =>
            ObjectDescription.of(
                defineObject({
                    ...local,
                    name: "shared",
                    plural: "shared",
                    scope: "universe",
                }),
            ),
        ),
        failure(() =>
            ObjectDescription.of(
                defineObject({
                    ...local,
                    name: "spread",
                    plural: "spreads",
                    scope: [user, notebook],
                }),
            ),
        ),
        failure(() =>
            ObjectDescription.of(
                defineObject({
                    ...local,
                    name: "sticky",
                    plural: "stickies",
                    inherited: {},
                }),
            ),
        ),
        failure(() =>
            ObjectDescription.of(
                defineObject(
                    {
                        ...local,
                        name: "remote",
                        plural: "remotes",
                        fields: { text: field.string() },
                        permissions: ["edit"],
                    },
                    (method) => ({
                        create: method.create("edit", { isPredicted: false }),
                    }),
                ),
            ),
        ),
    ]).toEqual([
        "local object draft only exists in its scope's store",
        "local object shared must belong to one scope type",
        "local object spread must belong to one scope type",
        "local object sticky cannot have inherited",
        "local object remote cannot run method create outside the client",
    ]);
});

/** Call as a signed-in user who lives inside the given subjects. */
function newCaller(id: string, inside: readonly Subject[], clock: Clock): AccessContext {
    const person = Caller.of(principal.user.reference(Scope.universe.id, id));

    return AccessContext.of({ ...person, groups: [...person.groups, ...inside] }, clock.now());
}

test("serve a device's local objects at its store to the people signed in on it alone", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // store a laptop's stickies
    const [ada, bob] = [Identifier.create("user", world), Identifier.create("user", world)];
    const laptop = device.reference(Scope.universe.id, Identifier.create("device", world));
    const storage = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage],
            follows: [],
            tables: [...accessTables, sticky.table],
        }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await new TestAccess(storage.database, world).replicateScope(laptop, device.policy);

    // serve them at the laptop's store
    const store = await TestObjectStore.at(
        principal.device.reference(Scope.universe.id, laptop.id),
        {
            chain: async () => undefined,
            environment: world,
            principal: principal.device.reference(Scope.universe.id, laptop.id),
            servers: serversOf([sticky]),
            database: storage.database,
            callKey: testCallKey,
            events: await testEvents(storage.database, world),
            origin: { package: sticky.package },
        },
    );

    // call as ada signed in on the laptop, who lives inside it, and as bob signed in elsewhere
    const callers = {
        ada: newCaller(ada, [{ ...laptop, relation: CONTAINED }], world.clock),
        bob: newCaller(bob, [], world.clock),
    };
    const write = (caller: AccessContext, text: string) =>
        refusal(
            store.mutate(caller, {
                id: RequestId.create(world),
                calls: [sticky.call("create", { deviceId: laptop.id, text })],
            }),
        );
    const list = async (caller: AccessContext) => {
        const listed = store.query(caller, sticky.call("list", { deviceId: laptop.id }));

        return listed.then((page) => page.items.map((row) => row.text), readRefusal);
    };

    // write and list a sticky as each, bob seeing no laptop at all
    expect({
        adaWrites: await write(callers.ada, "Water the plants"),
        bobWrites: await write(callers.bob, "Call home"),
        adaLists: await list(callers.ada),
        bobLists: await list(callers.bob),
    }).toEqual({
        adaWrites: "done",
        bobWrites: ["NOT_FOUND", "NOT_FOUND", "sticky not found"],
        adaLists: ["Water the plants"],
        bobLists: ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope ${laptop.id} not found`],
    });
});

test("let a device see and use the local objects of its scope as itself, and hide them from another device", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // store a laptop's stickies
    const laptop = device.reference(Scope.universe.id, Identifier.create("device", world));
    const desk = principal.device.reference(Scope.universe.id, Identifier.create("device", world));
    const storage = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage],
            follows: [],
            tables: [...accessTables, sticky.table],
        }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await new TestAccess(storage.database, world).replicateScope(laptop, device.policy);

    // serve them at the laptop's store, as the laptop
    const itself = principal.device.reference(Scope.universe.id, laptop.id);
    const store = await TestObjectStore.at(itself, {
        chain: async () => undefined,
        environment: world,
        principal: itself,
        servers: serversOf([sticky]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: { package: sticky.package },
    });

    // write and list a sticky as the laptop itself and as the desk
    const write = (caller: AccessContext, text: string) =>
        refusal(
            store.mutate(caller, {
                id: RequestId.create(world),
                calls: [sticky.call("create", { deviceId: laptop.id, text })],
            }),
        );
    const list = async (caller: AccessContext) => {
        const listed = store.query(caller, sticky.call("list", { deviceId: laptop.id }));

        return listed.then((page) => page.items.map((row) => row.text), readRefusal);
    };
    const callers = {
        laptop: AccessContext.of(Caller.of(itself), world.clock.now()),
        desk: AccessContext.of(Caller.of(desk), world.clock.now()),
    };

    // let the laptop write and list its own sticky, the desk seeing no laptop at all
    expect({
        laptopWrites: await write(callers.laptop, "Update the drivers"),
        deskWrites: await write(callers.desk, "Back up"),
        laptopLists: await list(callers.laptop),
        deskLists: await list(callers.desk),
    }).toEqual({
        laptopWrites: "done",
        deskWrites: ["NOT_FOUND", "NOT_FOUND", "sticky not found"],
        laptopLists: ["Update the drivers"],
        deskLists: ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope ${laptop.id} not found`],
    });
});

/** Read the drafts and panel widths a device shows its caller. */
async function readLocal(access: ObjectAccess<{ draft: typeof draft; panel: typeof panel }>) {
    // follow both types until read
    const drafts = access.query.draft.findMany().subscribe();
    const panels = access.query.panel.findMany().subscribe();
    await Promise.all([drafts.ready, panels.ready]);
    const rows = {
        drafts: (await drafts.read()).map((row) => row.text),
        panels: (await panels.read()).map((row) => row.width),
    };
    await Promise.all([drafts.close(), panels.close()]);

    return rows;
}

/** Read a call's refusal as its code, reason and message. */
function readRefusal(error: unknown): Promise<"done" | readonly [string, string, string]> {
    return refusal(Promise.reject(error));
}

/** Read the message a definition or construction fails with. */
function failure(build: () => unknown): string {
    try {
        build();
    } catch (error) {
        return error instanceof Error ? error.message : String(error);
    }

    throw new Error("the build did not fail");
}

/** Serve one database to a browser's tabs on their broadcast channel, as its worker does. */
function databaseWorker(
    database: WasmClient,
    environment: SimulatedBrowserEnvironment,
): BrowserTabWorker {
    return async () => {
        const stop = serveDatabase(
            database,
            broadcastChannel("destack:notes", environment),
            environment,
        );

        return async () => stop();
    };
}
