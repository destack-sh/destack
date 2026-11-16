import { testSimulation, testIdentity } from "../src/test/index.ts";
import { users, userContext } from "./fixture/user.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { principal, relation, union } from "@destack/access";
import { type DatabaseConnection, defineDatabase, eq, Snapshot } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { PackageVersion } from "@destack/package";
import { type ClientOptions } from "@destack/object/client";
import { definePackage } from "@destack/object";
import { RequestId, Scope } from "@destack/sync";
import { ObjectClient } from "../src/client/index.ts";
import {
    chunk,
    CHUNK_CHARACTERS,
    Chunk,
    defineObject,
    field,
    RICH_TEXT,
    TextSequence,
    type TextEdit,
    nodeEntry,
    TextSpan,
    TextTypes,
    type TextMark,
} from "../src/index.ts";
import { implementPackage } from "../src/server/index.ts";
import { user } from "./schema.ts";
import { openSpace, space, unmoved, spacePrincipal } from "../src/test/space.ts";
import { testCallKey } from "@destack/object/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { nth, type Identifier } from "@destack/schema";
import { Environment } from "@destack/environment";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { TestObjectStore } from "../src/test/index.ts";
import { shareable } from "../src/index.ts";
import { argument, plural, t } from "@destack/locale";

/** The space with the documents. */
const spaceId = "space-01996ab0-0000-7000-8000-000000000014";

/** Documents their owner and editors write together. */
const document = defineObject(
    {
        name: "document",
        plural: "documents",
        title: t`${plural(argument("count"), { one: "Document", other: "Documents" })}`,
        scope: space,
        fields: {
            owner: field.reference(principal.user).caller(),
            title: field.string().default(""),
            body: field.text(RICH_TEXT),
            summary: field.text(),
        },
        access: {
            relations: { editor: { subjects: [user] }, viewer: { subjects: [user] } },
            permissions: {
                read: union(relation("owner"), relation("editor"), relation("viewer")),
                edit: union(relation("owner"), relation("editor")),
                manage: relation("owner"),
            },
        },
        traits: [shareable({ by: "manage" })],
    },
    (method) => ({
        create: method.create("read"),
        update: method.update("edit"),
    }),
);

/** The documents package. */
const documentsPackage = definePackage({
    ...testIdentity,
    objects: [document],
});

/** The database of one space's documents. */
const documentsDatabase = defineDatabase({
    name: "main",
    ...tablesOf({
        packages: [objectPackage, { objects: { document }, tables: [] }],
        follows: [],
        tables: [],
    }),
});

/** Serve a space's documents to bearer-identified users. */
async function serveDocuments(world: Environment) {
    // store the space's documents
    const storage = await TestDatabase.create(documentsDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, world, spaceId);

    // authenticate each request as the user its bearer credential identifies
    const served = TestObjectStore.serve(
        await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
            servers: [implementPackage(documentsPackage, {})],
            chain: async () => undefined,
            environment: world,
            origin: { package: PackageVersion.of(documentsPackage) },
            principal: spacePrincipal(spaceId),
            callKey: testCallKey,
            events: await testEvents(database, world),
            database,
        }),
    );
    onTestFinished(() => served[Symbol.asyncDispose]());
    const endpoint = (name: Identifier<"user">) => served.as(name).endpoint(documentsPackage);
    const connect = (name: Identifier<"user">) => served.as(name).client(documentsPackage);

    return { connect, endpoint, database };
}

/** Open a user's client of the space on a new local database, following every document. */
async function openClient(world: Environment, name: Identifier<"user">, endpoint: ClientOptions) {
    // create the local database and follow the documents
    const tables = ObjectClient.tables({ document });
    const storage = await TestDatabase.create(tables, {
        environment: world,
        dialect: "sqlite",
        storage: "file",
    });
    onTestFinished(() => storage.close());
    const client = await ObjectClient.open({
        environment: world,
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
            characters: new TextSequence(row.runs).runs
                .map((piece) =>
                    "text" in piece && typeof piece.text === "string" ? piece.text.length : 0,
                )
                .reduce((sum, count) => sum + count, 0),
        }));
}

/** Write the digits cycling through a length. */
function digits(length: number, offset = 0): string {
    return Array.from({ length }, (_, index) => String((index + offset) % 10)).join("");
}

test("edit a text through chunk splits, read it assembled, undo exactly and guard it", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const { connect: connectAs, database } = await serveDocuments(world);
    const alice = connectAs(users.alice);
    const bob = connectAs(users.bob);
    const created = await alice.document.create({
        spaceId,
        requestId: RequestId.create(world),
        title: "Plan",
    });
    expect(created.body).toEqual([]);

    // edit through a mirror sequence, one call per change
    let mirror = new TextSequence();
    let clock = 0;
    const edit = async (from: number, to: number, insert: string) => {
        const edits = mirror.change({ from, to, insert }, { run: "alice", offset: clock });
        clock += insert.length;
        for (const each of edits) {
            mirror = mirror.apply(each);
        }

        return alice.document.edit({
            spaceId,
            id: created.id,
            requestId: RequestId.create(world),
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
    expect((await chunks(database)).every((row) => row.characters <= CHUNK_CHARACTERS)).toBe(true);

    // delete across chunks, rewriting only the chunks it touches
    const before = await chunks(database);
    const deleted = await edit(3000, 3400, "");
    const after = await chunks(database);
    expect(after.map((row) => row.id)).toEqual(before.map((row) => row.id));
    expect(
        after.flatMap((row, index) => (row.runs === nth(before, index).runs ? [] : [index])),
    ).toEqual([7, 8]);

    // read the text assembled through get and list
    const text = mirror.text();
    expect(text.length).toBe(8000 - 400);
    expect(TextSpan.plain((await alice.document.get({ spaceId, id: created.id })).body)).toBe(text);
    expect(
        (await alice.document.list({ spaceId })).items.map((item) => TextSpan.plain(item.body)),
    ).toEqual([text]);

    // undo the deletion exactly through its inverse
    const undone = await alice.document.edit({
        spaceId,
        id: created.id,
        requestId: RequestId.create(world),
        field: "body",
        edits: deleted.inverse,
    });
    expect(TextSpan.plain((await alice.document.get({ spaceId, id: created.id })).body)).toBe(
        digits(1000) + digits(5000, 3) + digits(2000, 1000),
    );

    // redo the deletion through the undo's inverse
    await alice.document.edit({
        spaceId,
        id: created.id,
        requestId: RequestId.create(world),
        field: "body",
        edits: undone.inverse,
    });
    expect(TextSpan.plain((await alice.document.get({ spaceId, id: created.id })).body)).toBe(text);

    // refuse text in a plain update, and an edit referring to a missing element
    const replaced = {
        spaceId,
        id: created.id,
        requestId: RequestId.create(world),
        body: "replaced",
    };
    const plain = await refusal(alice.document.update(replaced));
    expect(plain).toEqual([
        "BAD_REQUEST",
        "INPUT_INVALID",
        'invalid input: unrecognized key: "body"',
    ]);
    expect(
        await refusal(
            alice.document.edit({
                spaceId,
                id: created.id,
                requestId: RequestId.create(world),
                field: "body",
                edits: [
                    {
                        insert: "x",
                        run: "alice.99",
                        start: 0,
                        after: { run: "nobody.1", offset: 0 },
                    },
                ],
            }),
        ),
    ).toEqual([
        "BAD_REQUEST",
        "ELEMENT_INVALID",
        "element 'nobody.1' at offset 0 is invalid: no chunk has it",
    ]);

    // refuse a run the text already contains anywhere
    expect(
        await refusal(
            alice.document.edit({
                spaceId,
                id: created.id,
                requestId: RequestId.create(world),
                field: "body",
                edits: [{ insert: "x", run: "alice", start: 0 }],
            }),
        ),
    ).toEqual([
        "BAD_REQUEST",
        "RUN_INVALID",
        "run 'alice' is invalid: 'body' already has its elements from 0",
    ]);

    // report which elements the text contains, deleted ones included
    const owner = document.reference(spaceId, created.id);
    const contained = [
        await Chunk.contains(Snapshot.live(database), owner, "body", [
            { run: "alice", offset: 2999 },
        ]),
        await Chunk.contains(Snapshot.live(database), owner, "body", [
            { run: "alice", offset: 3000 },
            { run: "alice", offset: 7999 },
        ]),
        await Chunk.contains(Snapshot.live(database), owner, "body", [
            { run: "alice", offset: 8000 },
        ]),
    ];
    expect(contained).toEqual([true, true, false]);

    // hide the text from a caller without read, and show it once shared
    const listed = async () =>
        (await bob.document.list({ spaceId })).items.map((item) => TextSpan.plain(item.body));
    expect(await listed()).toEqual([]);
    await alice.document.grant({
        spaceId,
        id: created.id,
        requestId: RequestId.create(world),
        relation: "editor",
        subject: principal.user.reference("universe", users.bob),
    });
    expect(await listed()).toEqual([text]);
});

test("predict edits on two clients, converge after push, undo one exactly and hide chunks from others", async () => {
    // run in a world of its own, moving its time while the work waits
    const simulation = testSimulation();
    const world = simulation.environment("test");
    await (async () => {
        const { endpoint } = await serveDocuments(world);
        const alice = await openClient(world, users.alice, endpoint(users.alice));
        const bob = await openClient(world, users.bob, endpoint(users.bob));
        const carol = await openClient(world, users.carol, endpoint(users.carol));

        // create a shared document with a first line
        const created = alice.client.mutate(document).create({ title: "Plan" });
        const { id } = await created.predicted;
        await created.confirmed;
        const base = new TextSequence().change(
            { from: 0, to: 0, insert: "hello world" },
            { run: "alice", offset: 0 },
        );
        await alice.client.mutate(document).edit({ id, field: "body", edits: base }).confirmed;
        await alice.client.mutate(document).grant({
            id,
            relation: "editor",
            subject: principal.user.reference("universe", users.bob),
        }).confirmed;
        await expect.poll(() => body(bob.live)).toEqual(["hello world"]);

        // predict one edit on each client at once and see each locally first
        const replica = base.reduce((sequence, each) => sequence.apply(each), new TextSequence());
        const shout = replica.change(
            { from: 0, to: 5, insert: "HELLO" },
            { run: "alice", offset: 11 },
        );
        const exclaim = replica.change(
            { from: 11, to: 11, insert: "!" },
            { run: "bob", offset: 0 },
        );
        const shouted = alice.client.mutate(document).edit({ id, field: "body", edits: shout });
        const exclaimed = bob.client.mutate(document).edit({ id, field: "body", edits: exclaim });
        await Promise.all([shouted.predicted, exclaimed.predicted]);
        expect([await body(alice.live), await body(bob.live)]).toEqual([
            ["HELLO world"],
            ["hello world!"],
        ]);

        // converge both replicas once both edits arrive at the server
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
    })();
});

test("type in a new run after the store refuses an edit, so later typing lands", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const { endpoint } = await serveDocuments(world);
    const alice = await openClient(world, users.alice, endpoint(users.alice));
    const bob = await openClient(world, users.bob, endpoint(users.bob));
    const share = (granted: "viewer" | "editor") =>
        alice.client.mutate(document).grant({
            id,
            relation: granted,
            subject: principal.user.reference("universe", users.bob),
        }).confirmed;

    // share a document with bob for reading only, and follow its body on his client
    const created = alice.client.mutate(document).create({ title: "Plan" });
    const { id } = await created.predicted;
    await created.confirmed;
    await share("viewer");
    await expect.poll(() => body(bob.live)).toEqual([""]);
    const text = bob.client.text(document, id, "body");
    onTestFinished(() => text.close());
    await text.ready;

    // type while bob may only read, then again once he may edit
    const refused = await refusal(text.change({ from: 0, to: 0, insert: "no" }).confirmed);
    await share("editor");
    await text.change({ from: 0, to: 0, insert: "yes" }).confirmed;
    await expect.poll(() => body(alice.live)).toEqual(["yes"]);
    expect({ refused, text: text.rope.read().text() }).toEqual({
        refused: [
            "FORBIDDEN",
            "PERMISSION_DENIED",
            "permission 'edit' is forbidden: the caller lacks it",
        ],
        text: "yes",
    });
});

test("type into a live text by offsets, one change after another, and converge on the server", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const { endpoint } = await serveDocuments(world);
    const alice = await openClient(world, users.alice, endpoint(users.alice));
    const bob = await openClient(world, users.bob, endpoint(users.bob));

    // create a document shared with bob and follow its body as a sequence
    const created = alice.client.mutate(document).create({ title: "Plan" });
    const { id } = await created.predicted;
    await created.confirmed;
    await alice.client.mutate(document).grant({
        id,
        relation: "editor",
        subject: principal.user.reference("universe", users.bob),
    }).confirmed;
    const text = alice.client.text(document, id, "body");
    onTestFinished(() => text.close());
    await text.ready;

    // type key by key without waiting, reading each change at once
    const typed = "Hello world"
        .split("")
        .map((key, offset) => text.change({ from: offset, to: offset, insert: key }));
    const local = text.rope.read();

    // converge bob's replica and keep the typing in one run
    await Promise.all(typed.map((submission) => submission.confirmed));
    await expect.poll(() => body(bob.live)).toEqual(["Hello world"]);
    const runs = text.rope.read().runs.length;
    expect([local.text(), runs, alice.errors, bob.errors]).toEqual(["Hello world", 1, [], []]);
});

test("format a body with marks and nodes, order marks on the store, index mentions, undo them, and refuse what a field's schema lacks", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const { connect: connectAs, database } = await serveDocuments(world);
    const alice = connectAs(users.alice);
    const created = await alice.document.create({
        spaceId,
        requestId: RequestId.create(world),
        title: "Plan",
    });
    const edit = (edits: TextEdit[], text: "body" | "summary" = "body") =>
        alice.document.edit({
            spaceId,
            id: created.id,
            requestId: RequestId.create(world),
            field: text,
            edits,
        });
    const read = async () => (await alice.document.get({ spaceId, id: created.id })).body;

    // type a line, then bold "hello", link "world", head it, and mention bob after it in one edit
    await edit([{ insert: "hello world", run: "alice.1", start: 0 }]);
    const line = new TextSequence().apply({ insert: "hello world", run: "alice.1", start: 0 });
    const bold: TextMark = {
        id: "mark.1",
        type: "bold",
        value: true,
        ...line.range(0, 5, TextTypes.bold),
        order: null,
    };
    const href = { href: "https://destack.sh" };
    const link: TextMark = {
        id: "mark.2",
        type: "link",
        value: href,
        ...line.range(6, 11, TextTypes.link),
        order: null,
    };
    const heading = { type: "heading", value: { level: 1 } };
    const mention = { type: "mention", value: principal.user.reference("universe", users.bob) };
    const formatted = await edit([
        { mark: bold },
        { mark: link },
        { insert: heading, run: "alice.2", start: 0 },
        { insert: mention, run: "alice.3", start: 0, after: { run: "alice.1", offset: 10 } },
    ]);

    // read the spans, the marks ordered by the store, and the indexed mention
    const marks = async () =>
        (await database.select().from(chunk)).flatMap((row) =>
            row.marks.map((mark) => [mark.id, mark.order]),
        );
    const references = async () =>
        (await database.select().from(nodeEntry))
            .filter((row) => row["nodeType"] === "mention")
            .map((row) => [row["nodeType"], row["nodeValue"]]);
    expect([await read(), await marks(), await references()]).toEqual([
        [
            { node: heading },
            { text: "hello", marks: { bold: true } },
            { text: " " },
            { text: "world", marks: { link: href } },
            { node: mention },
        ],
        [
            ["mark.1", 1],
            ["mark.2", 2],
        ],
        [["mention", mention.value]],
    ]);

    // undo the formatting exactly, dropping the marks and the mention's index row
    await edit(formatted.inverse);
    expect([await read(), await marks(), await references()]).toEqual([
        [{ text: "hello world" }],
        [],
        [],
    ]);

    // refuse a mark the body's schema lacks, and any node in the plain summary
    expect([
        await refusal(edit([{ mark: { ...bold, id: "mark.3", type: "underline" } }])),
        await refusal(edit([{ insert: heading, run: "alice.4", start: 0 }], "summary")),
    ]).toEqual([
        [
            "BAD_REQUEST",
            "TEXT_EDIT_INVALID",
            "text mark underline is invalid: the field does not allow it",
        ],
        [
            "BAD_REQUEST",
            "TEXT_EDIT_INVALID",
            "text node heading is invalid: the field does not allow it",
        ],
    ]);
});

test("continue a run as one piece on the store, apart from the same run in another text, refusing gaps and repeats", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const { connect: connectAs, database } = await serveDocuments(world);
    const alice = connectAs(users.alice);
    const created = await alice.document.create({
        spaceId,
        requestId: RequestId.create(world),
        title: "Plan",
    });
    const edit = (edits: TextEdit[]) =>
        alice.document.edit({
            spaceId,
            id: created.id,
            requestId: RequestId.create(world),
            field: "body",
            edits,
        });

    // give another document pieces of the same run, one of them starting at 3
    const other = await alice.document.create({
        spaceId,
        requestId: RequestId.create(world),
        title: "Notes",
    });
    await alice.document.edit({
        spaceId,
        id: other.id,
        requestId: RequestId.create(world),
        field: "body",
        edits: [
            { insert: "abc", run: "alice.1", start: 0 },
            { insert: "d", run: "alice.1", start: 3 },
        ],
    });

    // type a word in three continued edits of one run
    await edit([{ insert: "he", run: "alice.1", start: 0 }]);
    await edit([{ insert: "ll", run: "alice.1", start: 2, after: { run: "alice.1", offset: 1 } }]);
    await edit([{ insert: "o", run: "alice.1", start: 4, after: { run: "alice.1", offset: 3 } }]);
    const runs = (
        await database.select().from(chunk).where(eq(chunk.parentId, created.id))
    ).flatMap((row) => row.runs);

    // refuse continuing past a gap, and repeating elements the run has
    expect({
        runs,
        gap: await refusal(
            edit([{ insert: "x", run: "alice.1", start: 7, after: { run: "alice.1", offset: 4 } }]),
        ),
        repeat: await refusal(
            edit([{ insert: "x", run: "alice.1", start: 3, after: { run: "alice.1", offset: 4 } }]),
        ),
    }).toEqual({
        runs: [{ run: "alice.1", start: 0, text: "hello" }],
        gap: [
            "BAD_REQUEST",
            "RUN_INVALID",
            "run 'alice.1' is invalid: it continues at 7, where none of its elements ends",
        ],
        repeat: [
            "BAD_REQUEST",
            "RUN_INVALID",
            "run 'alice.1' is invalid: 'body' already has its elements from 3",
        ],
    });
});

test("keep a mark on its words when chunks split and move them, and grow it as people type after it", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const { connect: connectAs, database } = await serveDocuments(world);
    const alice = connectAs(users.alice);
    const created = await alice.document.create({
        spaceId,
        requestId: RequestId.create(world),
        title: "Plan",
    });
    const edit = (edits: TextEdit[]) =>
        alice.document.edit({
            spaceId,
            id: created.id,
            requestId: RequestId.create(world),
            field: "body",
            edits,
        });

    // type past one chunk and bold the last word
    const typed = `${digits(2995)} plan`;
    await edit([{ insert: typed, run: "alice.1", start: 0 }]);
    const line = new TextSequence().apply({ insert: typed, run: "alice.1", start: 0 });
    await edit([
        {
            mark: {
                id: "mark.1",
                type: "bold",
                value: true,
                ...line.range(2996, 3000, TextTypes.bold),
                order: null,
            },
        },
    ]);

    // paste into the middle, splitting chunks again, and type after the bold word
    await edit([
        { insert: digits(3000), run: "alice.2", start: 0, after: { run: "alice.1", offset: 100 } },
    ]);
    await edit([
        { insert: "s!", run: "alice.3", start: 0, after: { run: "alice.1", offset: 2999 } },
    ]);
    // read the grown bold word and its one chunk
    const rows = await database.select().from(chunk);
    const holders = rows.filter((row) => row.marks.length > 0);
    const read = (await alice.document.get({ spaceId, id: created.id })).body;
    expect({
        chunks: rows.length > 2,
        last: read.at(-1),
        holders: holders.map((row) =>
            row.runs.some(
                (piece) =>
                    piece.run === "alice.1" &&
                    piece.start <= 2996 &&
                    2996 < piece.start + TextSequence.length(piece),
            ),
        ),
    }).toEqual({ chunks: true, last: { text: "plans!", marks: { bold: true } }, holders: [true] });
});

test("predict a mark last on a client until the store orders it, then read it ordered", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const { endpoint } = await serveDocuments(world);
    const alice = await openClient(world, users.alice, endpoint(users.alice));
    const created = alice.client.mutate(document).create({ title: "Plan" });
    const { id } = await created.predicted;
    await created.confirmed;
    const text = alice.client.text(document, id, "body");
    onTestFinished(() => text.close());
    await text.ready;

    // type, then bold the first word, reading the prediction before the store confirms it
    await text.change({ from: 0, to: 0, insert: "hello world" }).confirmed;
    const sequence = text.rope.read();
    const bold: TextMark = {
        id: "mark.1",
        type: "bold",
        value: true,
        ...sequence.range(0, 5, TextTypes.bold),
        order: null,
    };
    const marked = text.edit([{ mark: bold }]);
    const predicted = text.rope.read().marks.map((mark) => mark.order);
    await marked.confirmed;
    await expect.poll(() => text.rope.read().marks.map((mark) => mark.order)).toEqual([1]);
    const spans = text.rope.read().spans(document.fields.body.schema);
    expect({ predicted, spans, errors: alice.errors }).toEqual({
        predicted: [null],
        spans: [{ text: "hello", marks: { bold: true } }, { text: " world" }],
        errors: [],
    });
});

/** Read the bodies of the documents a user's client follows. */
async function body(live: Awaited<ReturnType<typeof openClient>>["live"]): Promise<string[]> {
    return (await live.read()).map((row) => TextSpan.plain(TextSpan.array().parse(row["body"])));
}

test("tell whether a document's body has the elements a reader asks about", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // write a body of five characters in one run
    const storage = await TestDatabase.create(documentsDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([document]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: { package: document.package },
    });
    const caller = userContext(users.frank, spaceId, { environment: world }).access();
    const [created] = await store.mutate(caller, {
        id: RequestId.create(world),
        calls: [document.call("create", { spaceId })],
    });
    await store.mutate(caller, {
        id: RequestId.create(world),
        calls: [
            document.call("edit", {
                spaceId,
                id: created.id,
                field: "body",
                edits: new TextSequence().change(
                    { from: 0, to: 0, insert: "hello" },
                    { run: "user", offset: 0 },
                ),
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
        first: await contains("user", 0),
        last: await contains("user", 4),
        past: await contains("user", 5),
        other: await contains("other", 0),
    }).toEqual({ first: true, last: true, past: false, other: false });
});
