import { users, userName } from "./fixture/user.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { journal } from "../src/index.ts";
import { eq, channelHub } from "@destack/db";
import { TEST_DIALECTS } from "@destack/db/test";
import { BrowserTab, type BrowserHost } from "../src/browser/index.ts";
import { ObjectClient } from "../src/client/index.ts";
import { serveDatabase, WasmClient, type Message } from "@destack/db/browser";
import { schema, present } from "@destack/schema";
import { RequestId, ObjectReference, type Page } from "@destack/sync";
import { getEventMeta } from "@destack/object";
import { note, notebook } from "./fixture/note.ts";
import { principal } from "@destack/access";

import { Device, serveNotes, spaceId } from "./fixture/device.ts";
import { unmoved } from "./fixture/space.ts";
import { Clock } from "@destack/environment";

test.each(TEST_DIALECTS)(
    "hold notes private until shared, move them between notebooks and sync them on %s",
    async (dialect) => {
        const { connect, endpoint } = await serveNotes(dialect);
        const alice = connect(users.alice);
        const bob = connect(users.bob);

        // hold a notebook's notes and a loose note private to their owner
        const research = await alice.notebook.create({
            spaceId,
            requestId: RequestId.create(Date.now()),
            name: "Research",
        });
        const filed = await alice.note.create({
            spaceId,
            requestId: RequestId.create(Date.now()),
            parentId: research.id,
            title: "Sources",
        });
        const loose = await alice.note.create({
            spaceId,
            requestId: RequestId.create(Date.now()),
            title: "Ideas",
        });
        expect((await bob.note.list({ spaceId })).items).toEqual([]);

        // refuse a grant through both a relation and a role
        await expect(
            alice.notebook.grant({
                spaceId,
                id: research.id,
                requestId: RequestId.create(Date.now()),
                relation: "editor",
                role: "reader",
                subject: principal.user.reference("universe", users.bob),
            }),
        ).rejects.toMatchObject({
            code: "BAD_REQUEST",
            message: "a grant has exactly one relation or role",
        });

        // share the notebook: its notes follow, loose notes stay private
        await alice.notebook.grant({
            spaceId,
            id: research.id,
            requestId: RequestId.create(Date.now()),
            relation: "editor",
            subject: principal.user.reference("universe", users.bob),
        });
        const titles = async () =>
            (await bob.note.list({ spaceId })).items.map((item) => item.title).toSorted();
        expect(await titles()).toEqual(["Sources"]);
        const edited = await bob.note.update({
            spaceId,
            id: filed.id,
            requestId: RequestId.create(Date.now()),
            revision: filed.revision,
            text: "Three papers",
        });
        expect(edited.text).toBe("Three papers");
        await expect(
            bob.note.delete({
                spaceId,
                id: filed.id,
                requestId: RequestId.create(Date.now()),
                revision: 2,
            }),
        ).rejects.toMatchObject({ code: "FORBIDDEN", message: "permission denied: manage" });

        // move a note only as its manager into an editable notebook
        await alice.note.move({
            spaceId,
            id: loose.id,
            requestId: RequestId.create(Date.now()),
            revision: loose.revision,
            parentId: research.id,
        });
        expect(await titles()).toEqual(["Ideas", "Sources"]);
        await expect(
            bob.note.move({
                spaceId,
                id: loose.id,
                requestId: RequestId.create(Date.now()),
                revision: 2,
                parentId: null,
            }),
        ).rejects.toMatchObject({ code: "FORBIDDEN", message: "permission denied: manage" });

        // hold deleted notes restorable from the trash, and refuse changing them there
        await alice.note.delete({
            spaceId,
            id: loose.id,
            requestId: RequestId.create(Date.now()),
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
                requestId: RequestId.create(Date.now()),
                text: "Late",
            }),
        ).rejects.toMatchObject({ code: "CONFLICT", message: "note is in the trash" });
        await expect(
            alice.note.grant({
                spaceId,
                id: loose.id,
                requestId: RequestId.create(Date.now()),
                relation: "viewer",
                subject: principal.user.reference("universe", users.bob),
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
        await alice.note.restore({
            spaceId,
            id: loose.id,
            requestId: RequestId.create(Date.now()),
        });
        expect(await titles()).toEqual(["Ideas", "Sources"]);

        // follow the notes on bob's device and write into the shared notebook, predicted at once
        const device = await Device.open(users.bob, endpoint(users.bob));
        device.online();
        await expect.poll(() => device.titles()).toEqual(["Ideas", "Sources"]);
        const questions = device.client
            .mutate(note)
            .create({ parentId: research.id, title: "Questions" });
        expect(ObjectReference.at("owner", await questions.predicted)).toEqual(
            principal.user.reference("universe", users.bob),
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
        const alice = connect(users.alice);

        // queue a notebook and its first note in one offline mutation
        const device = await Device.open(users.alice, endpoint(users.alice));
        const travel = device.client.mutation(async (mutation) => {
            const book = await mutation.call(notebook).create({ name: "Travel" });
            const first = await mutation.call(note).create({ parentId: book.id, title: "Packing" });

            return { book, first };
        });
        const { book, first } = await travel.predicted;
        expect(await device.titles()).toEqual(["Packing"]);
        expect((await alice.notebook.list({ spaceId })).items).toEqual([]);

        // retain the queue and the prediction when the device restarts
        const queued = await device.client.prediction.pending(device.client.database);
        await device.offline();
        const reconnected = await device.storage.connect(ObjectClient.tables({ notebook, note }));
        const restarted = await ObjectClient.open({
            clock: Clock.system,
            database: reconnected,
            objects: { notebook, note },
            scope: spaceId,
            caller: principal.user.reference("universe", users.alice),
            endpoint: endpoint(users.alice),
            reconnect: unmoved,
        });
        expect(await restarted.prediction.pending(restarted.database)).toEqual(queued);
        expect(queued.map((mutation) => mutation.calls)).toEqual([
            [
                {
                    type: notebook.typeReference,
                    method: "create",
                    release: notebook.package.version,
                    input: { name: "Travel", id: book.id, spaceId },
                },
                {
                    type: note.typeReference,
                    method: "create",
                    release: note.package.version,
                    input: {
                        parentId: book.id,
                        title: "Packing",
                        id: first.id,
                        spaceId,
                    },
                },
            ],
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
            requestId: RequestId.create(Date.now()),
            text: "passport, charger",
        });
        device.follow();
        const local = async () => {
            const [row] = await device.client.database.select().from(note.table);
            const { title, text } = present(row, "the local note");

            return [title, text];
        };
        await expect.poll(local).toEqual(["Luggage", "passport, charger"]);

        // push the new title beside the other device's text
        device.push();
        await retitled.confirmed;
        expect(await local()).toEqual(["Luggage", "passport, charger"]);
        const stored = await alice.note.get({ spaceId, id: packing.id });
        expect([stored.title, stored.text]).toEqual(["Luggage", "passport, charger"]);

        // refuse a batch the server forbids as a whole, and revert both of its predictions
        const bob = await Device.open(users.bob, endpoint(users.bob));
        await alice.notebook.grant({
            spaceId,
            id: book.id,
            requestId: RequestId.create(Date.now()),
            relation: "viewer",
            subject: principal.user.reference("universe", users.bob),
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
            message: "permission denied: write",
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
    "replicate only the queries a device subscribes to, with their includes, as rows move between them on %s",
    async (dialect) => {
        const { connect, endpoint } = await serveNotes(dialect);
        const alice = connect(users.alice);

        // file notes into two notebooks
        const notebookNamed = (name: string) =>
            alice.notebook.create({ spaceId, requestId: RequestId.create(Date.now()), name });
        const noteIn = (parentId: string, title: string) =>
            alice.note.create({
                spaceId,
                requestId: RequestId.create(Date.now()),
                parentId: schema.identifier("notebook").parse(parentId),
                title,
            });
        const travel = await notebookNamed("Travel");
        const work = await notebookNamed("Work");
        await noteIn(travel.id, "Packing");
        const plan = await noteIn(work.id, "Plan");

        // replicate the travel notes and the notebook they belong to, nothing of work
        const device = await Device.open(users.alice, endpoint(users.alice), []);
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
            requestId: RequestId.create(Date.now()),
            parentId: travel.id,
        });
        await expect.poll(() => device.titles()).toEqual(["Packing", "Plan"]);
        await alice.note.move({
            spaceId,
            id: plan.id,
            requestId: RequestId.create(Date.now()),
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
        const alice = connect(users.alice);
        const travel = await alice.notebook.create({
            spaceId,
            requestId: RequestId.create(Date.now()),
            name: "Travel",
        });
        expect(travel.noteCount).toBe(0);

        // predict the count of a note created offline, before the server has it
        const device = await Device.open(users.alice, endpoint(users.alice));
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

        // take the server's count after it executes the note, excluding trashed notes
        device.push();
        await created.confirmed;
        expect([
            await count(),
            (await alice.notebook.get({ spaceId, id: travel.id })).noteCount,
        ]).toEqual([1, 1]);
        await alice.note.delete({ spaceId, id, requestId: RequestId.create(Date.now()) });
        expect((await alice.notebook.get({ spaceId, id: travel.id })).noteCount).toBe(0);
        await expect.poll(count).toBe(0);
        expect(device.errors).toEqual([]);
    },
);

test("share one browser database between tabs, handing it over when the owning tab closes", async () => {
    const { connect, endpoint } = await serveNotes("sqlite");
    const alice = connect(users.alice);

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
            clock: Clock.system,
            name: "notes",
            objects: { notebook, note },
            scope: spaceId,
            caller: principal.user.reference("universe", users.alice),
            device: principal.device.reference("universe", "device-browser"),
            endpoint: endpoint(users.alice),
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

    // hand the database over once the owning tab closes, and go on writing through the second
    await first.close();
    const packing = second.client.mutate(note).create({ parentId: id, title: "Packing" });
    await packing.confirmed;
    expect((await alice.note.list({ spaceId })).items.map((item) => item.title)).toEqual([
        "Packing",
    ]);
    await second.close();
});

test("decide checks on a device from its replica of the caller's access rows, and follow a grant and its revocation into them", async () => {
    const { connect, endpoint } = await serveNotes("sqlite");
    const alice = connect(users.alice);

    // create a notebook alice owns, which bob's device cannot read yet
    const research = await alice.notebook.create({
        spaceId,
        requestId: RequestId.create(Date.now()),
        name: "Research",
    });
    const device = await Device.open(users.bob, endpoint(users.bob));
    device.online();
    const checks = async () => [
        await device.client.check(notebook, research.id, "read"),
        await device.client.check(notebook, research.id, "write"),
        await device.client.check(notebook, research.id, "manage"),
    ];
    expect(await checks()).toEqual([false, false, false]);

    // let bob edit once the grant reaches the device, without managing
    const granted = await alice.notebook.grant({
        spaceId,
        id: research.id,
        requestId: RequestId.create(Date.now()),
        relation: "editor",
        subject: principal.user.reference("universe", users.bob),
    });
    await expect.poll(checks).toEqual([true, true, false]);

    // take the access away again once the revocation reaches the device
    await alice.notebook.revoke({
        spaceId,
        id: research.id,
        requestId: RequestId.create(Date.now()),
        relationshipId: granted.id,
    });
    await expect.poll(checks).toEqual([false, false, false]);
    expect(device.errors).toEqual([]);
});

test("decide checks on a device as the server decides them, for a group its authenticator vouches for", async () => {
    const team = { ...principal.group.reference("universe", "team"), relation: "member" };
    const { connect, endpoint } = await serveNotes(
        "sqlite",
        Clock.system,
        new Map([[users.bob, [team]]]),
    );
    const alice = connect(users.alice);
    const bob = connect(users.bob);

    // share alice's notebook with the team bob is vouched into
    const research = await alice.notebook.create({
        spaceId,
        requestId: RequestId.create(Date.now()),
        name: "Research",
    });
    await alice.notebook.grant({
        spaceId,
        id: research.id,
        requestId: RequestId.create(Date.now()),
        relation: "editor",
        subject: team,
    });

    // allow on the device exactly what the server allows bob
    const device = await Device.open(users.bob, endpoint(users.bob));
    device.online();
    const checks = async () => [
        await device.client.check(notebook, research.id, "read"),
        await device.client.check(notebook, research.id, "write"),
        await device.client.check(notebook, research.id, "manage"),
    ];
    expect((await bob.notebook.get({ spaceId, id: research.id })).name).toBe("Research");
    await expect.poll(checks).toEqual([true, true, false]);
    expect(device.errors).toEqual([]);
});

test("resume a store stream from the last event it sent, at that page's position", async () => {
    const { connect, endpoint } = await serveNotes("sqlite");
    const alice = connect(users.alice);
    await alice.notebook.create({
        spaceId,
        requestId: RequestId.create(Date.now()),
        name: "Research",
    });
    const store = ObjectClient.store(endpoint(users.alice));
    const subscription = {
        name: "access",
        publication: "access",
        scope: spaceId,
        target: spaceId,
        parameters: {},
    };

    // read a stream's first complete page and the event identifier it came with
    const complete = async (lastEventId?: string) => {
        const controller = new AbortController();
        const options = lastEventId === undefined ? {} : { lastEventId };
        const pages = await store.stream(subscription, { signal: controller.signal, ...options });
        for await (const page of pages) {
            if (page.complete) {
                controller.abort();

                return { reset: page.reset, position: page.position, id: getEventMeta(page)?.id };
            }
        }
        throw new TypeError("the stream ended before a complete page");
    };

    // resume a second stream from the first one's event, at its position without starting over
    const first = await complete();
    const resumed = await complete(first.id);
    const { log, epoch, sequence } = first.position;
    expect([first, resumed]).toEqual([
        { reset: true, position: first.position, id: `${log}/${epoch}/${sequence}` },
        { reset: false, position: first.position, id: first.id },
    ]);
});

test("restart a caller's access stream only on access changes concerning the caller", async () => {
    const { connect, endpoint } = await serveNotes("sqlite");
    const alice = connect(users.alice);
    const research = await alice.notebook.create({
        spaceId,
        requestId: RequestId.create(Date.now()),
        name: "Research",
    });

    // follow bob's access rows, keeping every page
    const controller = new AbortController();
    onTestFinished(() => controller.abort());
    const pages: Page[] = [];
    const store = ObjectClient.store(endpoint(users.bob));
    const subscription = {
        name: "access",
        publication: "access",
        scope: spaceId,
        target: spaceId,
        parameters: {},
    };
    const errors: unknown[] = [];
    const following = (async () => {
        for await (const page of await store.stream(subscription, { signal: controller.signal })) {
            pages.push(page);
        }
    })().catch((error: unknown) => {
        // keep failures other than the closing abort
        if (!controller.signal.aborted) {
            errors.push(error);
        }
    });
    await expect.poll(() => pages.some((page) => page.complete)).toBe(true);

    // share the notebook with carol, then with bob
    const share = async (user: (typeof users)[keyof typeof users]) =>
        await alice.notebook.grant({
            spaceId,
            id: research.id,
            requestId: RequestId.create(Date.now()),
            relation: "viewer",
            subject: principal.user.reference("universe", user),
        });
    await share(users.carol);

    // have bob's stream confirm the position past carol's share, so a restart for it would show first
    const watermark = await ObjectClient.store(endpoint(users.alice)).watermark({
        scope: spaceId,
        homes: [],
    });
    await store.confirm({ subscription, watermark });
    const isPast = () =>
        pages.some(
            (page) =>
                page.complete &&
                page.position.log === watermark.log &&
                page.position.sequence >= watermark.sequence,
        );
    await expect.poll(isPast).toBe(true);
    const resetsPastCarol = pages.filter((page) => page.reset).length;
    const shared = await share(users.bob);

    // start one snapshot for bob's own share alone, none for carol's
    const isShared = () =>
        pages.some((page) => page.changes.some((change) => change.row["id"] === shared.id));
    await expect.poll(isShared).toBe(true);
    expect([resetsPastCarol, pages.filter((page) => page.reset).length]).toEqual([1, 2]);
    controller.abort();
    await following;
    expect(errors).toEqual([]);
});

test("show a notebook's relationships to everyone who reads it, and to no one else", async () => {
    const { connect } = await serveNotes("sqlite");
    const alice = connect(users.alice);
    const bob = connect(users.bob);
    const carol = connect(users.carol);
    const dave = connect(users.dave);

    // share alice's notebook with bob as editor and carol as viewer
    const research = await alice.notebook.create({
        spaceId,
        requestId: RequestId.create(Date.now()),
        name: "Research",
    });
    for (const [user, relation] of [
        [users.bob, "editor"],
        [users.carol, "viewer"],
    ] as const) {
        await alice.notebook.grant({
            spaceId,
            id: research.id,
            requestId: RequestId.create(Date.now()),
            relation,
            subject: principal.user.reference("universe", user),
        });
    }

    // list the same relationships for each reader, and refuse a stranger
    const listed = async (caller: typeof alice) =>
        (await caller.notebook.relationships({ spaceId, id: research.id })).items
            .map(
                (item) =>
                    `${"relation" in item ? item.relation : item.role}:${userName(item.subject.id)}`,
            )
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
