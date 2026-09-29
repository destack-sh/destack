import { AuditOutbox } from "@destack/audit/outbox";
import { expect, onTestFinished, test } from "@destack/test";
import { principal, relation, union } from "@destack/access";
import { AuditRecorder } from "@destack/audit";
import type { DatabaseConnection, Dialect } from "@destack/db";
import { defineDatabase } from "@destack/db/declare";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { Caller } from "@destack/service/authentication";
import { createClient } from "@destack/service/client";
import { defineJournal } from "@destack/service/database";
import { Health } from "@destack/service/health";
import { defineService } from "@destack/service";
import { RequestId } from "@destack/service/request";
import { Server } from "@destack/service/server";
import { ObjectClient } from "../src/client/index.ts";
import {
    chunk,
    CHUNK_CHARACTERS,
    Chunk,
    defineObject,
    field,
    method,
    Position,
    Sequence,
    type Run,
    type SequenceEdit,
} from "../src/index.ts";
import { ObjectServer } from "../src/server/index.ts";
import { user } from "./schema.ts";
import { openSpace, space, unmoved } from "./fixture/space.ts";

/** The space holding the documents. */
const spaceId = "space-01996ab0-0000-7000-8000-000000000014";

/** The package serving the documents. */
const audience = PackageId.parse("package-01a0d5eb-fb8a-74f4-ba37-8a4d6970e239");

/** Documents their owner and editors write together. */
const document = defineObject({
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
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("read"),
        update: method.update("read"),
    },
});

/** The documents service. */
const documentsService = defineService("documents", { objects: { document } });

/** Replayable method requests. */
const journal = defineJournal("journal");

/** The database of one space's documents. */
const documentsDatabase = defineDatabase({
    name: "main",
    tables: [...document.tables, journal],
});

/** Serve a space's documents to bearer-named users. */
async function serveDocuments(dialect: Dialect) {
    // hold the space's documents
    const storage = await TestDatabase.create(dialect, documentsDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, spaceId);

    // authenticate each request as the user its bearer credential names
    const server = Server.start({
        ...ObjectServer.serve(documentsService, {
            journal,
            database,
            audit: AuditRecorder.service(new AuditOutbox(database), {
                package: documentsService.package,
                service: "test",
            }),
        }),
        audience,
        scope: spaceId,
        resources: new ResourceContext(),
        health: new Health("documents"),
        drainTimeout: 1000,
        authorizeHost: async () => {},
        authenticate: async (request) => {
            const id = request.headers.get("authorization")!.slice("Bearer ".length);
            const subject = principal.user.reference("universe", id);
            const now = Date.now();

            return new Caller({
                subject,
                subjects: [subject],
                credential: { kind: "user", id },
                audience,
                scope: spaceId,
                verifiedAt: now,
                expiresAt: now + 60_000,
            });
        },
    });
    onTestFinished(() => server.close());
    const connect = (name: string) =>
        createClient(documentsService.router, {
            url: "https://documents.test",
            headers: { authorization: `Bearer ${name}` },
            fetch: (request: Request) => server.fetch(request),
        });

    return { connect, database };
}

/** The documents procedures one user calls. */
type Service = ReturnType<Awaited<ReturnType<typeof serveDocuments>>["connect"]>;

/** Open a user's client of the space on a new local database, following every document. */
async function openClient(name: string, service: Service) {
    // create the local database and follow the documents
    const tables = ObjectClient.tables([document]);
    const storage = await TestDatabase.create("sqlite", tables, { storage: "file" });
    onTestFinished(() => storage.close());
    const client = await ObjectClient.open({
        database: storage.database,
        objects: [document],
        scope: spaceId,
        caller: principal.user.reference("universe", name),
        service: service.replica,
        reconnect: unmoved,
    });
    const live = client.subscribe(document);

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
        .sort((left, right) => (left.position < right.position ? -1 : 1))
        .map((row) => ({
            id: row.id,
            runs: JSON.stringify(row.runs),
            characters: new Sequence(row.runs as Run[]).runs
                .map((piece) => (typeof piece.text === "string" ? piece.text.length : 0))
                .reduce((sum, count) => sum + count, 0),
        }));
}

/** Spell the digits cycling through a length. */
function digits(length: number, offset = 0): string {
    return Array.from({ length }, (_, index) => String((index + offset) % 10)).join("");
}

test("mint positions between others in collation-independent order, either end open", () => {
    // mint between open ends, below, above and between neighbours
    const middle = Position.between(undefined, undefined);
    const minted = [
        Position.between(undefined, middle),
        middle,
        Position.between(middle, undefined),
        Position.between("i", "j"),
        Position.between("i", "i1"),
    ];
    expect(minted).toEqual(["9", "i", "r", "ii", "i0i"]);

    // keep each between its neighbours
    expect(["i", "9", "r", "ii", "j", "i0i", "i1"].toSorted()).toEqual([
        "9",
        "i",
        "i0i",
        "i1",
        "ii",
        "j",
        "r",
    ]);
});

test("refuse text on a type without an update method, and methods writing text", () => {
    // refuse text no method edits
    expect(() =>
        defineObject({
            name: "memo",
            plural: "memos",
            scope: space,
            fields: { body: field.text() },
            permissions: ["read"],
            methods: { get: method.get("read") },
        }),
    ).toThrow(new TypeError("object memo holds text but no update method to edit it"));

    // refuse an update naming a text field
    expect(() =>
        defineObject({
            name: "memo",
            plural: "memos",
            scope: space,
            fields: { body: field.text() },
            permissions: ["read"],
            methods: {
                get: method.get("read"),
                update: method.update("read", { fields: ["body"] }),
            },
        }),
    ).toThrow(
        new TypeError("method update of memo writes text field body, which only edit changes"),
    );
});

test.for(TEST_DIALECTS)(
    "edit a text through chunk splits, read it assembled, undo exactly and guard it on %s",
    async (dialect) => {
        const { connect: connectAs, database } = await serveDocuments(dialect);
        const alice = connectAs("alice");
        const bob = connectAs("bob");
        const created = await alice.document.create({
            spaceId,
            requestId: RequestId.create(),
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
                requestId: RequestId.create(),
                field: "body",
                edits,
            });
        };

        // type past one chunk, then paste into the middle, splitting evenly
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
            after.flatMap((row, index) => (row.runs === before[index]!.runs ? [] : [index])),
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
            requestId: RequestId.create(),
            field: "body",
            edits: deleted.inverse as SequenceEdit[],
        });
        expect((await alice.document.get({ spaceId, id: created.id })).body).toBe(
            digits(1000) + digits(5000, 3) + digits(2000, 1000),
        );

        // redo the deletion through the undo's own inverse
        await alice.document.edit({
            spaceId,
            id: created.id,
            requestId: RequestId.create(),
            field: "body",
            edits: undone.inverse as SequenceEdit[],
        });
        expect((await alice.document.get({ spaceId, id: created.id })).body).toBe(text);

        // refuse text in a plain update, and an edit naming a missing element
        const plain = await alice.document
            .update({
                spaceId,
                id: created.id,
                requestId: RequestId.create(),
                body: "replaced",
            } as never)
            .catch((error: { code: string; message: string }) => [error.code, error.message]);
        expect(plain).toEqual(["BAD_REQUEST", 'invalid input: unrecognized key: "body"']);
        await expect(
            alice.document.edit({
                spaceId,
                id: created.id,
                requestId: RequestId.create(),
                field: "body",
                edits: [{ insert: "x", run: "alice.99", after: { run: "nobody.1", offset: 0 } }],
            }),
        ).rejects.toMatchObject({
            code: "BAD_REQUEST",
            message: "no chunk holds element nobody.1:0",
        });

        // refuse a run the text already holds anywhere
        await expect(
            alice.document.edit({
                spaceId,
                id: created.id,
                requestId: RequestId.create(),
                field: "body",
                edits: [{ insert: "x", run: "alice.1" }],
            }),
        ).rejects.toMatchObject({ code: "BAD_REQUEST", message: "body already holds run alice.1" });

        // report which elements the text holds, deleted ones included
        const owner = document.reference(spaceId, created.id);
        const held = [
            await Chunk.holds(database, owner, "body", [{ run: "alice.1", offset: 2999 }]),
            await Chunk.holds(database, owner, "body", [
                { run: "alice.2", offset: 0 },
                { run: "alice.2", offset: 4999 },
            ]),
            await Chunk.holds(database, owner, "body", [{ run: "alice.1", offset: 3000 }]),
        ];
        expect(held).toEqual([true, true, false]);

        // hide the text from a caller without read, and show it once shared
        const listed = async () =>
            (await bob.document.list({ spaceId })).items.map((item) => item.body);
        expect(await listed()).toEqual([]);
        await alice.document.grant({
            spaceId,
            id: created.id,
            requestId: RequestId.create(),
            relation: "editor",
            subject: principal.user.reference("universe", "bob"),
        });
        expect(await listed()).toEqual([text]);
    },
);

test.for(TEST_DIALECTS)(
    "predict edits on two clients, converge after push, undo one exactly and hide chunks from others on %s",
    async (dialect) => {
        const { connect: connectAs } = await serveDocuments(dialect);
        const alice = await openClient("alice", connectAs("alice"));
        const bob = await openClient("bob", connectAs("bob"));
        const carol = await openClient("carol", connectAs("carol"));

        // create a shared document with a first line
        const created = alice.client.mutate(document).create({ title: "Plan" });
        const { id } = await created.predicted;
        await created.confirmed;
        const base = new Sequence().change({ from: 0, to: 0, insert: "hello world" }, "alice.1");
        await alice.client.mutate(document).edit({ id, field: "body", edits: base }).confirmed;
        await alice.client.mutate(document).grant({
            id,
            relation: "editor",
            subject: principal.user.reference("universe", "bob"),
        }).confirmed;
        const body = async (live: typeof alice.live) => (await live.read()).map((row) => row.body);
        await expect.poll(() => body(bob.live)).toEqual(["hello world"]);

        // predict one edit on each client at once and see each locally first
        const copy = base.reduce((sequence, each) => sequence.apply(each), new Sequence());
        const shout = copy.change({ from: 0, to: 5, insert: "HELLO" }, "alice.2");
        const exclaim = copy.change({ from: 11, to: 11, insert: "!" }, "bob.1");
        const shouted = alice.client.mutate(document).edit({ id, field: "body", edits: shout });
        const exclaimed = bob.client.mutate(document).edit({ id, field: "body", edits: exclaim });
        await Promise.all([shouted.predicted, exclaimed.predicted]);
        expect([await body(alice.live), await body(bob.live)]).toEqual([
            ["HELLO world"],
            ["hello world!"],
        ]);

        // converge both copies once both edits reach the server
        await Promise.all([shouted.confirmed, exclaimed.confirmed]);
        await expect.poll(() => body(alice.live)).toEqual(["HELLO world!"]);
        await expect.poll(() => body(bob.live)).toEqual(["HELLO world!"]);

        // undo alice's edit exactly and keep bob's
        await alice.client.undo().confirmed;
        await expect.poll(() => body(bob.live)).toEqual(["hello world!"]);
        expect(await body(alice.live)).toEqual(["hello world!"]);

        // hold no chunks for a caller without read
        await carol.live.ready;
        const held = await carol.client.database.select().from(chunk);
        expect([await body(carol.live), held]).toEqual([[], []]);
        expect([alice.errors, bob.errors, carol.errors]).toEqual([[], [], []]);
    },
);

test.for(TEST_DIALECTS)(
    "type into a live text by offsets, one change after another, and converge on the server on %s",
    async (dialect) => {
        const { connect: connectAs } = await serveDocuments(dialect);
        const alice = await openClient("alice", connectAs("alice"));
        const bob = await openClient("bob", connectAs("bob"));

        // create a document shared with bob and follow its body as a sequence
        const created = alice.client.mutate(document).create({ title: "Plan" });
        const { id } = await created.predicted;
        await created.confirmed;
        await alice.client.mutate(document).grant({
            id,
            relation: "editor",
            subject: principal.user.reference("universe", "bob"),
        }).confirmed;
        const text = alice.client.text(document, id, "body");
        await text.ready;

        // type twice without waiting against the text of the previous change
        const typed = text.change({ from: 0, to: 0, insert: "Hello" });
        const extended = text.change({ from: 5, to: 5, insert: " world" });
        const edits = await Promise.all([typed, extended]);
        const local = (await text.read()).text();

        // converge bob's copy once both edits reach the server
        await Promise.all(edits.map((edit) => edit.confirmed));
        const body = async () => (await bob.live.read()).map((row) => row.body);
        await expect.poll(body).toEqual(["Hello world"]);
        await text.close();
        expect([local, alice.errors, bob.errors]).toEqual(["Hello world", [], []]);
    },
);

test.for(TEST_DIALECTS)(
    "edit one keystroke of a large text with the statements and chunk writes of a small one on %s",
    async (dialect) => {
        const { connect: connectAs, database } = await serveDocuments(dialect);
        const alice = connectAs("alice");

        // write a one-character text and a 200,000-character text
        const write = async (length: number) => {
            const { id } = await alice.document.create({ spaceId, requestId: RequestId.create() });
            const edits = new Sequence().change(
                { from: 0, to: 0, insert: digits(length) },
                `${id}.1`,
            );
            await alice.document.edit({
                spaceId,
                id,
                requestId: RequestId.create(),
                field: "body",
                edits,
            });

            return { id, sequence: edits.reduce((held, each) => held.apply(each), new Sequence()) };
        };
        const small = await write(1);
        const large = await write(200_000);

        // count the statements and chunk changes of one keystroke in the middle
        const keystroke = async (document: typeof small, offset: number) => {
            const before = await chunks(database);
            const statements = database.state.statements;
            const edits = document.sequence.change(
                { from: offset, to: offset, insert: "x" },
                `${document.id}.2`,
            );
            await alice.document.edit({
                spaceId,
                id: document.id,
                requestId: RequestId.create(),
                field: "body",
                edits,
            });
            const counted = database.state.statements - statements;
            const after = await chunks(database);
            const changed = after.filter(
                (row) => before.find((held) => held.id === row.id)?.runs !== row.runs,
            );

            return {
                statements: counted,
                chunks: after.length - before.length,
                changed: changed.length,
            };
        };
        // run one statement more on PostgreSQL for the fence lock
        const lock = dialect === "postgresql" ? 1 : 0;
        const smallCost = await keystroke(small, 1);
        const largeCost = await keystroke(large, 100_000);
        expect([smallCost, largeCost, (await chunks(database)).length]).toEqual([
            { statements: 18 + lock, chunks: 0, changed: 1 },
            { statements: 18 + lock, chunks: 0, changed: 1 },
            522,
        ]);
    },
);
