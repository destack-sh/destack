import { expect, test } from "@destack/test";
import { principal } from "@destack/access";
import { channelHub } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { serveDatabase, WasmClient, type Message } from "@destack/db/browser";
import { Identifier } from "@destack/schema";
import { defineService } from "@destack/service";
import { testCallKey } from "@destack/service/test";
import { BrowserTab, type BrowserHost } from "../src/browser/index.ts";
import { ObjectClient, type ObjectAccess } from "../src/client/index.ts";
import { client, defineObject, field } from "../src/index.ts";
import { ObjectExtension } from "../src/server/index.ts";
import { serveNotes, spaceId } from "./fixture/device.ts";
import { spacePrincipal } from "./fixture/space.ts";
import { draft, panel, user } from "./fixture/local.ts";
import { note, notebook } from "./fixture/note.ts";
import { unmoved } from "./fixture/space.ts";

test("read a local object one tab writes in another tab over the shared database, keeping it out of the outbox", async () => {
    const { endpoint } = await serveNotes("sqlite");
    const alice = Identifier.create("user");

    // open two tabs sharing one database worker
    const host = browserHost(await WasmClient.memory());
    const open = () =>
        BrowserTab.open({
            name: "notes",
            objects: { notebook, note, draft },
            scope: spaceId,
            caller: principal.user.reference("universe", alice),
            client: principal.client.reference("universe", Identifier.create("client")),
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

test("keep a user's local objects from another user on the same client, and the client's own from another client", async () => {
    const { endpoint } = await serveNotes("sqlite");
    const [alice, bob] = [Identifier.create("user"), Identifier.create("user")];
    const [laptop, phone] = [Identifier.create("client"), Identifier.create("client")];

    // open the users' clients over one local database, as the laptop and as the phone
    const objects = { notebook, note, draft, panel };
    const storage = await TestDatabase.create("sqlite", []);
    await ObjectClient.migrate(storage.database, objects);
    const clientOf = (person: string, device: string) =>
        ObjectClient.open({
            database: storage.database,
            objects,
            scope: spaceId,
            caller: principal.user.reference("universe", person),
            client: principal.client.reference("universe", device),
            endpoint: endpoint(person),
            reconnect: unmoved,
            isMigrated: true,
        });
    const aliceLaptop = await clientOf(alice, laptop);
    const bobLaptop = await clientOf(bob, laptop);
    const alicePhone = await clientOf(alice, phone);

    // write alice's draft and the laptop's panel width as alice on the laptop
    await aliceLaptop.mutate(draft).create({ text: "Alice's draft" }).confirmed;
    await aliceLaptop.mutate(panel).create({ width: 320 }).confirmed;

    // read both types as each user on each client
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

test("refuse to serve local objects, and refuse local definitions reaching beyond the client", async () => {
    const { database } = await serveNotes("sqlite");
    const local = { scope: client, storage: "local", fields: {}, permissions: [] } as const;

    // refuse a server for a local type, and local types in the universe, in several scopes or with durable traits
    expect([
        failure(
            () =>
                ObjectExtension.compose(
                    [ObjectExtension.of(defineService("drafts", { objects: { draft } }), [])],
                    {
                        origin: {
                            package: defineService("drafts", { objects: { draft } }).package,
                            service: defineService("drafts", { objects: { draft } }).name,
                        },
                        principal: spacePrincipal(spaceId),
                        callKey: testCallKey,
                        database,
                    },
                ).services[0],
        ),
        failure(() =>
            defineObject({ ...local, name: "shared", plural: "shared", scope: "universe" }),
        ),
        failure(() =>
            defineObject({ ...local, name: "spread", plural: "spreads", scope: [user, notebook] }),
        ),
        failure(() => defineObject({ ...local, name: "kept", plural: "kept", inherited: {} })),
        failure(() =>
            defineObject({
                ...local,
                name: "remote",
                plural: "remotes",
                fields: { text: field.string() },
                permissions: ["edit"],
                methods: (method) => ({
                    create: method.create("edit", { isPredicted: false }),
                }),
            }),
        ),
    ]).toEqual([
        "local object draft lives on its client, and no server serves it",
        "local object shared lives in one scope type",
        "local object spread lives in one scope type",
        "local object kept takes no inherited",
        "local object remote runs method create off the client",
    ]);
});

/** Read the drafts and panel widths a client shows its caller. */
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
