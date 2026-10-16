import { bearerUser, users } from "./fixture/user.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { principal, relation, union } from "@destack/access";
import { type DatabaseConnection, type Dialect, defineDatabase, Snapshot } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { Package } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { createClient, type ClientOptions } from "@destack/runtime/client";
import { Health } from "@destack/runtime/health";
import { definePackage } from "@destack/runtime";
import { RequestId } from "@destack/runtime/request";
import { Server } from "@destack/runtime/server";
import { LiveText, ObjectClient } from "../src/client/index.ts";
import { chunk, CHUNK_CHARACTERS, Chunk, defineObject, field, Sequence } from "../src/index.ts";
import { ObjectStore, defineServer } from "../src/server/index.ts";
import { user } from "./schema.ts";
import { openSpace, space, unmoved, spacePrincipal } from "./fixture/space.ts";
import { userContext } from "./fixture/user.ts";
import { testCallKey } from "@destack/runtime/test";
import { callTables, testEvents, serversOf } from "../src/test/index.ts";
import { aligned, type Identifier } from "@destack/schema";
import { Clock } from "@destack/environment";

/** The space with the documents. */
const spaceId = "space-01996ab0-0000-7000-8000-000000000014";

/** Documents their owner and editors write together. */
const document = defineObject(
    {
        name: "document",
        plural: "documents",
        scope: space,
        fields: {
            owner: field.reference(principal.user).caller(),
            title: field.string().default(""),
            body: field.text(),
        },
        relations: { editor: { subjects: [user] } },
        permissions: {
            read: union(relation("owner"), relation("editor")),
            manage: relation("owner"),
        },
        shareable: { by: "manage" },
    },
    (method) => ({
        create: method.create("read"),
        update: method.update("read"),
    }),
);

/** The documents package. */
const documentsPackage = definePackage({ objects: { document } });

/** The database of one space's documents. */
const documentsDatabase = defineDatabase({
    name: "main",
    tables: [...document.tables, ...callTables],
});

/** Serve a space's documents to bearer-identified users. */
async function serveDocuments(dialect: Dialect) {
    // store the space's documents
    const storage = await TestDatabase.create(dialect, documentsDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, spaceId);

    // authenticate each request as the user its bearer credential identifies
    const server = Server.start({
        clock: Clock.system,
        ...aligned(
            new ObjectStore({
                servers: [defineServer(documentsPackage, {})],
                chain: async () => undefined,
                clock: Clock.system,
                origin: { package: Package.of(documentsPackage) },
                principal: spacePrincipal(spaceId),
                callKey: testCallKey,
                events: await testEvents(database),
                database,
            }).packages,
            0,
        ),
        scope: spaceId,
        resources: new ResourceContext(),
        health: new Health("documents"),
        drainTimeout: 1000,
        authorizeMachine: async () => {},
        authenticate: bearerUser(documentsPackage.id, spaceId),
    });
    onTestFinished(() => server.close());
    const endpoint = (name: Identifier<"user">): ClientOptions => ({
        url: "https://documents.test",
        headers: { authorization: `Bearer ${name}` },
        fetch: (request: Request) => server.fetch(request),
    });
    const connect = (name: Identifier<"user">) => createClient(documentsPackage, endpoint(name));

    return { connect, endpoint, database };
}

/** Open a user's client of the space on a new local database, following every document. */
async function openClient(name: Identifier<"user">, endpoint: ClientOptions) {
    // create the local database and follow the documents
    const tables = ObjectClient.tables({ document });
    const storage = await TestDatabase.create("sqlite", tables, { storage: "file" });
    onTestFinished(() => storage.close());
    const client = await ObjectClient.open({
        clock: Clock.system,
        database: storage.database,
        objects: { document },
        scope: spaceId,
        caller: principal.user.reference("universe", name),
        endpoint,
        reconnect: unmoved,
    });
    const live = client.query.document.findMany().subscribe();

    // push and follow until the test finishes
    const errors: unknown[] = [];
    const stop = new AbortController();
    const running = client.run(stop.signal, (error) => errors.push(error));
    onTestFinished(async () => {
        stop.abort();
        await running;
        await client.close();
    });

    return { client, live, errors };
}

/** Read each chunk's field, position and characters, in position order. */
async function chunks(database: DatabaseConnection) {
    const rows = await database.select().from(chunk);

    return rows
        .toSorted((left, right) => (left.position < right.position ? -1 : 1))
        .map((row) => ({
            id: row.id,
            runs: JSON.stringify(row.runs),
            characters: new Sequence(row.runs).runs
                .map((piece) => (typeof piece.text === "string" ? piece.text.length : 0))
                .reduce((sum, count) => sum + count, 0),
        }));
}

/** Spell the digits cycling through a length. */
function digits(length: number, offset = 0): string {
    return Array.from({ length }, (_, index) => String((index + offset) % 10)).join("");
}

test.for(TEST_DIALECTS)(
    "edit a text through chunk splits, read it assembled, undo exactly and guard it on %s",
    async (dialect) => {
        const { connect: connectAs, database } = await serveDocuments(dialect);
        const alice = connectAs(users.alice);
        const bob = connectAs(users.bob);
        const created = await alice.document.create({
            spaceId,
            requestId: RequestId.create(Date.now()),
            title: "Plan",
        });
        expect(created.body).toBe("");

        // edit through a mirror sequence, one call per change
        let mirror = new Sequence();
        let counter = 0;
        const edit = async (from: number, to: number, insert: string) => {
            counter += 1;
            const edits = mirror.change({ from, to, insert }, `alice.${counter}`);
            for (const each of edits) {
                mirror = mirror.apply(each);
            }

            return alice.document.edit({
                spaceId,
                id: created.id,
                requestId: RequestId.create(Date.now()),
                field: "body",
                edits,
            });
        };

        // type past one chunk and paste into the middle, splitting evenly
        await edit(0, 0, digits(3000));
        expect((await chunks(database)).map((row) => row.characters)).toEqual(
            Array.from({ length: 8 }, () => 375),
        );
        await edit(1000, 1000, digits(5000, 3));
        expect((await chunks(database)).map((row) => row.characters)).toEqual([
            375,
            375,
            ...Array.from({ length: 13 }, () => 384),
            383,
            ...Array.from({ length: 5 }, () => 375),
        ]);
        expect((await chunks(database)).every((row) => row.characters <= CHUNK_CHARACTERS)).toBe(
            true,
        );

        // delete across chunks, rewriting only the chunks it touches
        const before = await chunks(database);
        const deleted = await edit(3000, 3400, "");
        const after = await chunks(database);
        expect(after.map((row) => row.id)).toEqual(before.map((row) => row.id));
        expect(
            after.flatMap((row, index) =>
                row.runs === aligned(before, index).runs ? [] : [index],
            ),
        ).toEqual([7, 8]);

        // read the text assembled through get and list
        const text = mirror.text();
        expect(text.length).toBe(8000 - 400);
        expect((await alice.document.get({ spaceId, id: created.id })).body).toBe(text);
        expect((await alice.document.list({ spaceId })).items.map((item) => item.body)).toEqual([
            text,
        ]);

        // undo the deletion exactly through its inverse
        const undone = await alice.document.edit({
            spaceId,
            id: created.id,
            requestId: RequestId.create(Date.now()),
            field: "body",
            edits: deleted.inverse,
        });
        expect((await alice.document.get({ spaceId, id: created.id })).body).toBe(
            digits(1000) + digits(5000, 3) + digits(2000, 1000),
        );

        // redo the deletion through the undo's own inverse
        await alice.document.edit({
            spaceId,
            id: created.id,
            requestId: RequestId.create(Date.now()),
            field: "body",
            edits: undone.inverse,
        });
        expect((await alice.document.get({ spaceId, id: created.id })).body).toBe(text);

        // refuse text in a plain update, and an edit referring to a missing element
        const replaced = {
            spaceId,
            id: created.id,
            requestId: RequestId.create(Date.now()),
            body: "replaced",
        };
        const plain = await refusal(alice.document.update(replaced));
        expect(plain).toEqual(["BAD_REQUEST", 'invalid input: unrecognized key: "body"']);
        expect(
            await refusal(
                alice.document.edit({
                    spaceId,
                    id: created.id,
                    requestId: RequestId.create(Date.now()),
                    field: "body",
                    edits: [
                        { insert: "x", run: "alice.99", after: { run: "nobody.1", offset: 0 } },
                    ],
                }),
            ),
        ).toEqual(["BAD_REQUEST", "no chunk has element nobody.1:0"]);

        // refuse a run the text already contains anywhere
        expect(
            await refusal(
                alice.document.edit({
                    spaceId,
                    id: created.id,
                    requestId: RequestId.create(Date.now()),
                    field: "body",
                    edits: [{ insert: "x", run: "alice.1" }],
                }),
            ),
        ).toEqual(["BAD_REQUEST", "body already has run alice.1"]);

        // report which elements the text contains, deleted ones included
        const owner = document.reference(spaceId, created.id);
        const contained = [
            await Chunk.contains(Snapshot.live(database), owner, "body", [
                { run: "alice.1", offset: 2999 },
            ]),
            await Chunk.contains(Snapshot.live(database), owner, "body", [
                { run: "alice.2", offset: 0 },
                { run: "alice.2", offset: 4999 },
            ]),
            await Chunk.contains(Snapshot.live(database), owner, "body", [
                { run: "alice.1", offset: 3000 },
            ]),
        ];
        expect(contained).toEqual([true, true, false]);

        // hide the text from a caller without read, and show it once shared
        const listed = async () =>
            (await bob.document.list({ spaceId })).items.map((item) => item.body);
        expect(await listed()).toEqual([]);
        await alice.document.grant({
            spaceId,
            id: created.id,
            requestId: RequestId.create(Date.now()),
            relation: "editor",
            subject: principal.user.reference("universe", users.bob),
        });
        expect(await listed()).toEqual([text]);
    },
);

test.for(TEST_DIALECTS)(
    "predict edits on two clients, converge after push, undo one exactly and hide chunks from others on %s",
    async (dialect) => {
        const { endpoint } = await serveDocuments(dialect);
        const alice = await openClient(users.alice, endpoint(users.alice));
        const bob = await openClient(users.bob, endpoint(users.bob));
        const carol = await openClient(users.carol, endpoint(users.carol));

        // create a shared document with a first line
        const created = alice.client.mutate(document).create({ title: "Plan" });
        const { id } = await created.predicted;
        await created.confirmed;
        const base = new Sequence().change({ from: 0, to: 0, insert: "hello world" }, "alice.1");
        await alice.client.mutate(document).edit({ id, field: "body", edits: base }).confirmed;
        await alice.client.mutate(document).grant({
            id,
            relation: "editor",
            subject: principal.user.reference("universe", users.bob),
        }).confirmed;
        await expect.poll(() => body(bob.live)).toEqual(["hello world"]);

        // predict one edit on each client at once and see each locally first
        const replica = base.reduce((sequence, each) => sequence.apply(each), new Sequence());
        const shout = replica.change({ from: 0, to: 5, insert: "HELLO" }, "alice.2");
        const exclaim = replica.change({ from: 11, to: 11, insert: "!" }, "bob.1");
        const shouted = alice.client.mutate(document).edit({ id, field: "body", edits: shout });
        const exclaimed = bob.client.mutate(document).edit({ id, field: "body", edits: exclaim });
        await Promise.all([shouted.predicted, exclaimed.predicted]);
        expect([await body(alice.live), await body(bob.live)]).toEqual([
            ["HELLO world"],
            ["hello world!"],
        ]);

        // converge both replicas once both edits reach the server
        await Promise.all([shouted.confirmed, exclaimed.confirmed]);
        await expect.poll(() => body(alice.live)).toEqual(["HELLO world!"]);
        await expect.poll(() => body(bob.live)).toEqual(["HELLO world!"]);

        // undo alice's edit exactly, leaving bob's
        await alice.client.undo().confirmed;
        await expect.poll(() => body(bob.live)).toEqual(["hello world!"]);
        expect(await body(alice.live)).toEqual(["hello world!"]);

        // store no chunks for a caller without read
        await carol.live.ready;
        const stored = await carol.client.database.select().from(chunk);
        expect([await body(carol.live), stored]).toEqual([[], []]);
        expect([alice.errors, bob.errors, carol.errors]).toEqual([[], [], []]);
    },
);

test.for(TEST_DIALECTS)(
    "type into a live text by offsets, one change after another, and converge on the server on %s",
    async (dialect) => {
        const { endpoint } = await serveDocuments(dialect);
        const alice = await openClient(users.alice, endpoint(users.alice));
        const bob = await openClient(users.bob, endpoint(users.bob));

        // create a document shared with bob and follow its body as a sequence
        const created = alice.client.mutate(document).create({ title: "Plan" });
        const { id } = await created.predicted;
        await created.confirmed;
        await alice.client.mutate(document).grant({
            id,
            relation: "editor",
            subject: principal.user.reference("universe", users.bob),
        }).confirmed;
        const text = LiveText.open(alice.client, document, id, "body");
        await text.ready;

        // type twice without waiting against the text of the previous change
        const typed = text.change({ from: 0, to: 0, insert: "Hello" });
        const extended = text.change({ from: 5, to: 5, insert: " world" });
        const edits = await Promise.all([typed, extended]);
        const local = (await text.read()).text();

        // converge bob's replica once both edits reach the server
        await Promise.all(edits.map((edit) => edit.confirmed));
        await expect.poll(() => body(bob.live)).toEqual(["Hello world"]);
        await text.close();
        expect([local, alice.errors, bob.errors]).toEqual(["Hello world", [], []]);
    },
);

/** Read the bodies of the documents a user's client follows. */
async function body(live: Awaited<ReturnType<typeof openClient>>["live"]): Promise<string[]> {
    return (await live.read()).map((row) => row["body"]);
}

test("tell whether a document's body has the elements a reader asks about", async () => {
    // write a body of five characters in one run
    const storage = await TestDatabase.create("sqlite", documentsDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([document]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: { package: document.package },
    });
    const caller = userContext(users.frank, spaceId).access();
    const [created] = await store.mutate(caller, {
        id: RequestId.create(Date.now()),
        calls: [document.call("create", { spaceId })],
    });
    await store.mutate(caller, {
        id: RequestId.create(Date.now()),
        calls: [
            document.call("edit", {
                spaceId,
                id: created.id,
                field: "body",
                edits: new Sequence().change({ from: 0, to: 0, insert: "hello" }, "user.1"),
            }),
        ],
    });
    const contains = async (run: string, offset: number) =>
        (
            await store.query(
                caller,
                document.call("contains", {
                    spaceId,
                    id: created.id,
                    field: "body",
                    elements: [{ run, offset }],
                }),
            )
        ).isContained;

    // find the run's first and last elements, and neither one past its end nor another run's
    expect({
        first: await contains("user.1", 0),
        last: await contains("user.1", 4),
        past: await contains("user.1", 5),
        other: await contains("user.2", 0),
    }).toEqual({ first: true, last: true, past: false, other: false });
});
