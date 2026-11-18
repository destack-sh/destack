import type { Clock } from "@destack/environment";
import { MemoryBlobStorage } from "@destack/resource";
import { testSimulation, testIdentity } from "../src/test/index.ts";
import { defineObject, device, field, ObjectDescription } from "../src/index.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { AccessContext, accessTables, Caller, CONTAINED, principal } from "@destack/access";
import { TestAccess } from "@destack/access/test";
import { RequestId, Scope, type Subject } from "@destack/sync";
import { TestDatabase } from "@destack/db/test";
import { type DatabaseConnection, Snapshot } from "@destack/db";
import { BrowserSimulation } from "@destack/environment/browser/simulation";
import { Identifier, nth, present } from "@destack/schema";
import { definePackage } from "@destack/object";
import { testCallKey } from "@destack/object/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { type BrowserDevice, BrowserTab } from "../src/browser/index.ts";
import { deviceDatabase, deviceWorker } from "./fixture/browser.ts";
import { ObjectClient, ProcedureRemote, type ObjectAccess } from "../src/client/index.ts";
import { ObjectStore, implementPackage } from "../src/server/index.ts";
import { serveNotes, spaceId } from "./fixture/device.ts";
import { spacePrincipal, unmoved } from "../src/test/space.ts";
import { draft, memo, memosPackage, panel, sticky, user } from "./fixture/local.ts";
import { note, notebook } from "../src/test/note.ts";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { TestObjectStore } from "../src/test/index.ts";
import { propagated } from "../src/index.ts";
import { argument, plural, t } from "@destack/locale";

/** The module an app's device worker starts from, as its tabs resolve it. */
const DEVICE_WORKER = new URL("./fixture/device.worker.ts", import.meta.url);

test("read a local object one tab writes in another tab through their device, out of the outbox", async () => {
    // run in a world of its own
    const simulation = testSimulation();
    const world = simulation.environment("test");

    const { endpoint } = await serveNotes({ simulation, node: "browser" });
    const alice = Identifier.create("user", world);

    // open two tabs of one browser sharing one device worker
    const environment = new BrowserSimulation(simulation).environment("browser");
    const objects = { notebook, note, draft };
    const caller = principal.user.reference("universe", alice);
    const browser = principal.device.reference("universe", Identifier.create("device", world));
    const worker = deviceWorker({
        database: await deviceDatabase(objects, environment),
        objects,
        caller,
        device: browser,
        endpoint: endpoint(alice),
        environment,
    });
    const open = () =>
        BrowserTab.open({
            environment,
            name: "notes",
            objects,
            scope: spaceId,
            caller,
            device: browser,
            worker,
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

test("replicate a space tab's scope chain from its device once the device's replica arrives", async () => {
    // open a device whose machine answers only once the tab asked for the space's access rows
    const simulation = testSimulation();
    const world = simulation.environment("test");
    const { endpoint } = await serveNotes({ simulation, node: "browser" });
    const alice = Identifier.create("user", world);
    const environment = new BrowserSimulation(simulation).environment("browser");
    const objects = { notebook, note, draft };
    const caller = principal.user.reference("universe", alice);
    const browser = principal.device.reference("universe", Identifier.create("device", world));
    const answering = Promise.withResolvers<void>();
    const reached = endpoint(alice);
    const served: BrowserDevice[] = [];
    const worker = deviceWorker({
        database: await deviceDatabase(objects, environment),
        objects,
        caller,
        device: browser,
        endpoint: {
            ...reached,
            fetch: async (...called) => {
                await answering.promise;

                return await present(reached.fetch, "the endpoint's fetch")(...called);
            },
        },
        environment,
        served: (opened) => served.push(opened),
    });

    // open the tab on the space, then let the machine answer
    const tab = await BrowserTab.open({
        environment,
        name: "notes",
        objects,
        scope: spaceId,
        caller,
        device: browser,
        worker,
    });
    answering.resolve();
    await tab.ready;

    // read the space's chain in the tab as the device has it, once the tab is ready
    expect({
        tab: await readChain(tab.client.database),
        device: await readChain(nth(served, 0).client.database),
    }).toEqual({ tab: [spaceId, Scope.universe.id], device: [spaceId, Scope.universe.id] });
    await tab.close();
});

test("replicate the person's own scope chain to a tab opened before the device's replica arrives", async () => {
    // serve the person's own scope, answering the device only once its tab asked
    const { caller, alice, open, answer } = await openOwnScope();
    const { tab } = await open();
    answer();
    await tab.ready;

    // read the person's chain in the tab, relayed once the device's replica arrived
    expect(
        (await Scope.chain(Snapshot.live(tab.client.database), alice)).map(
            (link) => link.object.id,
        ),
    ).toEqual([caller.id, Scope.universe.id]);
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
    const { tab } = await open();
    await tab.ready;
    const drafts = tab.client.query.draft.findMany().subscribe();
    await drafts.ready;

    // read the memo, which the device relays once its replica arrives
    const memos = tab.client.query.memo.findMany({ where: { scope: alice } }).subscribe();
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
            isMigrated: true,
            remote: ProcedureRemote.at({
                scope: spaceId,
                endpoint: endpoint(person),
                reconnect: unmoved,
                environment: environment,
            }),
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
    const tab = await BrowserTab.open({
        environment,
        name: "notes",
        objects: { notebook, note, draft },
        scope: spaceId,
        caller: principal.user.reference("universe", alice),
        device: principal.device.reference("universe", Identifier.create("device", world)),
        worker: BrowserTab.worker(DEVICE_WORKER, "notes", { name: "notes" }, environment),
    });

    expect(await refusal(tab.ready)).toEqual([
        "SERVICE_UNAVAILABLE",
        "DATABASE_UNAVAILABLE",
        "database 'notes' is unavailable: this browser cannot open it: InvalidStateError: the handle is held",
    ]);
});

/** Read the space's chain as a database holds its scope rows, nearest first. */
async function readChain(database: DatabaseConnection): Promise<string[]> {
    return (await Scope.chain(Snapshot.live(database), spaceId)).map((link) => link.object.id);
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
    const reached = served.as(alice).endpoint(memosPackage);
    const worker = deviceWorker({
        database: await deviceDatabase(objects, environment),
        objects,
        caller,
        device: browser,
        scope: alice,
        endpoint: {
            ...reached,
            fetch: async (...called) => {
                await answering.promise;

                return await present(reached.fetch, "the endpoint's fetch")(...called);
            },
        },
        environment,
    });
    const open = async () => ({
        tab: await BrowserTab.open({
            environment,
            name: "notes",
            objects,
            scope: alice,
            caller,
            device: browser,
            worker,
        }),
    });

    return {
        caller,
        alice,
        open,
        answer: () => answering.resolve(),
        home: served.as(alice).client(memosPackage),
        world,
    };
}
