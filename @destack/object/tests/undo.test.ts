import { expect, test } from "@destack/test";
import { eq } from "@destack/db";
import { TEST_DIALECTS } from "@destack/db/test";
import { RequestId } from "@destack/service/request";
import { ObjectClient } from "../src/client/index.ts";
import { defineObject, field, method, type Method, type Step } from "../src/index.ts";
import { Device, serveNotes, spaceId } from "./fixture/device.ts";
import { note, notebook } from "./fixture/notes.ts";
import { space, unmoved } from "./fixture/space.ts";
import { principal } from "@destack/access";

/** Folders holding pages. */
const folder = defineObject({
    name: "folder",
    plural: "folders",
    scope: space,
    fields: { name: field.string() },
    permissions: ["write"],
    methods: { create: method.create("write") },
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
    methods: {
        create: method.create("write"),
        update: method.update("write"),
        archive: method({ permission: "write", inverse: "unarchive" }),
        unarchive: method({ permission: "write" }),
        announce: method({ permission: "write" }),
    },
});

test("invert each trait's methods into the calls that undo them, and leave the rest undoable by nothing", () => {
    // invert a call of the page type
    const target = { spaceId: "space-1", id: "page-1" };
    const invert = (name: string, step: Omit<Step, "object" | "name">) =>
        (page.methods as Readonly<Record<string, Method>>)[name]!.inverse?.({
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
        [{ method: "page.delete", input: target }],
        [{ method: "page.restore", input: target }],
        [{ method: "page.delete", input: target }],
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
        [{ method: "page.update", input: { ...target, title: "A" } }],
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
        [{ method: "page.move", input: { ...target, parentId: "folder-1" } }],
        [{ method: "page.retract", input: target }],
        [{ method: "page.publish", input: target }],
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
        [{ method: "page.revoke", input: { ...target, relationshipId: "relationship-1" } }],
        [{ method: "page.unarchive", input: target }],
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
            database: await device.storage.connect(ObjectClient.tables([notebook, note])),
            objects: [notebook, note],
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
