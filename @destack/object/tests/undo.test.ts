import { worldTelemetry } from "../src/test/world.ts";
import { testSimulation, testIdentity } from "../src/test/index.ts";
import { users } from "./fixture/user.ts";
import { PackageVersion } from "@destack/package";
import { expect, onTestFinished, Recording, test } from "@destack/test";
import { defineDatabase, eq, type DatabaseConnection } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { definePackage } from "@destack/object";
import { RequestId, Scope } from "@destack/sync";
import { testCallKey } from "@destack/object/test";
import { testEvents } from "../src/test/index.ts";
import { ObjectClient } from "../src/client/index.ts";
import { undoEntry } from "../src/client/undo.ts";
import { defineObject, field, type Step } from "../src/index.ts";
import { implementPackage } from "../src/server/index.ts";
import { found, schema } from "@destack/schema";
import { serveNotes, spaceId } from "./fixture/device.ts";
import { note, notebook } from "../src/test/note.ts";
import { openSpace, space, spacePrincipal } from "../src/test/space.ts";
import { principal } from "@destack/access";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { TestObjectStore } from "../src/test/index.ts";
import { archivable, nested, recoverable, shareable } from "../src/index.ts";
import { argument, plural, t } from "@destack/locale";

/** Folders with pages. */
const folder = defineObject(
    {
        name: "folder",
        plural: "folders",
        title: t`${plural(argument("count"), { one: "Folder", other: "Folders" })}`,
        scope: space,
        fields: { name: field.string() },
        access: {
            permissions: ["write"],
        },
    },
    (method) => ({ create: method.create("write") }),
);

/** Pages with every trait that undo inverts. */
const page = defineObject(
    {
        name: "page",
        plural: "pages",
        title: t`${plural(argument("count"), { one: "Page", other: "Pages" })}`,
        scope: space,
        fields: {
            title: field.string(),
            body: field.string().optional(),
            status: field.state({
                initial: "draft",
                transitions: {
                    publish: { from: ["draft"], to: "published", permission: "write" },
                    retract: { from: ["published"], to: "draft", permission: "write" },
                },
            }),
        },
        access: {
            permissions: ["read", "write"],
        },
        traits: [
            nested({ in: folder, add: "write", move: "write" }),
            recoverable({ within: { days: 30 }, by: "write" }),
            archivable({ by: "write" }),
            shareable({ by: "write" }),
        ],
    },
    (method) => ({
        create: method.create("write"),
        createMany: method.createMany("write", { fields: ["title"] }),
        update: method.update("write"),
        deleteMany: method.deleteMany("write", { match: ["status"] }),
        announce: method.mutation({ title: t`Announce`, permission: "write" }),
    }),
);

/** Tallies with an exact count, stored as an exact integer. */
const tally = defineObject({
    name: "tally",
    plural: "tallies",
    title: t`${plural(argument("count"), { one: "Tally", other: "Tallies" })}`,
    scope: space,
    fields: { count: field.bigint(), label: field.string() },
    traits: [shareable({})],
});

/** Stamps with their bytes, stored as binary. */
const stamp = defineObject({
    name: "stamp",
    plural: "stamps",
    title: t`${plural(argument("count"), { one: "Stamp", other: "Stamps" })}`,
    scope: space,
    fields: { bytes: field.binary(), label: field.string() },
    traits: [shareable({})],
});

test("invert each trait's methods into the calls that undo them, and leave the rest undoable by nothing", () => {
    // invert a call of the page type
    const target = { spaceId: "space-1", id: "page-1" };
    const methods = new Map(Object.entries(page.methods));
    const invert = (name: string, step: Omit<Step, "object" | "name">) =>
        found(methods, name).inverse?.({
            object: page,
            name,
            ...step,
        });

    // delete a created page, restore a deleted one, and delete a restored one again
    expect([
        invert("create", { input: { spaceId: "space-1", title: "A" }, after: { id: "page-1" } }),
        invert("delete", { input: target }),
        invert("restore", { input: target }),
    ]).toEqual([
        [
            {
                type: page.typeReference,
                method: "delete",
                release: page.package.version,
                edition: 1,
                input: target,
            },
        ],
        [
            {
                type: page.typeReference,
                method: "restore",
                release: page.package.version,
                edition: 1,
                input: target,
            },
        ],
        [
            {
                type: page.typeReference,
                method: "delete",
                release: page.package.version,
                edition: 1,
                input: target,
            },
        ],
    ]);

    // delete each page a bulk creation created, and leave a bulk deletion undoable by nothing
    const created = { spaceId: "space-1", id: "page-2" };
    expect([
        invert("createMany", {
            input: { spaceId: "space-1", items: [{ title: "A" }, { title: "B" }] },
            result: { ids: ["page-1", "page-2"] },
        }),
        invert("deleteMany", { input: { spaceId: "space-1", where: { status: "draft" } } }),
    ]).toEqual([
        [
            {
                type: page.typeReference,
                method: "delete",
                release: page.package.version,
                edition: 1,
                input: target,
            },
            {
                type: page.typeReference,
                method: "delete",
                release: page.package.version,
                edition: 1,
                input: created,
            },
        ],
        undefined,
    ]);

    // restore only fields unchanged since
    const update = {
        input: { ...target, title: "B", body: "x" },
        before: { title: "A", body: null },
        after: { title: "B", body: "x" },
    };
    expect([
        invert("update", { ...update, current: { title: "B", body: "y" } }),
        invert("update", { ...update, current: { title: "C", body: "y" } }),
        invert("update", update),
    ]).toEqual([
        [
            {
                type: page.typeReference,
                method: "update",
                release: page.package.version,
                edition: 1,
                input: { ...target, title: "A" },
            },
        ],
        undefined,
        undefined,
    ]);

    // move back under the parent the page left, and transition back to the state it left
    expect([
        invert("move", {
            input: { ...target, parentId: "folder-2" },
            before: { parentId: "folder-1" },
        }),
        invert("publish", { input: target, before: { status: "draft" } }),
        invert("retract", { input: target, before: { status: "published" } }),
    ]).toEqual([
        [
            {
                type: page.typeReference,
                method: "move",
                release: page.package.version,
                edition: 1,
                input: { ...target, parentId: "folder-1" },
            },
        ],
        [
            {
                type: page.typeReference,
                method: "retract",
                release: page.package.version,
                edition: 1,
                input: target,
            },
        ],
        [
            {
                type: page.typeReference,
                method: "publish",
                release: page.package.version,
                edition: 1,
                input: target,
            },
        ],
    ]);

    // undo grants, archiving and unarchiving, and leave the rest undoable by nothing
    expect([
        invert("grant", {
            input: { ...target, relation: "reader" },
            result: { id: "relationship-1" },
        }),
        invert("archive", { input: target }),
        invert("unarchive", { input: target }),
        invert("announce", { input: target }),
        invert("revoke", { input: { ...target, relationshipId: "relationship-1" } }),
    ]).toEqual([
        [
            {
                type: page.typeReference,
                method: "revoke",
                release: page.package.version,
                edition: 1,
                input: { ...target, relationshipId: "relationship-1" },
            },
        ],
        [
            {
                type: page.typeReference,
                method: "unarchive",
                release: page.package.version,
                edition: 1,
                input: target,
            },
        ],
        [
            {
                type: page.typeReference,
                method: "archive",
                release: page.package.version,
                edition: 1,
                input: target,
            },
        ],
        undefined,
        undefined,
    ]);
});

test("undo and redo a party's mutations, restoring only what no one changed since, across a restart", async () => {
    const { connect, connection, openDevice, environment } = await serveNotes();
    const alice = connect(users.alice);
    const device = await openDevice(users.alice);
    device.online();
    // create a note, retitle it and write its text, both confirmed
    const created = device.client.mutate(note).create({ title: "Draft" });
    const { id } = await created.predicted;
    await created.confirmed;
    await device.client.mutate(note).update({ id, title: "Plan", text: "first" }).confirmed;
    const read = async (client: ObjectClient) => {
        const [row] = await client.database
            .select({
                title: note.table.title,
                text: note.table.text,
                deletedAt: note.table.deletedAt,
            })
            .from(note.table)
            .where(eq(note.table.id, id));

        return row;
    };

    // undo the update, restoring only the untouched title
    await alice.note.update({
        spaceId,
        id,
        requestId: RequestId.create(environment),
        text: "second",
    });
    await expect.poll(() => read(device.client)).toMatchObject({ text: "second" });
    await device.client.undoStack.undo().confirmed;
    expect(await read(device.client)).toEqual({
        title: "Draft",
        text: "second",
        deletedAt: null,
    });

    // undo the creation into the trash and redo it
    const trashed = async () =>
        (await alice.note.list({ spaceId, deleted: "only" })).items.map((item) => item.id);
    await device.client.undoStack.undo().confirmed;
    expect([await read(device.client), await trashed()]).toEqual([undefined, [id]]);
    await device.client.undoStack.redo().confirmed;
    await expect.poll(() => read(device.client)).toMatchObject({ deletedAt: null });
    expect(await trashed()).toEqual([]);

    // retain the client's stack across a restart, and redo the update from it
    await device.offline();
    const restarted = await ObjectClient.open({
        environment,
        database: await device.storage.connect(ObjectClient.tables({ notebook, note })),
        objects: { notebook, note },
        scope: spaceId,
        caller: principal.user.reference("universe", users.alice),
        isMigrated: true,
        remote: connection(users.alice).at(spaceId),
    });
    const stopping = new AbortController();
    const running = restarted.run(stopping.signal);
    await restarted.undoStack.redo().confirmed;
    expect(await read(restarted)).toEqual({
        title: "Plan",
        text: "second",
        deletedAt: null,
    });

    // leave nothing to redo, and undo both done entries by two quick undos, one entry each
    expect(await restarted.undoStack.redo().predicted).toBe(false);
    const quick = [restarted.undoStack.undo(), restarted.undoStack.undo()];
    expect(await Promise.all(quick.map((undone) => undone.predicted))).toEqual([true, true]);
    await Promise.all(quick.map((undone) => undone.confirmed));
    expect(await restarted.undoStack.undo().predicted).toBe(false);
    stopping.abort();
    await running;
    await restarted.close();
    expect(device.errors).toEqual([]);
});

test("group a party's updates of one note into one undo, apart across notes, closes and pauses", async () => {
    const simulation = testSimulation();
    const { openDevice, environment } = await serveNotes({ simulation });
    const device = await openDevice(users.alice);
    device.online();
    const notes = device.client.mutate(note);
    const titles = async () => {
        const rows = await device.client.database
            .select({ id: note.table.id, title: note.table.title })
            .from(note.table);

        return Object.fromEntries(rows.map((row) => [row.id, row.title]));
    };

    // create two notes, then pause
    const first = await notes.create({ title: "a" }).predicted;
    const second = await notes.create({ title: "b" }).predicted;
    await simulation.jump(environment.clock.now() + 1000);

    // retitle the first note three times, then the second once, again after a close, and again after a pause
    for (const title of ["a1", "a2", "a3"]) {
        await notes.update({ id: first.id, title }).predicted;
    }
    await notes.update({ id: second.id, title: "b1" }).predicted;
    await device.client.undoStack.close();
    await notes.update({ id: second.id, title: "b2" }).predicted;
    await simulation.jump(environment.clock.now() + 1000);
    await notes.update({ id: second.id, title: "b3" }).confirmed;

    // undo the pause's update, the closed one, the second note's first one, then the first note's three at once
    const undone = [];
    for (let step = 0; step < 4; step += 1) {
        await device.client.undoStack.undo().confirmed;
        undone.push(await titles());
    }
    expect(undone).toEqual([
        { [first.id]: "a3", [second.id]: "b2" },
        { [first.id]: "a3", [second.id]: "b1" },
        { [first.id]: "a3", [second.id]: "b" },
        { [first.id]: "a", [second.id]: "b" },
    ]);
    expect(device.errors).toEqual([]);
});

test("compose a party's queued updates into one mutation per undo group, apart across groups", async () => {
    const { connect, openDevice } = await serveNotes();
    const alice = connect(users.alice);
    const device = await openDevice(users.alice);
    const notes = device.client.mutate(note);
    const queued = async () =>
        (await device.client.prediction.pending(device.client.database)).map((mutation) =>
            mutation.calls.map((call) => [call.method, call.input["title"]]),
        );

    // create a note, retitle it three times, then once more after a close, all offline
    const created = notes.create({ title: "a" });
    const { id } = await created.predicted;
    await device.client.undoStack.close();
    const retitles = ["a1", "a2", "a3"].map((title) => notes.update({ id, title }));
    await Promise.all(retitles.map((retitled) => retitled.predicted));
    await device.client.undoStack.close();
    const closed = notes.update({ id, title: "a4" });
    await closed.predicted;
    expect(await queued()).toEqual([[["create", "a"]], [["update", "a3"]], [["update", "a4"]]]);

    // execute each composed mutation once, confirming every call in it
    device.online();
    await Promise.all([created, ...retitles, closed].map((submitted) => submitted.confirmed));
    expect((await alice.note.get({ spaceId, id })).title).toBe("a4");

    // undo the closed update, then the three composed ones at once
    await device.client.undoStack.undo().confirmed;
    await device.client.undoStack.undo().confirmed;
    expect((await alice.note.get({ spaceId, id })).title).toBe("a");
    expect(device.errors).toEqual([]);
});

test("compose a drag's updates in as many replica statements as its second", async () => {
    const simulation = testSimulation();
    const { openDevice } = await serveNotes({ simulation });
    const device = await openDevice(users.alice);
    const notes = device.client.mutate(note);
    const { id } = await notes.create({ title: "0" }).predicted;
    await device.client.undoStack.close();

    // count the device's statements for one update of the note
    const counted = async (title: string) => {
        const before = await Recording.read(simulation.telemetry);
        await notes.update({ id, title }).predicted;

        return (await Recording.read(simulation.telemetry))
            .since(before)
            .count(TestDatabase.statements(device.storage.database));
    };

    // start the drag, then count its second and its six hundredth update, ten seconds at 60 Hz
    await counted("1");
    const second = await counted("2");
    for (let index = 3; index < 600; index += 1) {
        await notes.update({ id, title: String(index) }).predicted;
    }
    const last = await counted("600");

    // compose every update into the one queued mutation, at a constant cost
    const queued = await device.client.prediction.pending(device.client.database);
    expect(queued.map((mutation) => mutation.calls.map((call) => call.input["title"]))).toEqual([
        ["0"],
        ["600"],
    ]);
    expect({ second, last }).toEqual({ second: 19, last: 19 });
});

test("follow whether a party can undo and redo as its stacks change", async () => {
    const { openDevice } = await serveNotes();
    const device = await openDevice(users.alice);
    const stopping = new AbortController();
    const watched = device.client.undoStack.watch(stopping.signal);
    const next = async () => {
        const step = await watched.next();
        if (step.done === true) {
            throw new TypeError("the stacks stopped before the next change");
        }

        return step.value;
    };

    // read the empty stacks, then after a creation and its undo
    const empty = await next();
    await device.client.mutate(note).create({ title: "Draft" }).predicted;
    const created = await next();
    await device.client.undoStack.undo().predicted;
    const undone = await next();
    stopping.abort();
    await watched.return(undefined);

    expect([empty, created, undone]).toEqual([
        { canUndo: false, canRedo: false },
        { canUndo: true, canRedo: false },
        { canUndo: false, canRedo: true },
    ]);
    await device.client.close();
});

test("store only a party's latest hundred undo groups, dropping the oldest", async () => {
    const { openDevice } = await serveNotes();
    const device = await openDevice(users.alice);

    // create a hundred and one notes offline, each its own group
    for (let count = 0; count < 101; count += 1) {
        await device.client.mutate(note).create({ title: `${count}` }).predicted;
    }

    // read the groups left, all but the first, by the titles they created
    const kept = await device.client.database.select({ steps: undoEntry.steps }).from(undoEntry);
    const Created = schema.tuple([
        schema.looseObject({ input: schema.looseObject({ title: schema.string() }) }),
    ]);
    const titles = kept.map((entry) => Created.parse(entry.steps)[0].input.title);
    expect(titles.toSorted()).toEqual(
        Array.from({ length: 100 }, (_, count) => `${count + 1}`).toSorted(),
    );
    await device.client.close();
});

test("undo an update of an exact integer, restoring its value on the device and the server", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // serve the tallies of a space
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({
                packages: [objectPackage, { objects: { tally }, tables: [] }],
                follows: [],
                tables: [],
            }),
        }),
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    const tallies = definePackage({
        ...testIdentity,
        objects: [tally],
    });
    const served = TestObjectStore.serve(
        await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
            servers: [implementPackage(tallies, {})],
            chain: async () => undefined,
            environment: world,
            origin: { package: PackageVersion.of(tallies) },
            principal: spacePrincipal(spaceId),
            callKey: testCallKey,
            events: await testEvents(storage.database, world),
            database: storage.database,
        }),
    );
    onTestFinished(() => served[Symbol.asyncDispose]());

    // store the space's tallies on a device that pushes and follows
    const local = await TestDatabase.create(ObjectClient.tables({ tally }), {
        environment: world,
        dialect: "sqlite",
        storage: "file",
    });
    onTestFinished(() => local.close());
    const client = await ObjectClient.open({
        environment: world,
        database: local.database,
        objects: { tally },
        scope: spaceId,
        caller: principal.user.reference("universe", users.alice),
        remote: served.as(users.alice).connect(tallies, { scope: spaceId }),
    });
    client.queryType(tally).findMany().subscribe();
    const stopping = new AbortController();
    const loops = [client.follow(stopping.signal), client.push(stopping.signal)];

    // create a tally beyond the exact range of numbers and change its count and label
    const created = client.mutate(tally).create({ count: (2n ** 62n).toString(), label: "a" });
    const { id } = await created.predicted;
    await created.confirmed;
    await client.mutate(tally).update({ id, count: "-7", label: "b" }).confirmed;

    // undo the change on the device and through the server
    await client.undoStack.undo().confirmed;
    const read = async (database: DatabaseConnection) =>
        database
            .select({ count: tally.table.count, label: tally.table.label })
            .from(tally.table)
            .where(eq(tally.table.id, id));
    stopping.abort();
    await Promise.all(loops);
    expect([
        await read(client.database),
        await read(storage.database),
        worldTelemetry(client.environment).exceptions(),
    ]).toEqual([[{ count: 2n ** 62n, label: "a" }], [{ count: 2n ** 62n, label: "a" }], []]);
    await client.close();
});

test("undo an update of a bytes field, restoring its bytes on the device and the server", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // serve the stamps of a space
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({
                packages: [objectPackage, { objects: { stamp }, tables: [] }],
                follows: [],
                tables: [],
            }),
        }),
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    const stamps = definePackage({
        ...testIdentity,
        objects: [stamp],
    });
    const served = TestObjectStore.serve(
        await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
            servers: [implementPackage(stamps, {})],
            chain: async () => undefined,
            environment: world,
            origin: { package: PackageVersion.of(stamps) },
            principal: spacePrincipal(spaceId),
            callKey: testCallKey,
            events: await testEvents(storage.database, world),
            database: storage.database,
        }),
    );
    onTestFinished(() => served[Symbol.asyncDispose]());

    // store the space's stamps on a device that pushes and follows
    const local = await TestDatabase.create(ObjectClient.tables({ stamp }), {
        environment: world,
        dialect: "sqlite",
        storage: "file",
    });
    onTestFinished(() => local.close());
    const client = await ObjectClient.open({
        environment: world,
        database: local.database,
        objects: { stamp },
        scope: spaceId,
        caller: principal.user.reference("universe", users.alice),
        remote: served.as(users.alice).connect(stamps, { scope: spaceId }),
    });
    client.queryType(stamp).findMany().subscribe();
    const stopping = new AbortController();
    const loops = [client.follow(stopping.signal), client.push(stopping.signal)];

    // create a stamp of every byte value and change its bytes and label
    const original = Uint8Array.from({ length: 256 }, (_, index) => index);
    const created = client.mutate(stamp).create({ bytes: original.toBase64(), label: "a" });
    const { id } = await created.predicted;
    await created.confirmed;
    await client.mutate(stamp).update({ id, bytes: Uint8Array.of(7).toBase64(), label: "b" })
        .confirmed;

    // undo the change on the device and through the server
    await client.undoStack.undo().confirmed;
    const read = async (database: DatabaseConnection) =>
        database
            .select({ bytes: stamp.table.bytes, label: stamp.table.label })
            .from(stamp.table)
            .where(eq(stamp.table.id, id));
    stopping.abort();
    await Promise.all(loops);
    expect([
        await read(client.database),
        await read(storage.database),
        worldTelemetry(client.environment).exceptions(),
    ]).toEqual([[{ bytes: original, label: "a" }], [{ bytes: original, label: "a" }], []]);
    await client.close();
});
