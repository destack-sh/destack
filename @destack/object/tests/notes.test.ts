import { expect, test } from "@destack/test";
import { eq, type DatabaseConnection } from "@destack/db";
import { Condition } from "@destack/db/query";
import { relayHub, TEST_DIALECTS } from "@destack/db/test";
import { BrowserTab, ObjectClient, type BrowserHost } from "../src/client/index.ts";
import { serveDatabase, type Message } from "@destack/db/shared";
import { WasmClient } from "@destack/db/wasm";
import { identifier } from "@destack/schema";
import { RequestId } from "@destack/service/request";
import { note, notebook, notesJournal } from "./fixture/notes.ts";
import { principal } from "@destack/access";

import { Device, serveNotes, spaceId } from "./fixture/device.ts";
import { unmoved } from "./fixture/space.ts";

test.each(TEST_DIALECTS)(
    "keep notes private until shared, move them between notebooks and sync them on %s",
    async (dialect) => {
        const { connect } = await serveNotes(dialect);
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
            subject: principal.user.reference("global", "bob"),
        });
        const titles = async () =>
            (await bob.note.list({ spaceId })).items.map((item) => item.title).sort();
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
        ).rejects.toMatchObject({ code: "FORBIDDEN", message: "Forbidden" });

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
        ).rejects.toMatchObject({ code: "FORBIDDEN", message: "Forbidden" });

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
                subject: principal.user.reference("global", "bob"),
            }),
        ).rejects.toMatchObject({ code: "CONFLICT", message: "note is in the trash" });

        // list and withdraw the trashed note's relationships and proposals
        expect([
            await alice.note.relationships({ spaceId, id: loose.id }),
            await alice.note.proposals({ spaceId, id: loose.id }),
        ]).toEqual([
            { items: [], cursor: null },
            { items: [], cursor: null },
        ]);
        await alice.note.restore({ spaceId, id: loose.id, requestId: RequestId.create() });
        expect(await titles()).toEqual(["Ideas", "Sources"]);

        // follow the notes on bob's device and write into the shared notebook, predicted at once
        const device = await Device.open("bob", connect("bob"));
        device.online();
        await expect.poll(() => device.titles()).toEqual(["Ideas", "Sources"]);
        const questions = device.client
            .mutate(note)
            .create({ parentId: research.id, title: "Questions" });
        expect((await questions.predicted).owner).toBe("bob");
        expect(await device.titles()).toEqual(["Ideas", "Questions", "Sources"]);
        await questions.confirmed;
        expect((await alice.note.list({ spaceId })).items.map((item) => item.title).sort()).toEqual(
            ["Ideas", "Questions", "Sources"],
        );
        expect(device.errors).toEqual([]);
    },
);

test.each(TEST_DIALECTS)(
    "queue notes offline, write batches atomically, drop rejected predictions and rebase on %s",
    async (dialect) => {
        const { connect, database } = await serveNotes(dialect);
        const alice = connect("alice");

        // queue a notebook and its first note in one offline mutation
        const device = await Device.open("alice", alice);
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
        const restarted = await ObjectClient.open({
            database: await device.storage.connect(ObjectClient.tables([notebook, note])),
            objects: [notebook, note],
            scope: spaceId,
            caller: principal.user.reference("global", "alice"),
            service: alice.replica,
            reconnect: unmoved,
        });
        expect(await restarted.outbox.pending(restarted.database)).toEqual([
            {
                id: expect.any(String),
                calls: [
                    { method: "notebook.create", input: { name: "Travel", id: book.id, spaceId } },
                    {
                        method: "note.create",
                        input: {
                            parentId: book.id,
                            title: "Packing",
                            id: expect.any(String),
                            spaceId,
                        },
                    },
                ],
            },
        ]);
        await (restarted.database as DatabaseConnection & { close(): Promise<void> }).close();

        // execute the batch on the server once online, and confirm it through the replica
        device.online();
        await travel.confirmed;
        expect((await alice.notebook.get({ spaceId, id: book.id })).name).toBe("Travel");
        expect(await device.titles()).toEqual(["Packing"]);

        // rebase a predicted title onto a text change another device made meanwhile
        const [packing] = (await alice.note.list({ spaceId })).items;
        await device.offline();
        const retitled = device.client.mutate(note).update({ id: packing!.id, title: "Luggage" });
        await retitled.predicted;
        await alice.note.update({
            spaceId,
            id: packing!.id,
            requestId: RequestId.create(),
            text: "passport, charger",
        });
        device.follow();
        const local = async () => {
            const [row] = await device.client.database.select().from(note.table);

            return [row!.title, row!.text];
        };
        await expect.poll(local).toEqual(["Luggage", "passport, charger"]);

        // push the new title, keeping the other device's text
        device.push();
        await retitled.confirmed;
        expect(await local()).toEqual(["Luggage", "passport, charger"]);
        const stored = await alice.note.get({ spaceId, id: packing!.id });
        expect([stored.title, stored.text]).toEqual(["Luggage", "passport, charger"]);

        // refuse a batch the server forbids as a whole, and revert both of its predictions
        const bob = await Device.open("bob", connect("bob"));
        await alice.notebook.grant({
            spaceId,
            id: book.id,
            requestId: RequestId.create(),
            relation: "viewer",
            subject: principal.user.reference("global", "bob"),
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

        // push again a note the server confirmed, then lost when its database was restored
        await device.offline();
        device.push();
        const lost = device.client.mutate(note).create({ parentId: book.id, title: "Tickets" });
        const { id: lostId } = await lost.predicted;
        await expect
            .poll(async () => await device.client.outbox.pending(device.client.database))
            .toEqual([]);
        await database.delete(note.table).where(eq(note.table.id, lostId));
        await database.delete(notesJournal);
        await database.log.renew();
        device.follow();
        await lost.confirmed;
        expect((await alice.note.get({ spaceId, id: lostId })).title).toBe("Tickets");
        expect(await device.titles()).toEqual(["Luggage", "Tickets"]);
        expect(device.errors).toEqual([]);
    },
);

test.each(TEST_DIALECTS)(
    "hold only the queries a device subscribes to, with their includes, as rows move between them on %s",
    async (dialect) => {
        const { connect } = await serveNotes(dialect);
        const alice = connect("alice");

        // file notes into two notebooks
        const notebookNamed = (name: string) =>
            alice.notebook.create({ spaceId, requestId: RequestId.create(), name });
        const noteIn = (parentId: string, title: string) =>
            alice.note.create({
                spaceId,
                requestId: RequestId.create(),
                parentId: identifier("notebook").parse(parentId),
                title,
            });
        const travel = await notebookNamed("Travel");
        const work = await notebookNamed("Work");
        await noteIn(travel.id, "Packing");
        const plan = await noteIn(work.id, "Plan");

        // hold the travel notes and the notebook they belong to, nothing of work
        const device = await Device.open("alice", alice, []);
        const travelNotes = device.client.subscribe(note, {
            where: Condition.eq("parentId", travel.id),
            include: { parent: {} },
        });
        device.online();
        await travelNotes.ready;
        const books = async () =>
            (
                await device.client.database
                    .select({ name: notebook.table.name })
                    .from(notebook.table)
            )
                .map((row) => row.name)
                .sort();
        expect([await device.titles(), await books()]).toEqual([["Packing"], ["Travel"]]);

        // take a note in once it moves into travel, and let it go once it moves out
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

        // add the work notebook with its notes, then stop following travel
        const workBook = device.client.subscribe(notebook, {
            where: Condition.eq("name", "Work"),
            include: { notes: {} },
        });
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
        const { connect } = await serveNotes(dialect);
        const alice = connect("alice");
        const travel = await alice.notebook.create({
            spaceId,
            requestId: RequestId.create(),
            name: "Travel",
        });
        expect(travel.noteCount).toBe(0);

        // predict the count of a note created offline, before the server holds it
        const device = await Device.open("alice", alice);
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
        const count = async () =>
            (
                await device.client.database
                    .select({ count: notebook.table.noteCount })
                    .from(notebook.table)
            )[0]!.count;
        expect([
            await count(),
            (await alice.notebook.get({ spaceId, id: travel.id })).noteCount,
        ]).toEqual([1, 0]);

        // hold the server's count once it executes the note, and leave notes in the trash out
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
    const { connect } = await serveNotes("sqlite");
    const alice = connect("alice");

    // share one relay, worker and lock queue between tabs
    const join = relayHub<Message>();
    const database = await WasmClient.memory();
    const locks = new Map<string, Promise<void>>();
    const host = (): BrowserHost => ({
        relay: join(),
        request: (name, hold, signal) =>
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

                // queue behind the lock's holder, holding it unless the wait aborted
                const previous = locks.get(name) ?? Promise.resolve();
                const next = previous.then(async () => {
                    if (!signal?.aborted) {
                        isGranted = true;
                        await hold();
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
            objects: [notebook, note],
            scope: spaceId,
            caller: principal.user.reference("global", "alice"),
            service: alice.replica,
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
    await second.client.subscribe(notebook).ready;
    await second.client.subscribe(note).ready;

    // write through the second tab
    const travel = second.client.mutate(notebook).create({ name: "Travel" });
    const { id } = await travel.predicted;
    await travel.confirmed;
    expect((await alice.notebook.get({ spaceId, id })).name).toBe("Travel");

    // forget a tab's subscriptions once it closes and frees its presence lock
    const third = await open();
    await third.ready;
    await third.client.subscribe(note).ready;
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
