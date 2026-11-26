import type { Clock, Environment } from "@destack/environment";
import { MemoryBlobStorage } from "@destack/resource";
import {
    testSimulation,
    testIdentity,
    testEvents,
    serversOf,
    TestObjectStore,
} from "../src/test/index.ts";
import {
    defineObject,
    device,
    field,
    nested,
    ObjectDescription,
    tablesOf,
    propagated,
} from "../src/index.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import {
    AccessContext,
    Caller,
    CONTAINED,
    principal,
    through,
    accessTables,
} from "@destack/access";
import { TestAccess } from "@destack/access/test";
import { RequestId, Scope, type Subject } from "@destack/sync";
import { TestDatabase } from "@destack/db/test";
import type { DatabaseConnection } from "@destack/db/database";
import { BrowserSimulation } from "@destack/environment/browser/simulation";
import { Identifier, nth } from "@destack/schema";
import { definePackage } from "@destack/object";
import { testCallKey } from "@destack/object/test";
import { BrowserTab, type BrowserTabWorker, DeviceElection } from "../src/client/browser.ts";
import { type BrowserDevice } from "../src/device/browser.ts";
import { deviceDatabase, deviceWorker, holdRemote } from "./fixture/browser.ts";
import { WasmClient, broadcastChannel, connectWasmSqlite } from "@destack/db/browser";
import { type ObjectAccess, ObjectQueryBuilder } from "../src/client/index.ts";
import { ObjectClient } from "../src/device/index.ts";
import { ChannelRemote } from "../src/replica/channel.ts";
import { ObjectStore, implementPackage } from "../src/server/index.ts";
import { serveNotes, spaceId } from "./fixture/device.ts";
import { space, spacePrincipal } from "../src/test/space.ts";
import { draft, memo, memosPackage, panel, sticky, user } from "./fixture/local.ts";
import { note, notebook } from "../src/test/note.ts";
import objectPackage from "../src/package.ts";
import { argument, plural, t } from "@destack/locale";

/** The module an app's device worker starts from, as its tabs resolve it. */
const DEVICE_WORKER = new URL("./fixture/device.worker.ts", import.meta.url);

test("read a local object one tab writes in another tab through their device, out of the outbox", async () => {
    // run in a world of its own
    const simulation = testSimulation();
    const world = simulation.environment("test");

    const { connection } = await serveNotes({ simulation, node: "browser" });
    const alice = Identifier.create("user", world);

    // open two tabs of one browser sharing one device worker
    const environment = new BrowserSimulation(simulation).environment("browser");
    const objects = { notebook, note, draft };
    const caller = principal.user.reference("universe", alice);
    const browser = principal.device.reference("universe", Identifier.create("device", world));
    const served: BrowserDevice[] = [];
    const worker = deviceWorker({
        database: await deviceDatabase(objects, environment),
        objects,
        caller,
        device: browser,
        remote: (scope: string) => connection(alice).at(scope),
        served: (opened) => served.push(opened),
        environment,
    });
    const open = () =>
        BrowserTab.open({
            environment,
            name: "notes",
            objects,
            scope: spaceId,
            caller,
            election: openElection(worker, environment),
        });
    const first = open();
    await first.ready;
    const second = open();
    await second.ready;

    // write a draft in the first tab and read it in the second
    const written = first.mutate(draft).create({ text: "Packing list" });
    const created = await written.predicted;
    await written.confirmed;
    const drafts = second.query.draft.findMany().subscribe();
    await drafts.ready;

    // expect the draft in the second tab, its partition alice's, and no mutation queued
    expect({
        drafts: (await drafts.read()).map((row) => [row.id, row.scope, row.text]),
        mutations: (await nth(served, 0).client.inspect()).mutations,
    }).toEqual({
        drafts: [[created.id, alice, "Packing list"]],
        mutations: { pending: 0, executed: 0, rejected: 0 },
    });
    await drafts.close();
    await second.close();
    await first.close();
});

test("query and write a space's objects through the device, the party keeping no database", async () => {
    // serve a device to a party on the tabs' channel, the party keeping nothing but what it reads
    const simulation = testSimulation();
    const world = simulation.environment("test");
    const { connection } = await serveNotes({ simulation, node: "browser" });
    const alice = Identifier.create("user", world);
    const environment = new BrowserSimulation(simulation).environment("browser");
    const objects = { notebook, note, draft };
    const caller = principal.user.reference("universe", alice);
    const browser = principal.device.reference("universe", Identifier.create("device", world));
    openElection(
        deviceWorker({
            database: await deviceDatabase(objects, environment),
            objects,
            caller,
            device: browser,
            remote: (scope: string) => connection(alice).at(scope),
            environment,
        }),
        environment,
    );
    const party = new ChannelRemote(
        broadcastChannel("destack:notes", environment),
        RequestId.create(environment),
        environment,
    );
    onTestFinished(() => party.close());
    const query = ObjectQueryBuilder.of(party.subscriber(spaceId, Object.values(objects)), objects);

    // follow the space's notebooks, then create one through the device
    const notebooks = query.notebook.findMany().subscribe();
    await notebooks.ready;
    const before = await notebooks.read();
    const written = party.mutate(spaceId, notebook, "create", { name: "Travel" });
    await written.predicted;
    const predicted = await notebooks.read();
    await written.confirmed;

    // expect no notebook first, then the device's prediction, before its home confirms it
    expect({
        before: before.map((row) => row.name),
        predicted: predicted.map((row) => row.name),
    }).toEqual({ before: [], predicted: ["Travel"] });
    await notebooks.close();
});

test("make a space tab ready once its device replicates the scope chain", async () => {
    // open a device whose machine answers only once the tab asked for the space's access rows
    const simulation = testSimulation();
    const world = simulation.environment("test");
    const { connection } = await serveNotes({ simulation, node: "browser" });
    const alice = Identifier.create("user", world);
    const environment = new BrowserSimulation(simulation).environment("browser");
    const objects = { notebook, note, draft };
    const caller = principal.user.reference("universe", alice);
    const browser = principal.device.reference("universe", Identifier.create("device", world));
    const answering = Promise.withResolvers<void>();
    const reached = connection(alice);
    const served: BrowserDevice[] = [];
    const worker = deviceWorker({
        database: await deviceDatabase(objects, environment),
        objects,
        caller,
        device: browser,
        remote: (scope: string) =>
            holdRemote(reached.at(scope), answering.promise, new Set(["stream", "push"])),
        environment,
        served: (opened) => served.push(opened),
    });

    // open the tab on the space, then let the machine answer
    const tab = BrowserTab.open({
        environment,
        name: "notes",
        objects,
        scope: spaceId,
        caller,
        election: openElection(worker, environment),
    });
    answering.resolve();
    await tab.ready;

    // read the space's chain at the device once the tab is ready
    expect(await readChain(nth(served, 0).client.database)).toEqual([spaceId, Scope.universe.id]);
    await tab.close();
});

test("make a tab of the person's own scope ready once its device replicates the chain", async () => {
    // serve the person's own scope, answering the device only once its tab asked
    const { caller, alice, open, answer, devices } = await openOwnScope();
    const { tab } = open();
    answer();
    await tab.ready;

    // read the person's chain at the device once the tab is ready
    const { database } = nth(devices, 0).client;
    expect((await Scope.chain(database.snapshot(), alice)).map((link) => link.object.id)).toEqual([
        caller.id,
        Scope.universe.id,
    ]);
    await tab.close();
});

test("follow a tab's replicated memos in the person's own scope after its local drafts", async () => {
    // write a memo at the person's home, and open a tab reading only local drafts first
    const { alice, open, answer, home, world } = await openOwnScope();
    answer();
    await home.memo.create({
        userId: alice,
        requestId: RequestId.create(world),
        text: "Call the bank",
    });
    const { tab } = open();
    await tab.ready;
    const drafts = tab.query.draft.findMany().subscribe();
    await drafts.ready;

    // read the memo, which the device relays once its replica arrives
    const memos = tab.query.memo.findMany({ where: { scope: alice } }).subscribe();
    await memos.ready;
    expect((await memos.read()).map((row) => row.text)).toEqual(["Call the bank"]);
    await memos.close();
    await drafts.close();
    await tab.close();
});

test("hide a user's local objects from another user on the same device, and the device's from another device", async () => {
    // run in a world of its own
    const simulation = testSimulation();
    const world = simulation.environment("test");

    const { connection, environment } = await serveNotes({ simulation });
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
            isMigrated: true,
            remote: connection(person).at(spaceId),
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
    const draftPackage = definePackage({
        ...testIdentity,
        objects: [draft],
    });
    expect([
        failure(() =>
            nth(
                new ObjectStore({
                    servers: [implementPackage(draftPackage, {})],
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
                    title: t`${plural(argument("count"), { one: "Shared", other: "Shared" })}`,
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
                    title: t`${plural(argument("count"), { one: "Spread", other: "Spreads" })}`,
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
                    title: t`${plural(argument("count"), { one: "Sticky", other: "Stickies" })}`,
                    traits: [propagated({})],
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
                        title: t`${plural(argument("count"), { one: "Remote", other: "Remotes" })}`,
                        fields: { text: field.string() },
                        access: {
                            permissions: ["edit"],
                        },
                    },
                    (method) => ({
                        create: method.create("edit", { isPredicted: false }),
                    }),
                ),
            ),
        ),
    ]).toEqual([
        "local object draft only exists in its scope's store",
        "local object 'shared' must belong to one scope type",
        "local object 'spread' must belong to one scope type",
        "local object 'sticky' cannot have 'propagated'",
        "local object 'remote' cannot run method 'create' outside the client",
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
    const store = TestObjectStore.of({
        chain: async () => undefined,
        environment: world,
        principal: principal.device.reference(Scope.universe.id, laptop.id),
        servers: serversOf([sticky]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: { package: sticky.package },
    });

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
        bobLists: ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope '${laptop.id}' not found`],
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
    const store = TestObjectStore.of({
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
        deskLists: ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope '${laptop.id}' not found`],
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

test("refuse a tab's readiness with the reason its device worker could not open the database", async () => {
    // let the browser's device worker fail to open its database, as a second holder of its storage does
    const simulation = testSimulation();
    const world = simulation.environment("test");
    simulation.workers.define(DEVICE_WORKER, (scope) => {
        scope.addEventListener("message", () =>
            scope.postMessage({ kind: "failed", message: "InvalidStateError: the handle is held" }),
        );
    });

    // open a tab leading the device through the real worker start
    const alice = Identifier.create("user", world);
    const environment = new BrowserSimulation(simulation).environment("browser");
    const tab = BrowserTab.open({
        environment,
        name: "notes",
        objects: { notebook, note, draft },
        scope: spaceId,
        caller: principal.user.reference("universe", alice),
        election: openElection(
            BrowserTab.worker(
                DEVICE_WORKER,
                "notes",
                Promise.resolve({ name: "notes" }),
                environment,
            ),
            environment,
        ),
    });

    expect(await refusal(tab.ready)).toEqual([
        "SERVICE_UNAVAILABLE",
        "DATABASE_UNAVAILABLE",
        "database 'notes' is unavailable: this browser cannot open it: InvalidStateError: the handle is held",
    ]);
});

/** Read the space's chain as a database holds its scope rows, nearest first. */
async function readChain(database: DatabaseConnection): Promise<string[]> {
    return (await Scope.chain(database.snapshot(), spaceId)).map((link) => link.object.id);
}

/** Serve a person's own scope with memos, opening tabs over a device answered once told. */
async function openOwnScope() {
    // serve memos at the person's own scope
    const simulation = testSimulation();
    const world = simulation.environment("test");
    const alice = Identifier.create("user", world);
    const caller = principal.user.reference("universe", alice);
    const served = await TestObjectStore.open({
        packages: [memosPackage],
        scope: caller,
        simulation,
        node: "browser",
    });
    onTestFinished(() => served[Symbol.asyncDispose]());

    // open the person's device on their own scope, its machine answering once told
    const environment = new BrowserSimulation(simulation).environment("browser");
    const objects = { memo, draft };
    const browser = principal.device.reference("universe", Identifier.create("device", world));
    const answering = Promise.withResolvers<void>();
    const reached = served.as(alice).connect(memosPackage, { scope: alice });
    const devices: BrowserDevice[] = [];
    const worker = deviceWorker({
        database: await deviceDatabase(objects, environment),
        objects,
        caller,
        device: browser,
        scope: alice,
        remote: (scope: string) =>
            holdRemote(reached.at(scope), answering.promise, new Set(["stream", "push"])),
        environment,
        served: (opened) => devices.push(opened),
    });
    const open = () => ({
        tab: BrowserTab.open({
            environment,
            name: "notes",
            objects,
            scope: alice,
            caller,
            election: openElection(worker, environment),
        }),
    });

    return {
        caller,
        alice,
        open,
        answer: () => answering.resolve(),
        devices,
        home: served.as(alice).client(memosPackage),
        world,
    };
}

/** Cards of a space, read by whoever reads the space, nested under each other. */
const card = defineObject({
    name: "card",
    plural: "cards",
    title: t`${plural(argument("count"), { one: "Card", other: "Cards" })}`,
    scope: space,
    fields: { title: field.string() },
    access: { permissions: { read: through("scope", "read"), write: through("scope", "read") } },
    traits: [nested({ in: "self", optional: true, add: "write" })],
});

/** The package serving the cards. */
const cardsPackage = definePackage({ ...testIdentity, objects: [card] });

test("follow a nested card from the machine in a tab through its device, and again through the reopened device database", async () => {
    // serve a private space alice reads, with one card written at the machine
    const simulation = testSimulation();
    const world = simulation.environment("test");
    const alice = Identifier.create("user", world);
    const caller = principal.user.reference("universe", alice);
    const objects = await TestObjectStore.open({
        packages: [cardsPackage],
        scope: space.reference(Scope.universe.id, spaceId),
        visibility: "private",
        simulation,
        node: "machine",
    });
    onTestFinished(() => objects[Symbol.asyncDispose]());
    const access = new TestAccess(objects.database, world);
    await access.replicateRelationship(
        space.reference(Scope.universe.id, spaceId),
        "reader",
        caller,
        0,
    );
    const machine = objects.as(alice).client(cardsPackage);
    await machine.card.create({ spaceId, requestId: RequestId.create(world), title: "Milk" });

    // open a tab through the device, then close it, as a page load and its unload do
    const environment = new BrowserSimulation(simulation).environment("browser");
    const types = { card };
    const browser = principal.device.reference("universe", Identifier.create("device", world));
    const memory = await WasmClient.memory(environment);
    const open = async () => {
        const worker = deviceWorker({
            database: connectWasmSqlite(memory, ObjectClient.tables(types), {
                name: "notes",
                environment,
            }),
            objects: types,
            caller,
            device: browser,
            remote: (scope: string) => objects.as(alice).connect(cardsPackage, { scope }),
            environment,
        });
        const tab = BrowserTab.open({
            environment,
            name: "notes",
            objects: types,
            scope: spaceId,
            caller,
            election: openElection(worker, environment),
        });
        await tab.ready;
        const cards = tab.query.card.findMany().subscribe();
        await cards.ready;
        const titles = (await cards.read()).map((row) => row.title);
        await cards.close();
        await tab.close();

        return titles;
    };
    const first = await open();

    // open it again over the same device database, as the next page load does
    const second = await open();
    expect({ first, second }).toEqual({ first: ["Milk"], second: ["Milk"] });
});

/** Stand for the notes device's election with a worker for the rest of the test. */
function openElection(worker: BrowserTabWorker, environment: Environment<"locks">): DeviceElection {
    const election = DeviceElection.start("notes", worker, environment);
    onTestFinished(() => election.close());

    return election;
}
