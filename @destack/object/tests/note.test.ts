import { expect, test } from "@destack/test";
import { journal } from "@destack/audit";
import { eq } from "@destack/db";
import { channelHub } from "@destack/db";
import { TEST_DIALECTS } from "@destack/db/test";
import { BrowserTab, type BrowserHost } from "../src/browser/index.ts";
import { ObjectClient } from "../src/client/index.ts";
import { serveDatabase, WasmClient, type Message } from "@destack/db/browser";
import { schema, present } from "@destack/schema";
import { RequestId } from "@destack/service/request";
import { note, notebook } from "./fixture/note.ts";
import { principal } from "@destack/access";
import { Subject } from "@destack/sync";

import { Device, serveNotes, spaceId } from "./fixture/device.ts";
import { unmoved } from "./fixture/space.ts";
import { any } from "./fixture/match.ts";

test.each(TEST_DIALECTS)(
    "keep notes private until shared, move them between notebooks and sync them on %s",
    async (dialect) => {
        const { connect, endpoint } = await serveNotes(dialect);
        const alice = connect("alice");
        const bob = connect("bob");

        // keep a notebook's notes and a loose note private to their owner
        const research = await alice.notebook.create({
            spaceId,
            requestId: RequestId.create(),
            name: "Research",
        });
        const filed = await alice.note.create({
            spaceId,
            requestId: RequestId.create(),
            parentId: research.id,
            title: "Sources",
        });
        const loose = await alice.note.create({
            spaceId,
            requestId: RequestId.create(),
            title: "Ideas",
        });
        expect((await bob.note.list({ spaceId })).items).toEqual([]);

        // share the notebook: its notes follow, loose notes stay private
        await alice.notebook.grant({
            spaceId,
            id: research.id,
            requestId: RequestId.create(),
            relation: "editor",
            subject: principal.user.reference("universe", "bob"),
        });
        const titles = async () =>
            (await bob.note.list({ spaceId })).items.map((item) => item.title).toSorted();
        expect(await titles()).toEqual(["Sources"]);
        const edited = await bob.note.update({
            spaceId,
            id: filed.id,
            requestId: RequestId.create(),
            revision: filed.revision,
            text: "Three papers",
        });
        expect(edited.text).toBe("Three papers");
        await expect(
            bob.note.delete({ spaceId, id: filed.id, requestId: RequestId.create(), revision: 2 }),
        ).rejects.toMatchObject({ code: "FORBIDDEN", message: "permission denied: manage" });

        // move a note only as its manager into an editable notebook
        await alice.note.move({
            spaceId,
            id: loose.id,
            requestId: RequestId.create(),
            revision: loose.revision,
            parentId: research.id,
        });
        expect(await titles()).toEqual(["Ideas", "Sources"]);
        await expect(
            bob.note.move({
                spaceId,
                id: loose.id,
                requestId: RequestId.create(),
                revision: 2,
                parentId: null,
            }),
        ).rejects.toMatchObject({ code: "FORBIDDEN", message: "permission denied: manage" });

        // keep deleted notes restorable from the trash, and refuse changing them there
        await alice.note.delete({
            spaceId,
            id: loose.id,
            requestId: RequestId.create(),
            revision: 2,
        });
        expect(await titles()).toEqual(["Sources"]);
        expect(
            (await alice.note.list({ spaceId, deleted: "only" })).items.map((item) => item.title),
        ).toEqual(["Ideas"]);
        await expect(
            alice.note.update({
                spaceId,
                id: loose.id,
                requestId: RequestId.create(),
                text: "Late",
            }),
        ).rejects.toMatchObject({ code: "CONFLICT", message: "note is in the trash" });
        await expect(
            alice.note.grant({
                spaceId,
                id: loose.id,
                requestId: RequestId.create(),
                relation: "viewer",
                subject: principal.user.reference("universe", "bob"),
            }),
        ).rejects.toMatchObject({ code: "CONFLICT", message: "note is in the trash" });

        // list and withdraw the trashed note's relationships and invitations
        expect([
            await alice.note.relationships({ spaceId, id: loose.id }),
            await alice.note.invitations({ spaceId, id: loose.id }),
        ]).toEqual([
            { items: [], cursor: null },
            { items: [], cursor: null },
        ]);
        await alice.note.restore({ spaceId, id: loose.id, requestId: RequestId.create() });
        expect(await titles()).toEqual(["Ideas", "Sources"]);

        // follow the notes on bob's device and write into the shared notebook, predicted at once
        const device = await Device.open("bob", endpoint("bob"));
        device.online();
        await expect.poll(() => device.titles()).toEqual(["Ideas", "Sources"]);
        const questions = device.client
            .mutate(note)
            .create({ parentId: research.id, title: "Questions" });
        expect(Subject.read((await questions.predicted).owner)).toEqual(
            principal.user.reference("universe", "bob"),
        );
        expect(await device.titles()).toEqual(["Ideas", "Questions", "Sources"]);
        await questions.confirmed;
        expect(
            (await alice.note.list({ spaceId })).items.map((item) => item.title).toSorted(),
        ).toEqual(["Ideas", "Questions", "Sources"]);
        expect(device.errors).toEqual([]);
    },
);

test.each(TEST_DIALECTS)(
    "queue notes offline, write batches atomically, drop rejected predictions and rebase on %s",
    async (dialect) => {
        const { connect, endpoint, database } = await serveNotes(dialect);
        const alice = connect("alice");

        // queue a notebook and its first note in one offline mutation
        const device = await Device.open("alice", endpoint("alice"));
        const travel = device.client.mutation(async (mutation) => {
            const book = await mutation.call(notebook).create({ name: "Travel" });
            await mutation.call(note).create({ parentId: book.id, title: "Packing" });

            return book;
        });
        const book = await travel.predicted;
        expect(await device.titles()).toEqual(["Packing"]);
        expect((await alice.notebook.list({ spaceId })).items).toEqual([]);

        // keep the queue and the prediction when the device restarts
        await device.offline();
        const reconnected = await device.storage.connect(ObjectClient.tables({ notebook, note }));
        const restarted = await ObjectClient.open({
            database: reconnected,
            objects: { notebook, note },
            scope: spaceId,
            caller: principal.user.reference("universe", "alice"),
            endpoint: endpoint("alice"),
            reconnect: unmoved,
        });
        expect(await restarted.prediction.pending(restarted.database)).toEqual([
            {
                id: any(String),
                calls: [
                    {
                        method: "notebook.create",
                        release: notebook.package.version,
                        input: { name: "Travel", id: book.id, spaceId },
                    },
                    {
                        method: "note.create",
                        release: note.package.version,
                        input: {
                            parentId: book.id,
                            title: "Packing",
                            id: any(String),
                            spaceId,
                        },
                    },
                ],
            },
        ]);
        await reconnected.close();

        // execute the batch on the server once online, and confirm it through the replica
        device.online();
        await travel.confirmed;
        expect((await alice.notebook.get({ spaceId, id: book.id })).name).toBe("Travel");
        expect(await device.titles()).toEqual(["Packing"]);

        // rebase a predicted title onto a text change another device made meanwhile
        const [listed] = (await alice.note.list({ spaceId })).items;
        const packing = present(listed, "the packing note");
        await device.offline();
        const retitled = device.client.mutate(note).update({ id: packing.id, title: "Luggage" });
        await retitled.predicted;
        await alice.note.update({
            spaceId,
            id: packing.id,
            requestId: RequestId.create(),
            text: "passport, charger",
        });
        device.follow();
        const local = async () => {
            const [row] = await device.client.database.select().from(note.table);
            const { title, text } = present(row, "the local note");

            return [title, text];
        };
        await expect.poll(local).toEqual(["Luggage", "passport, charger"]);

        // push the new title and keep the other device's text
        device.push();
        await retitled.confirmed;
        expect(await local()).toEqual(["Luggage", "passport, charger"]);
        const stored = await alice.note.get({ spaceId, id: packing.id });
        expect([stored.title, stored.text]).toEqual(["Luggage", "passport, charger"]);

        // refuse a batch the server forbids as a whole, and revert both of its predictions
        const bob = await Device.open("bob", endpoint("bob"));
        await alice.notebook.grant({
            spaceId,
            id: book.id,
            requestId: RequestId.create(),
            relation: "viewer",
            subject: principal.user.reference("universe", "bob"),
        });
        bob.online();
        await expect.poll(() => bob.titles()).toEqual(["Luggage"]);
        const intrusion = bob.client.mutation(async (mutation) => {
            await mutation.call(notebook).create({ name: "Mine" });
            await mutation.call(note).create({ parentId: book.id, title: "Graffiti" });
        });
        await intrusion.predicted;
        await expect(intrusion.confirmed).rejects.toMatchObject({
            code: "FORBIDDEN",
            message: "permission denied: edit",
        });
        expect(await bob.titles()).toEqual(["Luggage"]);
        expect(
            (
                await bob.client.database.select({ name: notebook.table.name }).from(notebook.table)
            ).map((row) => row.name),
        ).toEqual(["Travel"]);
        expect((await alice.notebook.list({ spaceId })).items.map((item) => item.name)).toEqual([
            "Travel",
        ]);
        expect([device.errors, bob.errors]).toEqual([[], []]);

        // push again a note the server confirmed and lost when its database was restored
        await device.offline();
        device.push();
        const lost = device.client.mutate(note).create({ parentId: book.id, title: "Tickets" });
        const { id: lostId } = await lost.predicted;
        await expect
            .poll(async () => await device.client.prediction.pending(device.client.database))
            .toEqual([]);
        await database.delete(note.table).where(eq(note.table.id, lostId));
        await database.delete(journal);
        await database.log.renew();
        device.follow();
        await lost.confirmed;
        expect((await alice.note.get({ spaceId, id: lostId })).title).toBe("Tickets");
        expect(await device.titles()).toEqual(["Luggage", "Tickets"]);
        expect(device.errors).toEqual([]);
    },
);

test.each(TEST_DIALECTS)(
    "keep only the queries a device subscribes to, with their includes, as rows move between them on %s",
    async (dialect) => {
        const { connect, endpoint } = await serveNotes(dialect);
        const alice = connect("alice");

        // file notes into two notebooks
        const notebookNamed = (name: string) =>
            alice.notebook.create({ spaceId, requestId: RequestId.create(), name });
        const noteIn = (parentId: string, title: string) =>
            alice.note.create({
                spaceId,
                requestId: RequestId.create(),
                parentId: schema.identifier("notebook").parse(parentId),
                title,
            });
        const travel = await notebookNamed("Travel");
        const work = await notebookNamed("Work");
        await noteIn(travel.id, "Packing");
        const plan = await noteIn(work.id, "Plan");

        // keep the travel notes and the notebook they belong to, nothing of work
        const device = await Device.open("alice", endpoint("alice"), []);
        const travelNotes = device.client.query.note
            .findMany({ where: { parentId: travel.id }, with: { parent: true } })
            .subscribe();
        device.online();
        await travelNotes.ready;
        const books = async () =>
            (
                await device.client.database
                    .select({ name: notebook.table.name })
                    .from(notebook.table)
            )
                .map((row) => row.name)
                .toSorted();
        expect([await device.titles(), await books()]).toEqual([["Packing"], ["Travel"]]);

        // take a note in when it moves into travel and let it go when it moves out
        await alice.note.move({
            spaceId,
            id: plan.id,
            requestId: RequestId.create(),
            parentId: travel.id,
        });
        await expect.poll(() => device.titles()).toEqual(["Packing", "Plan"]);
        await alice.note.move({
            spaceId,
            id: plan.id,
            requestId: RequestId.create(),
            parentId: work.id,
        });
        await expect.poll(() => device.titles()).toEqual(["Packing"]);

        // add the work notebook with its notes and stop following travel
        const workBook = device.client.query.notebook
            .findMany({ where: { name: "Work" }, with: { notes: true } })
            .subscribe();
        await workBook.ready;
        expect([await device.titles(), await books()]).toEqual([
            ["Packing", "Plan"],
            ["Travel", "Work"],
        ]);
        await travelNotes.close();
        await expect.poll(() => device.titles()).toEqual(["Plan"]);
        expect(await books()).toEqual(["Work"]);

        // resume from the recorded queries after a restart, without starting over
        await device.offline();
        device.online();
        await expect.poll(() => device.titles()).toEqual(["Plan"]);
        expect(device.errors).toEqual([]);
    },
);

test.each(TEST_DIALECTS)(
    "count a notebook's notes outside the trash, predicted on a device before the server confirms on %s",
    async (dialect) => {
        const { connect, endpoint } = await serveNotes(dialect);
        const alice = connect("alice");
        const travel = await alice.notebook.create({
            spaceId,
            requestId: RequestId.create(),
            name: "Travel",
        });
        expect(travel.noteCount).toBe(0);

        // predict the count of a note created offline, before the server has it
        const device = await Device.open("alice", endpoint("alice"));
        device.follow();
        await expect
            .poll(async () =>
                device.client.database.select({ id: notebook.table.id }).from(notebook.table),
            )
            .toEqual([{ id: travel.id }]);
        const created = device.client
            .mutate(note)
            .create({ parentId: travel.id, title: "Packing" });
        const { id } = await created.predicted;
        const count = async () => {
            const [row] = await device.client.database
                .select({ count: notebook.table.noteCount })
                .from(notebook.table);

            return present(row, "the notebook's count").count;
        };
        expect([
            await count(),
            (await alice.notebook.get({ spaceId, id: travel.id })).noteCount,
        ]).toEqual([1, 0]);

        // keep the server's count after it executes the note, excluding trashed notes
        device.push();
        await created.confirmed;
        expect([
            await count(),
            (await alice.notebook.get({ spaceId, id: travel.id })).noteCount,
        ]).toEqual([1, 1]);
        await alice.note.delete({ spaceId, id, requestId: RequestId.create() });
        expect((await alice.notebook.get({ spaceId, id: travel.id })).noteCount).toBe(0);
        await expect.poll(count).toBe(0);
        expect(device.errors).toEqual([]);
    },
);

test("share one browser database between tabs, handing it over when the owning tab closes", async () => {
    const { connect, endpoint } = await serveNotes("sqlite");
    const alice = connect("alice");

    // share one channel, worker and lock queue between tabs
    const join = channelHub<Message>();
    const database = await WasmClient.memory();
    const locks = new Map<string, Promise<void>>();
    const host = (): BrowserHost => ({
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
    const open = () =>
        BrowserTab.open({
            name: "notes",
            objects: { notebook, note },
            scope: spaceId,
            caller: principal.user.reference("universe", "alice"),
            endpoint: endpoint("alice"),
            reconnect: unmoved,
            host: host(),
            report: (error) => {
                throw error;
            },
        });

    // own the database in the first tab, and share it with the second
    const first = await open();
    await first.ready;
    const second = await open();
    await second.ready;
    await second.client.query.notebook.findMany().subscribe().ready;
    await second.client.query.note.findMany().subscribe().ready;

    // write through the second tab
    const travel = second.client.mutate(notebook).create({ name: "Travel" });
    const { id } = await travel.predicted;
    await travel.confirmed;
    expect((await alice.notebook.get({ spaceId, id })).name).toBe("Travel");

    // forget a tab's subscriptions after it closes and frees its presence lock
    const third = await open();
    await third.ready;
    await third.client.query.note.findMany().subscribe().ready;
    const isPresent = async () => (await first.client.origins()).has(third.client.origin);
    const before = await isPresent();
    await third.close();
    await first.client.database.log.until(
        async () => !(await isPresent()),
        AbortSignal.timeout(5000),
    );
    expect([before, await isPresent()]).toEqual([true, false]);

    // hand the database over once the owning tab closes, and keep writing through the second
    await first.close();
    const packing = second.client.mutate(note).create({ parentId: id, title: "Packing" });
    await packing.confirmed;
    expect((await alice.note.list({ spaceId })).items.map((item) => item.title)).toEqual([
        "Packing",
    ]);
    await second.close();
});

test("decide checks on a device from its copy of the caller's access rows, and follow a grant and its revocation into them", async () => {
    const { connect, endpoint } = await serveNotes("sqlite");
    const alice = connect("alice");

    // keep a notebook alice owns, which bob's device cannot read yet
    const research = await alice.notebook.create({
        spaceId,
        requestId: RequestId.create(),
        name: "Research",
    });
    const device = await Device.open("bob", endpoint("bob"));
    device.online();
    const checks = async () => [
        await device.client.can(notebook, research.id, "read"),
        await device.client.can(notebook, research.id, "edit"),
        await device.client.can(notebook, research.id, "manage"),
    ];
    expect(await checks()).toEqual([false, false, false]);

    // let bob edit once the grant reaches the device, without managing
    const granted = await alice.notebook.grant({
        spaceId,
        id: research.id,
        requestId: RequestId.create(),
        relation: "editor",
        subject: principal.user.reference("universe", "bob"),
    });
    await expect.poll(checks).toEqual([true, true, false]);

    // take the access away again once the revocation reaches the device
    await alice.notebook.revoke({
        spaceId,
        id: research.id,
        requestId: RequestId.create(),
        relationshipId: granted.id,
    });
    await expect.poll(checks).toEqual([false, false, false]);
    expect(device.errors).toEqual([]);
});

test("show a notebook's relationships to everyone who reads it, and to no one else", async () => {
    const { connect } = await serveNotes("sqlite");
    const alice = connect("alice");
    const bob = connect("bob");
    const carol = connect("carol");
    const dave = connect("dave");

    // share alice's notebook with bob as editor and carol as viewer
    const research = await alice.notebook.create({
        spaceId,
        requestId: RequestId.create(),
        name: "Research",
    });
    for (const [user, relation] of [
        ["bob", "editor"],
        ["carol", "viewer"],
    ] as const) {
        await alice.notebook.grant({
            spaceId,
            id: research.id,
            requestId: RequestId.create(),
            relation,
            subject: principal.user.reference("universe", user),
        });
    }

    // list the same relationships for each reader, and refuse a stranger
    const listed = async (caller: typeof alice) =>
        (await caller.notebook.relationships({ spaceId, id: research.id })).items
            .map((item) => `${"relation" in item ? item.relation : item.role}:${item.subject.id}`)
            .toSorted();
    expect([await listed(alice), await listed(bob), await listed(carol)]).toEqual([
        ["editor:bob", "viewer:carol"],
        ["editor:bob", "viewer:carol"],
        ["editor:bob", "viewer:carol"],
    ]);
    await expect(dave.notebook.relationships({ spaceId, id: research.id })).rejects.toMatchObject({
        code: "NOT_FOUND",
    });
});
