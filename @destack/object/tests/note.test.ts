import { FOLLOWER_REVISION } from "@destack/sync/test";
import { testSimulation } from "../src/test/index.ts";
import { users, userName } from "./fixture/user.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { TestDatabase } from "@destack/db/test";
import { journal, ViewSubscriber } from "../src/index.ts";
import { eq, Snapshot } from "@destack/db";
import { BrowserTab, type BrowserTabWorker } from "../src/browser/index.ts";
import { BrowserSimulation } from "@destack/environment/browser/simulation";
import { ObjectClient, RelationalQuery } from "../src/client/index.ts";
import { broadcastChannel, serveDatabase, WasmClient } from "@destack/db/browser";
import { schema, present } from "@destack/schema";
import { RequestId, ObjectReference, type Page } from "@destack/sync";
import { getEventMeta } from "@destack/object";
import { note, notebook } from "./fixture/note.ts";
import { principal } from "@destack/access";

import { serveNotes, spaceId } from "./fixture/device.ts";
import { unmoved } from "./fixture/space.ts";

/** The feed's heartbeat, after which a caught-up stream publishes its position: 10 s. */
const HEARTBEAT_MILLISECONDS = 10_000;

test("store notes private until shared, move them between notebooks and sync them", async () => {
    const { connect, openDevice, environment } = await serveNotes();
    const alice = connect(users.alice);
    const bob = connect(users.bob);

    // store a notebook's notes and a loose note private to their owner
    const research = await alice.notebook.create({
        spaceId,
        requestId: RequestId.create(environment),
        name: "Research",
    });
    const filed = await alice.note.create({
        spaceId,
        requestId: RequestId.create(environment),
        parentId: research.id,
        title: "Sources",
    });
    const loose = await alice.note.create({
        spaceId,
        requestId: RequestId.create(environment),
        title: "Ideas",
    });
    expect((await bob.note.list({ spaceId })).items).toEqual([]);

    // refuse a grant through both a relation and a role
    expect(
        await refusal(
            alice.notebook.grant({
                spaceId,
                id: research.id,
                requestId: RequestId.create(environment),
                relation: "editor",
                role: "role-01996ab0-0000-7000-8000-0000000000a8",
                subject: principal.user.reference("universe", users.bob),
            }),
        ),
    ).toEqual([
        "BAD_REQUEST",
        "CALL_INVALID",
        "grant is invalid: it must have exactly one relation or role",
    ]);

    // share the notebook: its notes follow, loose notes stay private
    await alice.notebook.grant({
        spaceId,
        id: research.id,
        requestId: RequestId.create(environment),
        relation: "editor",
        subject: principal.user.reference("universe", users.bob),
    });
    const titles = async () =>
        (await bob.note.list({ spaceId })).items.map((item) => item.title).toSorted();
    expect(await titles()).toEqual(["Sources"]);
    const edited = await bob.note.update({
        spaceId,
        id: filed.id,
        requestId: RequestId.create(environment),
        revision: filed.revision,
        text: "Three papers",
    });
    expect(edited.text).toBe("Three papers");
    expect(
        await refusal(
            bob.note.delete({
                spaceId,
                id: filed.id,
                requestId: RequestId.create(environment),
                revision: 2,
            }),
        ),
    ).toEqual([
        "FORBIDDEN",
        "PERMISSION_DENIED",
        "permission 'manage' is forbidden: the caller lacks it",
    ]);

    // move a note only as its manager into an editable notebook
    await alice.note.move({
        spaceId,
        id: loose.id,
        requestId: RequestId.create(environment),
        revision: loose.revision,
        parentId: research.id,
    });
    expect(await titles()).toEqual(["Ideas", "Sources"]);
    expect(
        await refusal(
            bob.note.move({
                spaceId,
                id: loose.id,
                requestId: RequestId.create(environment),
                revision: 2,
                parentId: null,
            }),
        ),
    ).toEqual([
        "FORBIDDEN",
        "PERMISSION_DENIED",
        "permission 'manage' is forbidden: the caller lacks it",
    ]);

    // leave deleted notes restorable from the trash, and refuse changing them there
    await alice.note.delete({
        spaceId,
        id: loose.id,
        requestId: RequestId.create(environment),
        revision: 2,
    });
    expect(await titles()).toEqual(["Sources"]);
    expect(
        (await alice.note.list({ spaceId, deleted: "only" })).items.map((item) => item.title),
    ).toEqual(["Ideas"]);
    expect(
        await refusal(
            alice.note.update({
                spaceId,
                id: loose.id,
                requestId: RequestId.create(environment),
                text: "Late",
            }),
        ),
    ).toEqual(["PRECONDITION_FAILED", "OBJECT_BLOCKED", "cannot change note: it is in the trash"]);
    expect(
        await refusal(
            alice.note.grant({
                spaceId,
                id: loose.id,
                requestId: RequestId.create(environment),
                relation: "viewer",
                subject: principal.user.reference("universe", users.bob),
            }),
        ),
    ).toEqual(["PRECONDITION_FAILED", "OBJECT_BLOCKED", "cannot change note: it is in the trash"]);

    // list the trashed note's relationships
    expect(await alice.note.relationships({ spaceId, id: loose.id })).toEqual({
        items: [],
        cursor: null,
    });
    await alice.note.restore({
        spaceId,
        id: loose.id,
        requestId: RequestId.create(environment),
    });
    expect(await titles()).toEqual(["Ideas", "Sources"]);

    // follow the notes on bob's device and write into the shared notebook, predicted at once
    const device = await openDevice(users.bob);
    device.online();
    await device.waitFor(() => device.titles(), ["Ideas", "Sources"]);
    const questions = device.client
        .mutate(note)
        .create({ parentId: research.id, title: "Questions" });
    expect(ObjectReference.at("owner", await questions.predicted)).toEqual(
        principal.user.reference("universe", users.bob),
    );
    expect(await device.titles()).toEqual(["Ideas", "Questions", "Sources"]);
    await questions.confirmed;
    expect((await alice.note.list({ spaceId })).items.map((item) => item.title).toSorted()).toEqual(
        ["Ideas", "Questions", "Sources"],
    );
    expect(device.errors).toEqual([]);
});

test("queue notes offline, write batches atomically, drop rejected predictions and rebase", async () => {
    const { connect, endpoint, database, openDevice, environment } = await serveNotes();
    const alice = connect(users.alice);

    // queue a notebook and its first note in one offline mutation
    const device = await openDevice(users.alice);
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
        environment,
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
        requestId: RequestId.create(environment),
        text: "passport, charger",
    });
    device.follow();
    const local = async () => {
        const [row] = await device.client.database.select().from(note.table);
        const { title, text } = present(row, "the local note");

        return [title, text];
    };
    await device.waitFor(local, ["Luggage", "passport, charger"]);

    // push the new title beside the other device's text
    device.push();
    await retitled.confirmed;
    expect(await local()).toEqual(["Luggage", "passport, charger"]);
    const stored = await alice.note.get({ spaceId, id: packing.id });
    expect([stored.title, stored.text]).toEqual(["Luggage", "passport, charger"]);

    // refuse a batch the server forbids as a whole, and revert both of its predictions
    const bob = await openDevice(users.bob);
    await alice.notebook.grant({
        spaceId,
        id: book.id,
        requestId: RequestId.create(environment),
        relation: "viewer",
        subject: principal.user.reference("universe", users.bob),
    });
    bob.online();
    await bob.waitFor(() => bob.titles(), ["Luggage"]);
    const intrusion = bob.client.mutation(async (mutation) => {
        await mutation.call(notebook).create({ name: "Mine" });
        await mutation.call(note).create({ parentId: book.id, title: "Graffiti" });
    });
    await intrusion.predicted;
    expect(await refusal(intrusion.confirmed)).toEqual([
        "FORBIDDEN",
        "PERMISSION_DENIED",
        "permission 'write' is forbidden: the caller lacks it",
    ]);
    expect(await bob.titles()).toEqual(["Luggage"]);
    expect(
        (await bob.client.database.select({ name: notebook.table.name }).from(notebook.table)).map(
            (row) => row.name,
        ),
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
    await device.waitFor(() => device.client.prediction.pending(device.client.database), []);
    await database.transaction(
        async (transaction) =>
            await transaction.delete(note.table).where(eq(note.table.id, lostId)),
    );
    await database.transaction(async (transaction) => await transaction.delete(journal));
    await database.log.renew();
    device.follow();
    await lost.confirmed;
    expect((await alice.note.get({ spaceId, id: lostId })).title).toBe("Tickets");
    expect(await device.titles()).toEqual(["Luggage", "Tickets"]);
    expect(device.errors).toEqual([]);
});

test("replicate only the queries a device subscribes to, with their includes, as rows move between them", async () => {
    const { connect, openDevice, environment } = await serveNotes();
    const alice = connect(users.alice);

    // file notes into two notebooks
    const notebookNamed = (name: string) =>
        alice.notebook.create({ spaceId, requestId: RequestId.create(environment), name });
    const noteIn = (parentId: string, title: string) =>
        alice.note.create({
            spaceId,
            requestId: RequestId.create(environment),
            parentId: schema.identifier("notebook").parse(parentId),
            title,
        });
    const travel = await notebookNamed("Travel");
    const work = await notebookNamed("Work");
    await noteIn(travel.id, "Packing");
    const plan = await noteIn(work.id, "Plan");

    // replicate the travel notes and the notebook they belong to, nothing of work
    const device = await openDevice(users.alice, []);
    const travelNotes = device.client.query.note
        .findMany({ where: { parentId: travel.id }, with: { parent: true } })
        .subscribe();
    device.online();
    await travelNotes.ready;
    const books = async () =>
        (await device.client.database.select({ name: notebook.table.name }).from(notebook.table))
            .map((row) => row.name)
            .toSorted();
    expect([await device.titles(), await books()]).toEqual([["Packing"], ["Travel"]]);

    // take a note in when it moves into travel and let it go when it moves out
    await alice.note.move({
        spaceId,
        id: plan.id,
        requestId: RequestId.create(environment),
        parentId: travel.id,
    });
    await device.waitFor(() => device.titles(), ["Packing", "Plan"]);
    await alice.note.move({
        spaceId,
        id: plan.id,
        requestId: RequestId.create(environment),
        parentId: work.id,
    });
    await device.waitFor(() => device.titles(), ["Packing"]);

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
    await device.waitFor(() => device.titles(), ["Plan"]);
    expect(await books()).toEqual(["Work"]);

    // resume from the recorded queries after a restart, without starting over
    await device.offline();
    device.online();
    await device.waitFor(() => device.titles(), ["Plan"]);
    expect(device.errors).toEqual([]);
});

test("count a notebook's notes outside the trash, predicted on a device before the server confirms", async () => {
    const { connect, openDevice, environment } = await serveNotes();
    const alice = connect(users.alice);
    const travel = await alice.notebook.create({
        spaceId,
        requestId: RequestId.create(environment),
        name: "Travel",
    });
    expect(travel.noteCount).toBe(0);

    // predict the count of a note created offline, before the server has it
    const device = await openDevice(users.alice);
    device.follow();
    await device.waitFor(
        async () => device.client.database.select({ id: notebook.table.id }).from(notebook.table),
        [{ id: travel.id }],
    );
    const created = device.client.mutate(note).create({ parentId: travel.id, title: "Packing" });
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
    await alice.note.delete({ spaceId, id, requestId: RequestId.create(environment) });
    expect((await alice.notebook.get({ spaceId, id: travel.id })).noteCount).toBe(0);
    await device.waitFor(count, 0);
    expect(device.errors).toEqual([]);
});

test("share one browser database between tabs, handing it over when the leading tab closes", async () => {
    const simulation = testSimulation();
    const { connect, endpoint } = await serveNotes({ simulation, node: "browser" });
    const alice = connect(users.alice);

    // share one browser's locks, channel and database worker between tabs
    const environment = new BrowserSimulation(simulation).environment("browser");
    const database = await WasmClient.memory(environment);
    const worker: BrowserTabWorker = async () => {
        const channel = broadcastChannel("destack:notes", environment);
        const stop = serveDatabase(database, channel, environment);

        return async () => stop();
    };
    const open = () =>
        BrowserTab.open({
            environment,
            name: "notes",
            objects: { notebook, note },
            scope: spaceId,
            caller: principal.user.reference("universe", users.alice),
            device: principal.device.reference("universe", "device-browser"),
            endpoint: endpoint(users.alice),
            reconnect: unmoved,
            worker,
            report: (error) => {
                throw error;
            },
        });

    // lead the database in the first tab, and share it with the second
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
    await first.client.database.log.waitUntil(
        async () => !(await isPresent()),
        environment.clock.timeout(5000),
    );
    expect([before, await isPresent()]).toEqual([true, false]);

    // hand the database over once the leading tab closes, and go on writing through the second
    await first.close();
    const packing = second.client.mutate(note).create({ parentId: id, title: "Packing" });
    await packing.confirmed;
    expect((await alice.note.list({ spaceId })).items.map((item) => item.title)).toEqual([
        "Packing",
    ]);
    await second.close();
});

test("decide checks on a device from its replica of the caller's access rows, and follow a grant and its revocation into them", async () => {
    const { connect, openDevice, environment } = await serveNotes();
    const alice = connect(users.alice);

    // create a notebook of alice, which bob's device cannot read yet
    const research = await alice.notebook.create({
        spaceId,
        requestId: RequestId.create(environment),
        name: "Research",
    });
    const device = await openDevice(users.bob);
    device.online();
    const checks = async () => [
        await device.client.check(notebook, research.id, "read"),
        await device.client.check(notebook, research.id, "write"),
        await device.client.check(notebook, research.id, "manage"),
    ];
    expect(await checks()).toEqual([false, false, false]);

    // let bob edit once the grant arrives at the device, without managing
    const granted = await alice.notebook.grant({
        spaceId,
        id: research.id,
        requestId: RequestId.create(environment),
        relation: "editor",
        subject: principal.user.reference("universe", users.bob),
    });
    await device.waitFor(checks, [true, true, false]);

    // take the access away again once the revocation arrives at the device
    await alice.notebook.revoke({
        spaceId,
        id: research.id,
        requestId: RequestId.create(environment),
        relationshipId: granted.id,
    });
    await device.waitFor(checks, [false, false, false]);
    expect(device.errors).toEqual([]);
});

test("decide checks on a device as the server decides them, for a group its authenticator vouches for", async () => {
    const team = { ...principal.group.reference("universe", "team"), relation: "member" };
    const { connect, openDevice, environment } = await serveNotes({
        groups: new Map([[users.bob, [team]]]),
    });
    const alice = connect(users.alice);
    const bob = connect(users.bob);

    // share alice's notebook with the team bob is vouched into
    const research = await alice.notebook.create({
        spaceId,
        requestId: RequestId.create(environment),
        name: "Research",
    });
    await alice.notebook.grant({
        spaceId,
        id: research.id,
        requestId: RequestId.create(environment),
        relation: "editor",
        subject: team,
    });

    // allow on the device exactly what the server allows bob
    const device = await openDevice(users.bob);
    device.online();
    const checks = async () => [
        await device.client.check(notebook, research.id, "read"),
        await device.client.check(notebook, research.id, "write"),
        await device.client.check(notebook, research.id, "manage"),
    ];
    expect((await bob.notebook.get({ spaceId, id: research.id })).name).toBe("Research");
    await device.waitFor(checks, [true, true, false]);
    expect(device.errors).toEqual([]);
});

test("resume a store stream from the last event it sent, at that page's position", async () => {
    // run in a world of its own
    const simulation = testSimulation();
    const world = simulation.environment("test");

    const { connect, endpoint, environment } = await serveNotes({ simulation });
    const alice = connect(users.alice);
    await alice.notebook.create({
        spaceId,
        requestId: RequestId.create(environment),
        name: "Research",
    });
    const store = ObjectClient.store(endpoint(users.alice), world);
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
        const pages = await store.stream(
            { ...subscription, revision: FOLLOWER_REVISION },
            { signal: controller.signal, ...options },
        );
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

test("follow a caller's access rows on behalf of the caller itself as without a subject", async () => {
    // run in a world of its own, alice with a notebook
    const simulation = testSimulation();
    const world = simulation.environment("test");
    const { connect, endpoint, environment } = await serveNotes({ simulation });
    await connect(users.alice).notebook.create({
        spaceId,
        requestId: RequestId.create(environment),
        name: "Research",
    });
    const store = ObjectClient.store(endpoint(users.alice), world);
    const subscription = {
        name: "access",
        publication: "access",
        scope: spaceId,
        target: spaceId,
        parameters: {},
        revision: FOLLOWER_REVISION,
    };

    // read the rows up to the first complete page, without a subject and with alice as it
    const complete = async (subject?: ReturnType<typeof principal.user.reference>) => {
        const controller = new AbortController();
        const asked = subject === undefined ? subscription : { ...subscription, subject };
        const changes: Page["changes"][number][] = [];
        for await (const page of await store.stream(asked, { signal: controller.signal })) {
            changes.push(...page.changes);
            if (page.complete) {
                controller.abort();

                return changes;
            }
        }
        throw new TypeError("the stream ended before a complete page");
    };
    const alice = principal.user.reference("universe", users.alice);
    const own = await complete();
    expect({ isFilled: own.length > 0, onBehalf: await complete(alice) }).toEqual({
        isFilled: true,
        onBehalf: own,
    });
});

test("restart a caller's access stream only on access changes concerning the caller", async () => {
    // run in a world of its own
    const simulation = testSimulation();
    const world = simulation.environment("test");

    const { connect, endpoint, environment } = await serveNotes({ simulation });
    const alice = connect(users.alice);
    const research = await alice.notebook.create({
        spaceId,
        requestId: RequestId.create(environment),
        name: "Research",
    });

    // follow bob's access rows, recording every page
    const controller = new AbortController();
    onTestFinished(() => controller.abort());
    const pages: Page[] = [];
    const store = ObjectClient.store(endpoint(users.bob), world);
    const subscription = {
        name: "access",
        publication: "access",
        scope: spaceId,
        target: spaceId,
        parameters: {},
    };
    const errors: unknown[] = [];
    const following = (async () => {
        for await (const page of await store.stream(
            { ...subscription, revision: FOLLOWER_REVISION },
            { signal: controller.signal },
        )) {
            pages.push(page);
        }
    })().catch((error: unknown) => {
        // record failures other than the closing abort
        if (!controller.signal.aborted) {
            errors.push(error);
        }
    });
    await simulation.waitUntil(() => pages.some((page) => page.complete));

    // share the notebook with carol, then with bob
    const share = async (user: (typeof users)[keyof typeof users]) =>
        await alice.notebook.grant({
            spaceId,
            id: research.id,
            requestId: RequestId.create(environment),
            relation: "viewer",
            subject: principal.user.reference("universe", user),
        });
    await share(users.carol);

    // have bob's stream publish the position past carol's share at the feed's next heartbeat, so a restart for it would show first
    const watermark = await ObjectClient.store(endpoint(users.alice), world).watermark({
        scope: spaceId,
        homes: [],
    });
    await simulation.sleep(
        simulation.clock.now() + HEARTBEAT_MILLISECONDS - simulation.clock.now(),
    );
    const isPast = () =>
        pages.some(
            (page) =>
                page.complete &&
                page.position.log === watermark.log &&
                page.position.sequence >= watermark.sequence,
        );
    await simulation.waitUntil(isPast);
    const resetsPastCarol = pages.filter((page) => page.reset).length;
    const shared = await share(users.bob);

    // start one snapshot for bob's share alone, none for carol's, and deliver the share
    const resets = () => pages.filter((page) => page.reset).length;
    await simulation.waitUntil(() => resets() === 2);
    const isShared = pages.some((page) =>
        page.changes.some((change) => change.row["id"] === shared.id),
    );
    expect([resetsPastCarol, resets(), isShared]).toEqual([1, 2, true]);
    controller.abort();
    await following;
    expect(errors).toEqual([]);
});

test("show a notebook's relationships to everyone who reads it, and to no one else", async () => {
    const { connect, environment } = await serveNotes();
    const alice = connect(users.alice);
    const bob = connect(users.bob);
    const carol = connect(users.carol);
    const dave = connect(users.dave);

    // share alice's notebook with bob as editor and carol as viewer
    const research = await alice.notebook.create({
        spaceId,
        requestId: RequestId.create(environment),
        name: "Research",
    });
    for (const [user, relation] of [
        [users.bob, "editor"],
        [users.carol, "viewer"],
    ] as const) {
        await alice.notebook.grant({
            spaceId,
            id: research.id,
            requestId: RequestId.create(environment),
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
    expect(await refusal(dave.notebook.relationships({ spaceId, id: research.id }))).toEqual([
        "NOT_FOUND",
        "NOT_FOUND",
        `notebook '${research.id}' not found`,
    ]);
});

test("read notebooks with their notes and notes with their notebook in a method's view, a limit per notebook", async () => {
    const { connect, database, relations, environment } = await serveNotes();
    const alice = connect(users.alice);

    // file two notes into travel, one into work, and keep one loose
    const create = (name: string) =>
        alice.notebook.create({ spaceId, requestId: RequestId.create(environment), name });
    const file = (title: string, parentId?: string) =>
        alice.note.create({
            spaceId,
            requestId: RequestId.create(environment),
            title,
            ...(parentId === undefined
                ? {}
                : { parentId: schema.identifier("notebook").parse(parentId) }),
        });
    const travel = await create("Travel");
    const work = await create("Work");
    await file("Packing", travel.id);
    await file("Tickets", travel.id);
    await file("Plan", work.id);
    await file("Ideas");

    // read both directions of the notebook relation through the live view, as a method does
    const queries = ViewSubscriber.queries({ notebook, note }, relations, (table) =>
        Snapshot.live(database).read(table),
    );
    const books = await queries.notebook.findMany({
        orderBy: { name: "asc" },
        with: { notes: { orderBy: { title: "asc" } } },
    });
    const firsts = await queries.notebook.findMany({
        orderBy: { name: "asc" },
        with: { notes: { orderBy: { title: "asc" }, limit: 1 } },
    });
    const notes = await queries.note.findMany({
        orderBy: { title: "asc" },
        with: { parent: true },
    });

    // expect every note under its notebook, the first per notebook, and a loose note's notebook null
    expect({
        books: books.map((book) => [book.name, book.notes.map((each) => each.title)]),
        firsts: firsts.map((book) => [book.name, book.notes.map((each) => each.title)]),
        notes: notes.map((each) => [each.title, each.parent?.name ?? null]),
    }).toEqual({
        books: [
            ["Travel", ["Packing", "Tickets"]],
            ["Work", ["Plan"]],
        ],
        firsts: [
            ["Travel", ["Packing"]],
            ["Work", ["Plan"]],
        ],
        notes: [
            ["Ideas", null],
            ["Packing", "Travel"],
            ["Plan", "Work"],
            ["Tickets", "Travel"],
        ],
    });
});

test("watch a query's value at each change and several queries' latest values together, ending once aborted", async () => {
    const { connect, openDevice, environment } = await serveNotes();
    const alice = connect(users.alice);
    const create = (name: string) =>
        alice.notebook.create({ spaceId, requestId: RequestId.create(environment), name });

    // watch the notebooks' names, and the names beside the notes' titles
    const device = await openDevice(users.alice);
    device.online();
    const { query } = device.client;
    const watching = new AbortController();
    const names = query.notebook.findMany({ orderBy: { name: "asc" } }).watch(watching.signal);
    const together = RelationalQuery.latest(
        {
            books: query.notebook.findMany({ orderBy: { name: "asc" } }),
            notes: query.note.findMany({ orderBy: { title: "asc" } }),
        },
        watching.signal,
    );
    const named = async () =>
        (await reached(names, (rows) => rows.length === 2)).map((row) => row.name);
    const joined = async () => {
        const { books, notes } = await reached(together, (rows) => rows.books.length === 2);

        return [books.map((row) => row.name), notes.map((row) => row.title)];
    };

    // see both notebooks once the second arrives, then nothing after the abort
    await create("Travel");
    await create("Work");
    const seen = [await named(), await joined()];
    watching.abort();
    const ended = [(await names.next()).done, (await together.next()).done];
    expect({ seen, ended }).toEqual({
        seen: [
            ["Travel", "Work"],
            [["Travel", "Work"], []],
        ],
        ended: [true, true],
    });
});

/** Read a watched value until one meets a condition. */
async function reached<Value>(
    values: AsyncGenerator<Value>,
    isReached: (value: Value) => boolean,
): Promise<Value> {
    for (let next = await values.next(); next.done !== true; next = await values.next()) {
        if (isReached(next.value)) {
            return next.value;
        }
    }
    throw new TypeError("the watch ended before the value was reached");
}

test("fail the queries open once the source refuses to follow, and refuse later ones again", async () => {
    const { endpoint, environment } = await serveNotes();
    const errors: unknown[] = [];

    // follow a space the store does not serve, as alice
    const storage = await TestDatabase.create(ObjectClient.tables({ notebook, note }), {
        environment,
        dialect: "sqlite",
        storage: "file",
    });
    const client = await ObjectClient.open({
        database: storage.database,
        objects: { notebook, note },
        scope: schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000ff"),
        caller: principal.user.reference("universe", users.alice),
        endpoint: endpoint(users.alice),
        reconnect: unmoved,
        environment,
    });
    const stopping = new AbortController();
    const running = client.run(stopping.signal, (error) => errors.push(error));
    onTestFinished(async () => {
        stopping.abort();
        await running;
        await client.close();
        await storage.close();
    });

    // watch the notebooks and read them, then read the notes after the refusal, refused each time
    const outside = [
        "NOT_FOUND",
        "SCOPE_OUTSIDE_REQUEST_NOT_FOUND",
        "scope 'space-01996ab0-0000-7000-8000-0000000000ff' in the request's scope not found",
    ];
    const watched = client.query.notebook.findMany().watch(stopping.signal);
    const failed = await Promise.allSettled([watched.next(), client.query.notebook.findMany()]);
    const again = await Promise.allSettled([client.query.note.findMany()]);
    const reasons = [...failed, ...again].map((settled) =>
        settled.status === "rejected"
            ? refusal(Promise.reject(settled.reason))
            : Promise.resolve(settled.value),
    );

    // expect each query to fail with the refusal, which every refused follow reported
    const reported = await Promise.all(errors.map((error) => refusal(Promise.reject(error))));
    expect({
        reasons: await Promise.all(reasons),
        reported: [...new Set(reported.map((refused) => JSON.stringify(refused)))],
    }).toEqual({ reasons: [outside, outside, outside], reported: [JSON.stringify(outside)] });
});
