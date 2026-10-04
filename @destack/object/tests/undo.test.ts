import { expect, onTestFinished, test } from "@destack/test";
import { journal } from "@destack/audit";
import {
    bigint,
    binary,
    defineDatabase,
    eq,
    type ColumnBuilder,
    type ColumnDefinition,
    type ColumnValue,
    type DatabaseConnection,
} from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { defineService } from "@destack/service";
import { RequestId } from "@destack/service/request";
import { testCallKey } from "@destack/service/test";
import { ObjectClient } from "../src/client/index.ts";
import { defineObject, Field, field, type Step } from "../src/index.ts";
import { ObjectServer } from "../src/server/index.ts";
import { found } from "@destack/schema";
import { Device, serveNotes, serveObjects, spaceId } from "./fixture/device.ts";
import { note, notebook } from "./fixture/note.ts";
import { openSpace, space, unmoved } from "./fixture/space.ts";
import { principal } from "@destack/access";

/** Folders with pages. */
const folder = defineObject({
    name: "folder",
    plural: "folders",
    scope: space,
    fields: { name: field.string() },
    permissions: ["write"],
    methods: (method) => ({ create: method.create("write") }),
});

/** Pages with every trait that undo inverts. */
const page = defineObject({
    name: "page",
    plural: "pages",
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
    nested: { in: folder, receive: "write", move: "write" },
    recoverable: { within: { days: 30 }, by: "write" },
    permissions: ["read", "write"],
    shareable: { by: "write" },
    methods: (method) => ({
        create: method.create("write"),
        update: method.update("write"),
        archive: method.mutation({ permission: "write", inverse: "unarchive" }),
        unarchive: method.mutation({ permission: "write" }),
        announce: method.mutation({ permission: "write" }),
    }),
});

/** Tallies with an exact count, kept as an exact integer. */
const tally = defineObject({
    name: "tally",
    plural: "tallies",
    scope: space,
    fields: { count: kept(bigint), label: field.string() },
    shareable: {},
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("edit"),
        update: method.update("edit"),
    }),
});

/** Stamps with their bytes, kept as binary. */
const stamp = defineObject({
    name: "stamp",
    plural: "stamps",
    scope: space,
    fields: { bytes: kept(binary), label: field.string() },
    shareable: {},
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("edit"),
        update: method.update("edit"),
    }),
});

/** The flags of a required field callers write, kept in a column of a value. */
type Kept<Value extends ColumnValue> = {
    readonly value: Value;
    readonly required: true;
    readonly default: false;
    readonly guarded: false;
    readonly written: true;
    readonly sensitive: false;
    readonly caller: false;
    readonly reference: false;
    readonly relation: never;
};

/** Build a required field callers write, kept in a column a column constructor builds. */
function kept<Value extends ColumnValue>(
    build: (name: string) => ColumnBuilder<ColumnDefinition<Value>>,
): Field<Kept<Value>> {
    return new Field<Kept<Value>>({
        type: "json",
        build,
        required: true,
        isDefault: false,
        guarded: false,
        written: true,
        isSensitive: false,
        isCallerFilled: false,
        isReference: false,
    });
}

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
        [{ method: "page.delete", release: page.package.version, input: target }],
        [{ method: "page.restore", release: page.package.version, input: target }],
        [{ method: "page.delete", release: page.package.version, input: target }],
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
                method: "page.update",
                release: page.package.version,
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
                method: "page.move",
                release: page.package.version,
                input: { ...target, parentId: "folder-1" },
            },
        ],
        [{ method: "page.retract", release: page.package.version, input: target }],
        [{ method: "page.publish", release: page.package.version, input: target }],
    ]);

    // undo grants and custom methods with inverses
    expect([
        invert("grant", {
            input: { ...target, relation: "reader" },
            result: { id: "relationship-1" },
        }),
        invert("archive", { input: target }),
        invert("announce", { input: target }),
        invert("revoke", { input: { ...target, relationshipId: "relationship-1" } }),
    ]).toEqual([
        [
            {
                method: "page.revoke",
                release: page.package.version,
                input: { ...target, relationshipId: "relationship-1" },
            },
        ],
        [{ method: "page.unarchive", release: page.package.version, input: target }],
        undefined,
        undefined,
    ]);
});

test.each(TEST_DIALECTS)(
    "undo and redo a party's mutations, restoring only what no one changed since, across a restart, on %s",
    async (dialect) => {
        const { connect, endpoint } = await serveNotes(dialect);
        const alice = connect("alice");
        const device = await Device.open("alice", endpoint("alice"));
        device.online();
        // create a note, then retitle it and write its text, both confirmed
        const created = device.client.mutate(note).create({ title: "Draft" });
        const { id } = await created.predicted;
        await created.confirmed;
        await device.client.mutate(note).update({ id, title: "Plan", text: "first" }).confirmed;
        const read = async (client: ObjectClient) => {
            const [row] = await client.database
                .select({
                    title: note.table.title,
                    text: note.table.text,
                    deletionRequestedAt: note.table.deletionRequestedAt,
                })
                .from(note.table)
                .where(eq(note.table.id, id));

            return row;
        };

        // undo the update, restoring only the untouched title
        await alice.note.update({ spaceId, id, requestId: RequestId.create(), text: "second" });
        await expect.poll(() => read(device.client)).toMatchObject({ text: "second" });
        await device.client.undo().confirmed;
        expect(await read(device.client)).toEqual({
            title: "Draft",
            text: "second",
            deletionRequestedAt: null,
        });

        // undo the creation into the trash and redo it
        const trashed = async () =>
            (await alice.note.list({ spaceId, deleted: "only" })).items.map((item) => item.id);
        await device.client.undo().confirmed;
        expect([await read(device.client), await trashed()]).toEqual([undefined, [id]]);
        await device.client.redo().confirmed;
        await expect.poll(() => read(device.client)).toMatchObject({ deletionRequestedAt: null });
        expect(await trashed()).toEqual([]);

        // keep the party's stack across a restart, and redo the update from it
        await device.offline();
        const restarted = await ObjectClient.open({
            database: await device.storage.connect(ObjectClient.tables({ notebook, note })),
            objects: { notebook, note },
            scope: spaceId,
            caller: principal.user.reference("universe", "alice"),
            endpoint: endpoint("alice"),
            reconnect: unmoved,
            origin: device.client.origin,
            isMigrated: true,
        });
        const stopping = new AbortController();
        const running = restarted.run(stopping.signal, (error) => device.errors.push(error));
        await restarted.redo().confirmed;
        expect(await read(restarted)).toEqual({
            title: "Plan",
            text: "second",
            deletionRequestedAt: null,
        });

        // leave nothing to redo, and undo both done entries by two quick undos, one entry each
        expect(await restarted.redo().predicted).toBe(false);
        const quick = [restarted.undo(), restarted.undo()];
        expect(await Promise.all(quick.map((undone) => undone.predicted))).toEqual([true, true]);
        await Promise.all(quick.map((undone) => undone.confirmed));
        expect(await restarted.undo().predicted).toBe(false);
        stopping.abort();
        await running;
        await restarted.close();
        expect(device.errors).toEqual([]);
    },
);

test("undo an update of an exact integer, restoring its value on the device and the server", async () => {
    // serve the tallies of a space
    const storage = await TestDatabase.create(
        "sqlite",
        defineDatabase({ name: "main", tables: [...tally.tables, journal] }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);
    const service = defineService("tallies", { objects: { tally } });
    const served = serveObjects(
        ObjectServer.serve(service, { callKey: testCallKey, database: storage.database }),
        spaceId,
    );

    // keep the space's tallies on a device that pushes and follows
    const local = await TestDatabase.create("sqlite", ObjectClient.tables({ tally }), {
        storage: "file",
    });
    onTestFinished(() => local.close());
    const client = await ObjectClient.open({
        database: local.database,
        objects: { tally },
        scope: spaceId,
        caller: principal.user.reference("universe", "alice"),
        endpoint: served.endpoint("alice"),
        reconnect: unmoved,
    });
    client.queryOf(tally).findMany().subscribe();
    const stopping = new AbortController();
    const errors: unknown[] = [];
    const loops = [
        client.follow(stopping.signal, (error) => errors.push(error)),
        client.push(stopping.signal, (error) => errors.push(error)),
    ];

    // create a tally beyond the exact range of numbers, then change its count and label
    const created = client.mutate(tally).create({ count: (2n ** 62n).toString(), label: "a" });
    const { id } = await created.predicted;
    await created.confirmed;
    await client.mutate(tally).update({ id, count: "-7", label: "b" }).confirmed;

    // undo the change on the device and through the server
    await client.undo().confirmed;
    const read = async (database: DatabaseConnection) =>
        database
            .select({ count: tally.table.count, label: tally.table.label })
            .from(tally.table)
            .where(eq(tally.table.id, id));
    stopping.abort();
    await Promise.all(loops);
    expect([await read(client.database), await read(storage.database), errors]).toEqual([
        [{ count: 2n ** 62n, label: "a" }],
        [{ count: 2n ** 62n, label: "a" }],
        [],
    ]);
    await client.close();
});

test("undo an update of a bytes field, restoring its bytes on the device and the server", async () => {
    // serve the stamps of a space
    const storage = await TestDatabase.create(
        "sqlite",
        defineDatabase({ name: "main", tables: [...stamp.tables, journal] }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);
    const service = defineService("stamps", { objects: { stamp } });
    const served = serveObjects(
        ObjectServer.serve(service, { callKey: testCallKey, database: storage.database }),
        spaceId,
    );

    // keep the space's stamps on a device that pushes and follows
    const local = await TestDatabase.create("sqlite", ObjectClient.tables({ stamp }), {
        storage: "file",
    });
    onTestFinished(() => local.close());
    const client = await ObjectClient.open({
        database: local.database,
        objects: { stamp },
        scope: spaceId,
        caller: principal.user.reference("universe", "alice"),
        endpoint: served.endpoint("alice"),
        reconnect: unmoved,
    });
    client.queryOf(stamp).findMany().subscribe();
    const stopping = new AbortController();
    const errors: unknown[] = [];
    const loops = [
        client.follow(stopping.signal, (error) => errors.push(error)),
        client.push(stopping.signal, (error) => errors.push(error)),
    ];

    // create a stamp of every byte value, then change its bytes and label
    const original = Uint8Array.from({ length: 256 }, (_, index) => index);
    const created = client.mutate(stamp).create({ bytes: original.toBase64(), label: "a" });
    const { id } = await created.predicted;
    await created.confirmed;
    await client.mutate(stamp).update({ id, bytes: Uint8Array.of(7).toBase64(), label: "b" })
        .confirmed;

    // undo the change on the device and through the server
    await client.undo().confirmed;
    const read = async (database: DatabaseConnection) =>
        database
            .select({ bytes: stamp.table.bytes, label: stamp.table.label })
            .from(stamp.table)
            .where(eq(stamp.table.id, id));
    stopping.abort();
    await Promise.all(loops);
    expect([await read(client.database), await read(storage.database), errors]).toEqual([
        [{ bytes: original, label: "a" }],
        [{ bytes: original, label: "a" }],
        [],
    ]);
    await client.close();
});
