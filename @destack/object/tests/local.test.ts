import { defineObject, device, field, ObjectDescription } from "../src/index.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { AccessContext, accessTables, Caller, CONTAINED, principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { RequestId } from "@destack/runtime/request";
import { Scope, type Subject } from "@destack/sync";
import { channelHub } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { serveDatabase, WasmClient, type Message } from "@destack/db/browser";
import { Identifier, nth } from "@destack/schema";
import { definePackage } from "@destack/runtime";
import { testCallKey } from "@destack/runtime/test";
import { callTables, testEvents, serversOf } from "../src/test/index.ts";
import { BrowserTab, type BrowserHost } from "../src/browser/index.ts";
import { ObjectClient, type ObjectAccess } from "../src/client/index.ts";
import { ObjectStore, defineServer } from "../src/server/index.ts";
import { serveNotes, spaceId } from "./fixture/device.ts";
import { spacePrincipal } from "./fixture/space.ts";
import { draft, panel, sticky, user } from "./fixture/local.ts";
import { note, notebook } from "./fixture/note.ts";
import { unmoved } from "./fixture/space.ts";
import { Clock } from "@destack/environment";

test("read a local object one tab writes in another tab over the shared database, out of the outbox", async () => {
    const { endpoint } = await serveNotes("sqlite");
    const alice = Identifier.create("user");

    // open two tabs sharing one database worker
    const host = browserHost(await WasmClient.memory());
    const open = () =>
        BrowserTab.open({
            clock: Clock.system,
            name: "notes",
            objects: { notebook, note, draft },
            scope: spaceId,
            caller: principal.user.reference("universe", alice),
            device: principal.device.reference("universe", Identifier.create("device")),
            endpoint: endpoint(alice),
            reconnect: unmoved,
            host: host(),
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

test("hide a user's local objects from another user on the same device, and the device's own from another device", async () => {
    const { endpoint } = await serveNotes("sqlite");
    const [alice, bob] = [Identifier.create("user"), Identifier.create("user")];
    const [laptop, phone] = [Identifier.create("device"), Identifier.create("device")];

    // open the users' object clients over one local database, as the laptop and as the phone
    const objects = { notebook, note, draft, panel };
    const storage = await TestDatabase.create("sqlite", []);
    await ObjectClient.migrate(storage.database, objects);
    const openClient = (person: Identifier<"user">, on: string) =>
        ObjectClient.open({
            clock: Clock.system,
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

test("refuse to serve local objects at another principal's store, and refuse local definitions reaching beyond the device", async () => {
    const { database } = await serveNotes("sqlite");
    const local = { scope: device, storage: "local", fields: {}, permissions: [] } as const;
    const events = await testEvents(database);

    // refuse a server for a local type, and local types in the universe, in several scopes or with durable traits
    expect([
        failure(() =>
            nth(
                new ObjectStore({
                    servers: [defineServer(definePackage({ objects: { draft } }), {})],
                    chain: async () => undefined,
                    clock: Clock.system,
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
        "local object draft lives only in its scope's own store",
        "local object shared must live in one scope type",
        "local object spread must live in one scope type",
        "local object sticky cannot have inherited",
        "local object remote runs method create off the client",
    ]);
});

/** Call as a signed-in user who lives inside the given subjects. */
function newCaller(id: string, inside: readonly Subject[]): AccessContext {
    const person = Caller.of(principal.user.reference(Scope.universe.id, id));

    return AccessContext.of({ ...person, groups: [...person.groups, ...inside] }, Date.now());
}

test("serve a device's local objects at its own store to the people signed in on it alone", async () => {
    // store a laptop's stickies
    const [ada, bob] = [Identifier.create("user"), Identifier.create("user")];
    const laptop = device.reference(Scope.universe.id, Identifier.create("device"));
    const storage = await TestDatabase.create(
        "sqlite",
        [...accessTables, sticky.table, ...callTables],
        {
            isMigrated: true,
        },
    );
    onTestFinished(() => storage.close());
    await new AccessFixture(storage.database).replicateScope(laptop);

    // serve them at the laptop's own store
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: principal.device.reference(Scope.universe.id, laptop.id),
        servers: serversOf([sticky]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: { package: sticky.package },
    });

    // call as ada signed in on the laptop, who lives inside it, and as bob signed in elsewhere
    const callers = {
        ada: newCaller(ada, [{ ...laptop, relation: CONTAINED }]),
        bob: newCaller(bob, []),
    };
    const write = (caller: AccessContext, text: string) =>
        refusal(
            store.mutate(caller, {
                id: RequestId.create(Date.now()),
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
        bobWrites: ["NOT_FOUND", "sticky not found"],
        adaLists: ["Water the plants"],
        bobLists: ["NOT_FOUND", `no scope ${laptop.id}`],
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

/** Read a call's refusal as its code and message. */
function readRefusal(error: unknown): Promise<"done" | readonly [string, string]> {
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

/** Share one channel hub, database worker and lock queue between the tabs of one browser. */
function browserHost(database: WasmClient): () => BrowserHost {
    const join = channelHub<Message>();
    const locks = new Map<string, Promise<void>>();

    return () => ({
        channel: join(),
        request: (name, callback, signal) =>
            new Promise<void>((resolve, reject) => {
                // reject an aborted wait
                let isGranted = false;
                signal?.addEventListener(
                    "abort",
                    () => {
                        if (!isGranted) {
                            reject(signal.reason);
                        }
                    },
                    { once: true },
                );

                // queue behind the lock's holder and take it unless the wait aborted
                const previous = locks.get(name) ?? Promise.resolve();
                const next = previous.then(async () => {
                    if (signal?.aborted !== true) {
                        isGranted = true;
                        await callback();
                    }
                });
                locks.set(
                    name,
                    next.catch(() => undefined),
                );
                next.then(resolve, reject);
            }),
        start: async () => {
            const stop = serveDatabase(database, join());

            return async () => stop();
        },
    });
}
