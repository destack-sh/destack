import { expect, refusal, test } from "@destack/test";
import {
    asc,
    binary,
    defineTable,
    eq,
    Key,
    TABLE,
    text,
    type Row,
    type Relation,
    type Table,
    type LogPosition,
    type DatabaseConnection,
    defineDatabase,
    Snapshot,
    Transaction,
} from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { onTestFinished } from "@destack/test";
import { Feed } from "../feed/feed.ts";
import type { Stream } from "../feed/stream.ts";
import { nth, schema } from "@destack/schema";
import { replicaTables, Replica, replicaPosition, replicaKey } from "./replica.ts";
import type { Resumption } from "./publication.ts";
import { Subject } from "../call/subject.ts";
import {
    asset,
    first,
    label,
    note,
    open,
    openReplica,
    project,
    relations,
    replicate,
    tag,
    task,
    TABLES,
} from "../test/fixture.ts";
import type { Query } from "../query/query.ts";
import type { AggregateRow } from "../query/aggregate.ts";
import type { Page } from "../query/page.ts";
import { SyncError } from "../error/error.ts";
import { Scope } from "../scope/scope.ts";
import { Clock } from "@destack/environment";
import { VirtualClock } from "@destack/environment/simulation";

/** The source log of the replica scenarios. */
const LOG = "01996ab0-0000-7000-8000-0000000000aa";

/** The rows of a feed's page in the paging scenarios, a few to cross pages with few rows. */
const PAGE_SIZE = 4;

/** The notes of a snapshot spanning two pages. */
const SNAPSHOT_NOTES = PAGE_SIZE + 2;

/** The rows one insert writes, within every dialect's parameter limit. */
const INSERT_BATCH = 500;

/** The commits the follow cost test counts: enough for a steady cost. */
const FOLLOWED_COMMITS = 20;

/** A principal a caller-audience replica is decided for. */
const ALICE = {
    packageId: schema.identifier("package").parse("package-01996ab0-0000-7000-8000-000000000001"),
    type: "user",
    scope: Scope.universe.id,
    id: "alice",
};

/** A principal reading beside alice. */
const BOB = { ...ALICE, id: "bob" };

test("follow a scope of another database into a local replica, then promote its rows to local rows", async () => {
    const source = await open();
    const local = await openReplica();
    const feed = new Feed(source, [note, replicaPosition], { clock: Clock.system });
    const notes = new Replica({ name: "notes", scope: "inbox", tables: [note] });
    await source.insert(note).values([first, { ...first, id: "b", scope: "archive" }]);
    await local.insert(note).values({ ...first, id: "stale" });

    // follow until the replica has the renamed note, confirming each awaited position
    const controller = new AbortController();
    const { following, confirmed } = followConfirmed(notes, local, source, feed, controller.signal);
    expect(await confirmed()).toBe(true);

    // record the replica synced, storing the scope's notes and no other table, for no reader without a subscription
    expect([
        await Replica.isSynced(local, "notes", "inbox"),
        await Replica.isSynced(local, "notes", "archive"),
        await Replica.isReplicated(Snapshot.live(local), "inbox", note),
        await Replica.isReplicated(Snapshot.live(local), "inbox", task),
        await Replica.scopes(local, note),
        await Replica.scopes(local, task),
        await Replica.scopes(local, note, "inbox", undefined),
    ]).toEqual([true, false, true, false, ["inbox"], [], []]);
    await source.update(note).set({ title: "Renamed" }).where(eq(note.id, "a"));
    await source.delete(note).where(eq(note.id, "b"));
    expect(await confirmed()).toBe(true);
    controller.abort();
    await following;

    // retain the renamed rows and prune the rest
    expect((await local.select().from(note)).map((row) => [row.id, row.title])).toEqual([
        ["a", "Renamed"],
    ]);
    expect(await notes.position(local)).toEqual(await source.log.position());

    // release a row the replica's database pruned from the replica's bookkeeping
    const stored = async () =>
        (await local.select({ key: replicaKey.key }).from(replicaKey)).map((row) => row.key);
    const before = await stored();
    await Replica.release(local, note, [{ id: "a" }]);
    expect([before, await stored()]).toEqual([[Key.name(note, { id: "a" })], []]);

    // retain the rows once promoted
    await Transaction.run(local, (transaction) => notes.promote(transaction));
    expect([
        await Replica.isReplicated(Snapshot.live(local), "inbox"),
        (await local.select().from(note)).map((row) => row.id),
    ]).toEqual([false, ["a"]]);
});

test("delete the rows of a table a reshaped replica stopped replicating, retaining its other tables' rows", async () => {
    // store a note and a task in the inbox
    const source = await open();
    const local = await openReplica();
    const feed = new Feed(source, [note, task, replicaPosition], { clock: Clock.system });
    await source.insert(note).values(first);
    await source
        .insert(task)
        .values({ id: "t1", scope: "inbox", state: "open", rank: 1, isSecret: false });
    const follow = async (replica: Replica) => {
        const controller = new AbortController();
        const following = replica.follow(
            local,
            (from, signal) => feed.subscribe(replica.queries, from.after, signal),
            controller.signal,
            { synced: () => {}, clock: Clock.system },
        );
        const signal = AbortSignal.timeout(5000);
        expect(await Replica.wait(local, "inbox", await source.log.position(), signal)).toBe(true);
        controller.abort();
        await following;

        return [
            (await local.select().from(note)).map((row) => row.id),
            (await local.select().from(task)).map((row) => row.id),
        ];
    };

    // replicate both tables, then only the notes under the same name once another note arrived
    const both = await follow(new Replica({ name: "work", scope: "inbox", tables: [note, task] }));
    await source.insert(note).values({ ...first, id: "b" });
    const notes = await follow(new Replica({ name: "work", scope: "inbox", tables: [note] }));

    // retain the task while replicated, and delete it once the reshaped replica drops tasks
    expect([both, notes]).toEqual([
        [["a"], ["t1"]],
        [["a", "b"], []],
    ]);
});

test("follow two databases each replicating the other's table, receiving each write as a separate page and no bare position", async () => {
    // store the notes on one database and the projects on the other, on a clock that stands still
    const clock = new VirtualClock(0);
    const home = await open();
    const other = await open();
    await home.insert(note).values(first);
    await other.insert(project).values({ id: "p", scope: "inbox", name: "Launch" });

    // follow each database's table from the other, with the follower's origin, recording the pages
    const notes = new Replica({ name: "notes", scope: "inbox", tables: [note] });
    const projects = new Replica({ name: "projects", scope: "inbox", tables: [project] });
    const homeFeed = new Feed(home, [note, replicaPosition], { clock });
    const otherFeed = new Feed(other, [project, replicaPosition], { clock });
    const received: { notes: Page[]; projects: Page[] } = { notes: [], projects: [] };
    const controller = new AbortController();
    const following = Promise.all([
        notes.follow(other, recordFrom(homeFeed, notes, received.notes), controller.signal, {
            synced: () => {},
            clock,
        }),
        projects.follow(
            home,
            recordFrom(otherFeed, projects, received.projects),
            controller.signal,
            { synced: () => {}, clock },
        ),
    ]);

    // see each other's rows
    const rows = async () => [
        (await other.select().from(note)).map((row) => row.id),
        (await home.select().from(project)).map((row) => row.id),
    ];
    await expect.poll(rows).toEqual([["a"], ["p"]]);

    // write once more on each side, and receive each write as the next page, never a bare position first
    received.notes.length = 0;
    received.projects.length = 0;
    await home.insert(note).values({ ...first, id: "b" });
    await other.insert(project).values({ id: "q", scope: "inbox", name: "Ship" });
    await expect.poll(rows).toEqual([
        ["a", "b"],
        ["p", "q"],
    ]);
    controller.abort();
    await following;
    expect([changedIds(received.notes), changedIds(received.projects)]).toEqual([[["b"]], [["q"]]]);
});

test("refuse publishing a replicated table back to the database it replicates from, with rows or without, and publish it on to any other", async () => {
    // replicate the inbox's notes from one database into another
    const home = await open();
    const other = await open();
    await home.insert(note).values(first);
    const inbox = new Replica({ name: "inbox", scope: "inbox", tables: [note] });
    const controller = new AbortController();
    const following = inbox.follow(
        other,
        followFrom(new Feed(home, [note, replicaPosition], { clock: Clock.system }), inbox),
        controller.signal,
        { synced: () => {}, clock: Clock.system },
    );
    const signal = AbortSignal.timeout(5000);
    expect(await Replica.wait(other, "inbox", await home.log.position(), signal)).toBe(true);
    controller.abort();
    await following;

    // publish the replica on to a third database, never back to the notes' home
    const homeOrigin = (await home.log.position()).log;
    const archive = new Replica({ name: "archive", scope: "archive", tables: [note] });
    await expect(inbox.requireAcyclic(other, "third")).resolves.toBeUndefined();
    await expect(archive.requireAcyclic(other, homeOrigin)).rejects.toEqual(
        new SyncError(
            "CYCLE",
            `replica archive of archive would publish note back to ${homeOrigin}, its source in replica inbox of inbox`,
        ),
    );

    // refuse publishing the notes back once the replica from their home has none of them
    await other.delete(note);
    await other.delete(replicaKey);
    await expect(archive.requireAcyclic(other, homeOrigin)).rejects.toEqual(
        new SyncError(
            "CYCLE",
            `replica archive of archive would publish note back to ${homeOrigin}, its source in replica inbox of inbox`,
        ),
    );
});

test("replicate several scopes in one replica recorded elsewhere, storing each scope with its row in the replica", async () => {
    // store the inbox's and the archive's rows with their scope rows, and a note of the drafts
    const tables = [Scope.table, ...TABLES];
    const source = await open(tables);
    const local = await openReplica(tables);
    const scopes = ["inbox", "archive"];
    await source.insert(Scope.table).values(
        scopes.map((scope) => ({
            scope,
            parent: Scope.universe.id,
            partition: Scope.universe.id,
            packageId: Scope.universe.packageId,
            type: "folder",
        })),
    );
    await source
        .insert(note)
        .values([
            first,
            { ...first, id: "b", scope: "archive" },
            { ...first, id: "c", scope: "drafts" },
        ]);

    // follow both scopes in one replica recorded at the universe
    const folders = new Replica({
        name: "folders",
        scope: Scope.universe.id,
        tables: [Scope.table, note],
        scopes: new Map<Table, readonly string[]>([
            [Scope.table, scopes],
            [note, scopes],
        ]),
    });
    const feed = new Feed(source, [Scope.table, note, replicaPosition], {
        clock: Clock.system,
    });
    const subscription = { ...inboxSubscription("folders"), scope: Scope.universe.id };
    const controller = new AbortController();
    const following = folders.follow(
        local,
        (from, signal) => feed.subscribe(folders.queries, from.after, signal),
        controller.signal,
        { synced: () => {}, clock: Clock.system, subscription, audience: "contained" },
    );
    const head = await source.log.position();
    const signal = AbortSignal.timeout(5000);
    const arrived = [
        await Replica.wait(local, "inbox", head, signal),
        await Replica.wait(local, "archive", head, signal),
    ];
    controller.abort();
    await following;

    // store both scopes' notes, and find the one record as the replica of each scope with its row in the replica
    const origins = await Replica.origins(local, "notes", [...scopes, "drafts", "universe"]);
    expect([
        arrived,
        (await local.select({ id: note.id }).from(note).orderBy(asc(note.id))).map((row) => row.id),
        [...origins.keys()].toSorted(),
        [...origins.values()].map((origin) => origin.position),
    ]).toEqual([
        [true, true],
        ["a", "b"],
        ["archive", "inbox", "universe"],
        [head, head, head],
    ]);
});

test("replicate a whole database: follow its logged rows, then capture every column once its source stops writing", async () => {
    // store entries of several scopes with sensitive and binary columns, an unlogged cache and a machine-bound key
    const tables = [entry, cache, machineKey, ...replicaTables];
    const source = (await TestDatabase.create(tables, { isMigrated: true })).database;
    const target = (await TestDatabase.create(tables, { isMigrated: true, isReplica: true }))
        .database;
    onTestFinished(async () => {
        await source.close();
        await target.close();
    });
    const replicated = [entry, cache, machineKey];
    const replica = new Replica({
        name: "database",
        scope: "database",
        tables: replicated,
        everywhere: new Set(replicated),
    });
    const feed = new Feed(source, [entry, machineKey, replicaPosition], {
        clock: Clock.system,
    });
    const data = new Uint8Array([1, 2, 3]);
    await source
        .insert(entry)
        .values({ id: "a", scope: "inbox", title: "First", secret: "s1", data });

    // follow the logged rows while the source writes
    const controller = new AbortController();
    const following = replica.follow(
        target,
        (from, signal) => feed.subscribe(replica.queries, from.after, signal),
        controller.signal,
        { synced: () => {}, clock: Clock.system },
    );
    const signal = AbortSignal.timeout(5000);
    expect(await Replica.wait(target, "database", await source.log.position(), signal)).toBe(true);
    await source
        .insert(entry)
        .values({ id: "b", scope: "archive", title: "Second", secret: "s2", data });
    await source.insert(cache).values({ id: "c", value: "cached" });
    await source.insert(machineKey).values({ id: "k", scope: "inbox", wrapped: "source:k" });
    expect(await Replica.wait(target, "database", await source.log.position(), signal)).toBe(true);
    controller.abort();
    await following;

    // capture every column once the source stops writing
    const seal = async (table: Table, row: Row) => rewrapped(table, row, "source:", "moving:");
    const unseal = async (table: Table, row: Row) => rewrapped(table, row, "moving:", "target:");
    const applied = replica.apply(target, feed.capture(replica.captured, signal, { seal }), {
        clock: Clock.system,
        open: unseal,
    });
    while ((await applied.next()).done !== true) {
        // apply each captured page
    }
    await Transaction.run(target, (transaction) => replica.promote(transaction));

    // store exactly the source's rows with the key rewrapped for the target
    const read = async (database: typeof source) => [
        await database.select().from(entry).orderBy(asc(entry.id)),
        await database.select().from(cache),
        await database.select().from(machineKey),
    ];
    expect(await read(target)).toEqual([
        (await read(source))[0],
        [{ id: "c", value: "cached" }],
        [{ id: "k", scope: "inbox", wrapped: "target:k" }],
    ]);
    expect(await Replica.isReplicated(Snapshot.live(target), "database")).toBe(false);
});

test("follow and wait for each commit of one row in a fixed count of statements, registering the replica once", async () => {
    const source = await open();
    const local = await openReplica();
    // stand the feed's clock still, so no heartbeat lands between two commits
    const feed = new Feed(source, [note, replicaPosition], { clock: new VirtualClock(0) });
    const notes = new Replica({ name: "notes", scope: "inbox", tables: [note] });

    // follow the inbox until the replica has its first note
    const controller = new AbortController();
    const { following, confirmed } = followConfirmed(notes, local, source, feed, controller.signal);
    await source.insert(note).values(first);
    await confirmed();

    // commit notes one at a time, counting the statements each costs the replica once it arrives at the commit
    const costs: number[] = [];
    for (let index = 0; index < FOLLOWED_COMMITS; index++) {
        const before = local.driver.state.statements;
        await source.insert(note).values({ ...first, id: `followed-${index}` });
        const committed = await source.log.position();
        expect(await Replica.wait(local, notes.scope, committed, AbortSignal.timeout(5000))).toBe(
            true,
        );
        costs.push(local.driver.state.statements - before);
    }
    controller.abort();
    await following;

    // spend nine statements on each commit, the wait's own read included
    expect(new Set(costs)).toEqual(new Set([9]));
});

test("capture a space's few dozen rows across many tables as one page applied in one transaction, and a larger one in pages of rows staged before the last", async () => {
    // store a space's rows in many tables, most of them empty
    const tables = SPACE_TABLES;
    const source = (await TestDatabase.create([...tables, ...replicaTables], { isMigrated: true }))
        .database;
    const target = (
        await TestDatabase.create([...tables, ...replicaTables], {
            isMigrated: true,
            isReplica: true,
        })
    ).database;
    onTestFinished(async () => {
        await source.close();
        await target.close();
    });
    const replica = new Replica({ name: "transfer", scope: "space", tables });
    const feed = new Feed(source, [...tables, replicaPosition], { clock: Clock.system });
    const signal = AbortSignal.timeout(5000);

    // capture and apply the space's rows, counting pages and statements
    const capture = async () => {
        const read = source.driver.state.statements;
        const written = target.driver.state.statements;
        const pages: Page[] = [];
        const applied = replica.apply(target, feed.capture(replica.captured, signal), {
            clock: Clock.system,
        });
        for await (const page of applied) {
            pages.push(page);
        }
        const staged = pages.filter((page) => !page.complete).length;

        return {
            pages: pages.map((page) => [page.reset, page.complete, page.changes.length]),
            staged,
            read: source.driver.state.statements - read,
            written: target.driver.state.statements - written,
        };
    };

    // capture a few dozen rows in a tenth of the tables as one page
    for (const table of tables.slice(0, SPACE_TABLES.length / 10)) {
        await source.insert(table).values(rowsOf(table, 6));
    }
    const small = await capture();

    // capture more rows than a page across two tables as two pages, the first staged
    const head = nth(tables, 0);
    await source.insert(head).values(rowsOf(head, INSERT_BATCH).slice(6));
    const large = await capture();

    // read each table once per page of its rows, write each table's rows once, prune each table once
    expect({ small, large }).toEqual({
        small: { pages: [[true, true, 24]], staged: 0, read: 42, written: 60 },
        large: {
            pages: [
                [true, false, 500],
                [false, true, 18],
            ],
            staged: 1,
            read: 43,
            written: 62,
        },
    });
    expect(await target.select().from(head)).toHaveLength(INSERT_BATCH);
});

test("snapshot again once the replicated tables change shape", async () => {
    const source = await open();
    const local = await openReplica();
    const feed = new Feed(source, [project, task, replicaPosition], { clock: Clock.system });
    const projects = new Replica({ name: "work", scope: "inbox", tables: [project] });
    await source.insert(project).values({ id: "p1", scope: "inbox", name: "Plan" });

    // replicate projects, and projects with tasks
    const controller = new AbortController();
    const following = projects.follow(
        local,
        (from, signal) => feed.subscribe(projects.queries, from.after, signal),
        controller.signal,
        { synced: () => {}, clock: Clock.system },
    );
    const signal = AbortSignal.timeout(5000);
    expect(await Replica.wait(local, "inbox", await source.log.position(), signal)).toBe(true);
    controller.abort();
    await following;
    const work = new Replica({ name: "work", scope: "inbox", tables: [project, task] });

    // record a position per publication
    expect([await projects.position(local), await work.position(local)]).toEqual([
        await source.log.position(),
        undefined,
    ]);
});

test("snapshot a row the source replaced under a new key with the same unique name", async () => {
    const tables = [label, project, ...replicaTables];
    const source = await open(tables);
    const local = await openReplica(
        defineDatabase({
            name: "replica",
            tables: replicaTables,
            replicated: [label, project],
        }),
    );
    const feed = new Feed(source, [label, project, replicaPosition], { clock: Clock.system });
    await source.insert(label).values({ id: "l1", scope: "inbox", name: "red" });

    // replicate the labels until synced
    await syncLabels(
        new Replica({ name: "work", scope: "inbox", tables: [label] }),
        source,
        local,
        feed,
    );

    // replace the label under a new key, then snapshot again with the replica reshaped
    await source.delete(label).where(eq(label.id, "l1"));
    await source.insert(label).values({ id: "l2", scope: "inbox", name: "red" });
    await syncLabels(
        new Replica({ name: "work", scope: "inbox", tables: [label, project] }),
        source,
        local,
        feed,
    );

    // retain only the replacing row
    expect(await local.select().from(label)).toEqual([{ id: "l2", scope: "inbox", name: "red" }]);
});

test("replicate rows that swapped their unique names across several commits of one page", async () => {
    const tables = [label, ...replicaTables];
    const source = await open(tables);
    const local = await openReplica(
        defineDatabase({ name: "replica", tables: replicaTables, replicated: [label] }),
    );
    const feed = new Feed(source, [label, replicaPosition], { clock: Clock.system });
    await source.insert(label).values([
        { id: "l1", scope: "inbox", name: "red" },
        { id: "l2", scope: "inbox", name: "blue" },
    ]);

    // replicate the labels until synced
    const labels = new Replica({ name: "work", scope: "inbox", tables: [label] });
    await syncLabels(labels, source, local, feed);

    // swap the names through a third one, one commit each
    await source.update(label).set({ name: "green" }).where(eq(label.id, "l1"));
    await source.update(label).set({ name: "red" }).where(eq(label.id, "l2"));
    await source.update(label).set({ name: "blue" }).where(eq(label.id, "l1"));
    await syncLabels(labels, source, local, feed);

    // retain both rows under their swapped names
    expect(await local.select().from(label).orderBy(label.id)).toEqual([
        { id: "l1", scope: "inbox", name: "blue" },
        { id: "l2", scope: "inbox", name: "red" },
    ]);
});

test("arrive at a position only through a replica that has it", async () => {
    const local = await openReplica();
    const notes = new Replica({ name: "notes", scope: "inbox", tables: [note] });
    const epoch = "01996ab0-0000-7000-8000-000000000001";
    const confirm = () =>
        Replica.wait(local, "inbox", { log: LOG, epoch, sequence: 5 }, AbortSignal.timeout(20));

    // read no position of an unreplicated scope
    expect([await Replica.isReplicated(Snapshot.live(local), "inbox"), await confirm()]).toEqual([
        false,
        false,
    ]);

    // wait until a complete page arrives at the position
    await notes.register(local, { clock: Clock.system });
    expect([await Replica.isReplicated(Snapshot.live(local), "inbox"), await confirm()]).toEqual([
        true,
        false,
    ]);
    await replicate(notes, local, [
        {
            reset: true,
            complete: true,
            changes: [],
            position: { log: LOG, epoch, sequence: 5 },
        },
    ]);
    expect(await confirm()).toBe(true);

    // refuse an older epoch
    await replicate(notes, local, [
        {
            reset: true,
            complete: true,
            changes: [],
            position: { log: LOG, epoch: "01996ab0-0000-7000-8000-000000000002", sequence: 1 },
        },
    ]);
    expect(await refusal(confirm())).toEqual([
        "GONE",
        "STALE_EPOCH",
        "inbox started a new epoch after the position",
    ]);
});

test("arrive at a position through the replicas of its log, skipping a required one following another log", async () => {
    // replicate the inbox from its home's log, and another scope from another log
    const local = await openReplica();
    const inbox = new Replica({ name: "notes", scope: "inbox", tables: [note] });
    const other = new Replica({ name: "notes", scope: "elsewhere", tables: [note] });
    const epoch = "01996ab0-0000-7000-8000-000000000001";
    for (const [replica, log] of [
        [inbox, LOG],
        [other, "01996ab0-0000-7000-8000-0000000000bb"],
    ] as const) {
        await replica.register(local, { clock: Clock.system });
        await replicate(replica, local, [
            { reset: true, complete: true, changes: [], position: { log, epoch, sequence: 5 } },
        ]);
    }

    // arrive at the inbox's position with the other replica required, which follows another log
    const required = [{ name: "notes", scope: "elsewhere" }];
    expect([
        await Replica.isAt(local, "inbox", { log: LOG, epoch, sequence: 5 }, required),
        await Replica.isAt(local, "inbox", { log: LOG, epoch, sequence: 6 }, required),
    ]).toEqual([true, false]);
});

test("arrive at a position of a restored log only once every replica of the log follows its new epoch", async () => {
    // replicate two scopes of one log, the other one still at the epoch before a restore
    const local = await openReplica();
    const inbox = new Replica({ name: "notes", scope: "inbox", tables: [note] });
    const other = new Replica({ name: "notes", scope: "elsewhere", tables: [note] });
    const before = "01996ab0-0000-7000-8000-000000000001";
    const restored = "01996ab0-0000-7000-8000-000000000002";
    const follow = (replica: Replica, epoch: string) =>
        replicate(replica, local, [
            {
                reset: true,
                complete: true,
                changes: [],
                position: { log: LOG, epoch, sequence: 5 },
            },
        ]);
    for (const replica of [inbox, other]) {
        await replica.register(local, { clock: Clock.system });
    }
    await follow(inbox, restored);
    await follow(other, before);

    // wait on the other replica behind the restore, then arrive at the position once it follows
    const required = [{ name: "notes", scope: "elsewhere" }];
    const position = { log: LOG, epoch: restored, sequence: 5 };
    const behind = await Replica.isAt(local, "inbox", position, required);
    await follow(other, restored);
    expect([behind, await Replica.isAt(local, "inbox", position, required)]).toEqual([false, true]);
});

test("stage a run's pages and apply them at once when the run completes", async () => {
    const local = await openReplica();
    const notes = new Replica({ name: "notes", scope: "inbox", tables: [note] });
    const epoch = "01996ab0-0000-7000-8000-000000000001";
    const snapshot: Page[] = [
        {
            reset: true,
            complete: false,
            changes: [noteInsert("a")],
            position: { log: LOG, epoch, sequence: 3 },
        },
        {
            reset: false,
            complete: true,
            changes: [noteInsert("b")],
            position: { log: LOG, epoch, sequence: 4 },
        },
    ];
    const state = async () => [
        (await local.select({ id: note.id }).from(note).orderBy(asc(note.id))).map((row) => row.id),
        await notes.position(local),
    ];
    await local.insert(note).values([
        { ...first, id: "b" },
        { ...first, id: "stale" },
    ]);

    // retain the replica while a run is staged
    const stream = notes.apply(local, snapshot, { clock: Clock.system });
    await stream.next();
    expect(await state()).toEqual([["b", "stale"], undefined]);

    // drop a broken stream's staged run, applying the next stream's snapshot alone
    await stream.return(undefined);
    await replicate(notes, local, [
        {
            reset: true,
            complete: true,
            changes: [noteInsert("c")],
            position: { log: LOG, epoch, sequence: 2 },
        },
    ]);
    expect(await state()).toEqual([["c"], { log: LOG, epoch, sequence: 2 }]);

    // apply the snapshot and prune the left out rows
    await replicate(notes, local, snapshot);
    expect(await state()).toEqual([["a", "b"], { log: LOG, epoch, sequence: 4 }]);
});

test("nest each retained row's measures, whole or per group, in the rows a replica reads", async () => {
    const source = await open();
    const local = await openReplica();
    const feed = new Feed(source, [project, task, replicaPosition], { clock: Clock.system });
    const projects = new Replica({ name: "projects", scope: "inbox", tables: [project, task] });
    await source.insert(project).values([
        { id: "p", scope: "inbox", name: "Plans" },
        { id: "q", scope: "inbox", name: "Quiet" },
    ]);
    await source
        .insert(task)
        .values([taskOf("p", "open", 1), taskOf("p", "open", 4), taskOf("p", "done", 2)]);

    // count tasks per project and state
    const query: Query = {
        table: project,
        scopes: ["inbox"],
        relations,
        orderBy: { name: "asc" },
        with: {
            size: {
                aggregate: {
                    values: {
                        tasks: { function: "count" },
                        top: { function: "max", column: "rank" },
                    },
                },
            },
            states: {
                aggregate: {
                    groupBy: ["state"],
                    values: { points: { function: "sum", column: "points" } },
                },
            },
        },
    };
    const controller = new AbortController();
    const following = projects.follow(
        local,
        (from, signal) => feed.subscribe({ projects: query }, from.after, signal),
        controller.signal,
        { synced: () => {}, clock: Clock.system },
    );
    expect(
        await Replica.wait(local, "inbox", await source.log.position(), AbortSignal.timeout(5000)),
    ).toBe(true);
    controller.abort();
    await following;

    // nest the measures of each project and state
    const { scopes: _routes, ...unrouted } = query;
    const read = await projects.rows(local, "projects", unrouted);
    expect(
        read.map((item) => [
            item.row["name"],
            item.measures["size"],
            byGroup(item.measures["states"] ?? []),
        ]),
    ).toEqual([
        [
            "Plans",
            [{ group: {}, values: { tasks: 3, top: 4 } }],
            [
                { group: { state: "done" }, values: { points: 4 } },
                { group: { state: "open" }, values: { points: 10 } },
            ],
        ],
        ["Quiet", [{ group: {}, values: { tasks: 0, top: null } }], []],
    ]);
});

test("replicate a table's binary column and store nothing for a required column the log leaves out", async () => {
    // follow an asset with binary content and a secret outside the log
    const source = await open([asset, ...replicaTables]);
    const local = await openReplica([asset, ...replicaTables]);
    const feed = new Feed(source, [asset, replicaPosition], { clock: Clock.system });
    const assets = new Replica({ name: "assets", scope: "inbox", tables: [asset] });
    await source
        .insert(asset)
        .values({ id: "a", scope: "inbox", content: new Uint8Array([1]), secret: "key" });
    const controller = new AbortController();
    const following = assets.follow(
        local,
        (from, signal) => feed.subscribe(assets.queries, from.after, signal),
        controller.signal,
        { synced: () => {}, clock: Clock.system },
    );
    const signal = AbortSignal.timeout(5000);
    expect(await Replica.wait(local, "inbox", await source.log.position(), signal)).toBe(true);
    controller.abort();
    await following;

    // store the row with its content and without its secret
    expect(await local.select().from(asset)).toEqual([
        { id: "a", scope: "inbox", content: new Uint8Array([1]), secret: null },
    ]);
});

test("resume a stream the source ends from the replica's position at once, reporting the follow synced once", async () => {
    const source = await open();
    const local = await openReplica();
    const feed = new Feed(source, [note, replicaPosition], { clock: Clock.system });
    const notes = new Replica({ name: "notes", scope: "inbox", tables: [note] });
    await source.insert(note).values(first);

    // end the first two streams after their first completed run, like a lapsed caller
    const requested: (LogPosition | undefined)[] = [];
    const third = Promise.withResolvers<void>();
    const controller = new AbortController();
    let syncs = 0;
    const following = notes.follow(
        local,
        async function* ({ after }, signal) {
            requested.push(after);
            const isEnding = requested.length <= 2;
            if (requested.length === 3) {
                third.resolve();
            }
            for await (const page of feed.subscribe(notes.queries, after, signal)) {
                yield page;
                if (isEnding && page.complete) {
                    return;
                }
            }
        },
        controller.signal,
        {
            synced: () => {
                syncs += 1;
            },
            clock: Clock.system,
        },
    );

    // follow a change through the third stream
    await third.promise;
    const snapshot = await source.log.position();
    await source.update(note).set({ title: "Renamed" }).where(eq(note.id, "a"));
    const signal = AbortSignal.timeout(5000);
    expect(await Replica.wait(local, "inbox", await source.log.position(), signal)).toBe(true);
    controller.abort();
    await following;

    // resume from the snapshot's position, store the renamed note, and report synced at the first stream's complete page only
    expect({
        requested,
        notes: (await local.select().from(note)).map((row) => [row.id, row.title]),
        syncs,
    }).toEqual({ requested: [undefined, snapshot, snapshot], notes: [["a", "Renamed"]], syncs: 1 });
});

test("fail following once the source fails or ends a stream without a page", async () => {
    const local = await openReplica();
    const notes = new Replica({ name: "notes", scope: "inbox", tables: [note] });
    const signal = new AbortController().signal;

    // report a failure within a run and an empty stream
    const failing = notes.follow(
        local,
        async function* () {
            yield {
                reset: true,
                complete: false,
                changes: [],
                position: {
                    log: LOG,
                    epoch: "01996ab0-0000-7000-8000-000000000001",
                    sequence: 1,
                },
            };
            throw new Error("source failed");
        },
        signal,
        { synced: () => {}, clock: Clock.system },
    );
    await expect(failing).rejects.toThrow(new Error("source failed"));
    const empty = notes.follow(
        local,
        async function* () {
            yield* [];
        },
        signal,
        { synced: () => {}, clock: Clock.system },
    );
    await expect(empty).rejects.toEqual(
        new SyncError("INVALID_STREAM", "the source of inbox ended a stream without a page"),
    );
});

test("finish a snapshot of several pages past a drain, then resume from its completed position", async () => {
    const source = await open();
    const local = await openReplica();
    const feed = new Feed(source, [note, replicaPosition], {
        clock: Clock.system,
        pageSize: PAGE_SIZE,
    });
    const notes = new Replica({ name: "notes", scope: "inbox", tables: [note] });
    const ids = Array.from({ length: SNAPSHOT_NOTES }, (_, index) => `n${index}`);
    await source.insert(note).values(ids.map((id) => ({ ...first, id })));

    // drain the first stream at its first page as at a lapse, and end the second after one page
    const requested: (LogPosition | undefined)[] = [];
    const received: (readonly [number, boolean, boolean, number])[] = [];
    const third = Promise.withResolvers<void>();
    const controller = new AbortController();
    const following = notes.follow(
        local,
        async function* ({ after }, signal) {
            requested.push(after);
            const stream = requested.length;
            if (stream === 3) {
                third.resolve();
            }
            const drain = new AbortController();
            for await (const page of feed.subscribe(notes.queries, after, signal, {
                drain: drain.signal,
            })) {
                received.push([stream, page.reset, page.complete, page.changes.length]);
                if (stream === 1) {
                    drain.abort();
                }
                yield page;
                if (stream === 2) {
                    return;
                }
            }
        },
        controller.signal,
        { synced: () => {}, clock: Clock.system },
    );
    const position = await source.log.position();
    await third.promise;
    controller.abort();
    await following;

    // complete the snapshot and resume twice from its position
    expect({
        requested,
        received: received.slice(0, 2),
        notes: (await local.select().from(note)).length,
    }).toEqual({
        requested: [undefined, position, position],
        received: [
            [1, true, false, PAGE_SIZE],
            [1, false, true, SNAPSHOT_NOTES - PAGE_SIZE],
        ],
        notes: SNAPSHOT_NOTES,
    });
});

test("match the rows of the replicas each reader sees by audience, and retain a row two replicas include until neither does", async () => {
    const local = await openReplica();
    const epoch = "01996ab0-0000-7000-8000-000000000001";
    const snapshot = (sequence: number, ids: readonly string[]): Page[] => [
        {
            reset: true,
            complete: true,
            changes: ids.map((id) => noteInsert(id)),
            position: { log: LOG, epoch, sequence },
        },
    ];
    const stored = async () =>
        (await local.select({ id: note.id }).from(note).orderBy(asc(note.id))).map((row) => row.id);

    // include a shared row in a replica for the target's readers and one for alice, and another row in a contained one
    const left = new Replica({ name: "left", scope: "inbox", tables: [note] });
    const right = new Replica({ name: "right", scope: "inbox", tables: [note] });
    const chain = new Replica({ name: "chain", scope: "inbox", tables: [note] });
    const subject = ALICE;
    await left.register(local, {
        subscription: inboxSubscription("left"),
        audience: "reader",
        clock: Clock.system,
    });
    await right.register(local, {
        subscription: { ...inboxSubscription("right"), subject },
        audience: "caller",
        clock: Clock.system,
    });
    await chain.register(local, {
        subscription: inboxSubscription("chain"),
        audience: "contained",
        clock: Clock.system,
    });
    await replicate(left, local, snapshot(1, ["a", "shared"]));
    await replicate(right, local, snapshot(1, ["shared", "z"]));
    await replicate(chain, local, snapshot(1, ["contained"]));

    // match the rows the target's readers see, alice those decided for her too, and other scopes' readers none
    const included = async (target: string, reader: Subject | undefined) =>
        (
            await local
                .select({ id: note.id })
                .from(note)
                .where(Replica.includes(note, target, reader))
                .orderBy(asc(note.id))
        ).map((row) => row.id);
    expect([
        await included("reader", undefined),
        await included("reader", BOB),
        await included("reader", ALICE),
        await included("other", ALICE),
    ]).toEqual([["a", "shared"], ["a", "shared"], ["a", "shared", "z"], []]);

    // retain the shared row while one replica still includes it, and delete it after
    await replicate(left, local, snapshot(2, ["a"]));
    const before = await stored();
    await replicate(right, local, snapshot(2, ["z"]));
    expect([before, await stored()]).toEqual([
        ["a", "contained", "shared", "z"],
        ["a", "contained", "z"],
    ]);
});

test("drop a replica: delete the rows no other replica includes and forget its record", async () => {
    const local = await openReplica(TABLES);
    const epoch = "01996ab0-0000-7000-8000-000000000001";
    const snapshot = (ids: readonly string[]): Page[] => [
        {
            reset: true,
            complete: true,
            changes: ids.map((id) => noteInsert(id)),
            position: { log: LOG, epoch, sequence: 1 },
        },
    ];
    // store a shared row in two replicas
    const left = new Replica({ name: "left", scope: "inbox", tables: [note] });
    const right = new Replica({ name: "right", scope: "inbox", tables: [note] });
    await left.register(local, {
        subscription: inboxSubscription("left"),
        audience: "contained",
        clock: Clock.system,
    });
    await right.register(local, {
        subscription: inboxSubscription("right"),
        audience: "contained",
        clock: Clock.system,
    });
    await replicate(left, local, snapshot(["a", "shared"]));
    await replicate(right, local, snapshot(["shared", "z"]));

    // drop the left replica and retain the right replica's rows and record
    await Replica.drop(local, left);
    expect([
        (await local.select({ id: note.id }).from(note).orderBy(asc(note.id))).map((row) => row.id),
        await Replica.subscriptions(Snapshot.live(local)),
    ]).toEqual([["shared", "z"], [inboxSubscription("right")]]);
});

test("replicate the rows in the scopes of replicated rows, and take them out with the row they are in", async () => {
    const source = await open();
    const local = await openReplica();
    const feed = new Feed(source, [project, task, tag, replicaPosition], {
        clock: Clock.system,
    });
    const work = new Replica({
        name: "work",
        scope: "inbox",
        tables: [project, task, tag],
        within: new Map([[tag, [project, task]]]),
    });
    await source.insert(project).values([
        { id: "p1", scope: "inbox", name: "Plan" },
        { id: "p2", scope: "archive", name: "Old" },
    ]);
    await source.insert(tag).values([
        { id: "g1", scope: "p1", name: "urgent" },
        { id: "g2", scope: "p2", name: "done" },
        { id: "g3", scope: "inbox", name: "loose" },
        { id: "g6", scope: "t1", name: "own" },
    ]);
    await source.insert(task).values({
        id: "t1",
        scope: "inbox",
        state: "open",
        rank: 1,
        isSecret: false,
    });
    const signal = AbortSignal.timeout(5000);
    const tags = async () => {
        expect(await Replica.wait(local, "inbox", await source.log.position(), signal)).toBe(true);

        return (await local.select().from(tag).orderBy(asc(tag.id))).map((row) => [
            row.id,
            row.scope,
        ]);
    };

    // replicate the tags only in the scopes of the replicated project and task
    const controller = new AbortController();
    const following = work.follow(
        local,
        (from, stream) => feed.subscribe(work.queries, from.after, stream),
        controller.signal,
        { synced: () => {}, clock: Clock.system },
    );
    expect(await tags()).toEqual([
        ["g1", "p1"],
        ["g6", "t1"],
    ]);

    // add a tag entering a replicated project's scope, and the tags of a project entering the replica
    await source.insert(tag).values({ id: "g4", scope: "p1", name: "later" });
    await source.update(project).set({ scope: "inbox" }).where(eq(project.id, "p2"));
    expect(await tags()).toEqual([
        ["g1", "p1"],
        ["g2", "p2"],
        ["g4", "p1"],
        ["g6", "t1"],
    ]);

    // take a project's tags out with it
    await source.delete(project).where(eq(project.id, "p1"));
    expect(await tags()).toEqual([
        ["g2", "p2"],
        ["g6", "t1"],
    ]);
    controller.abort();
    await following;

    // take out the tags a later snapshot leaves out
    await source.delete(tag).where(eq(tag.id, "g2"));
    await source.insert(tag).values({ id: "g5", scope: "p2", name: "retained" });
    const again = new AbortController();
    const snapshotting = work.follow(
        local,
        (_from, stream) => feed.subscribe(work.queries, undefined, stream),
        again.signal,
        { synced: () => {}, clock: Clock.system },
    );
    expect(await tags()).toEqual([
        ["g5", "p2"],
        ["g6", "t1"],
    ]);
    again.abort();
    await snapshotting;
});

test("retain a column one replica hides while another shows it, and clear it once none does", async () => {
    const local = await openReplica();
    const epoch = "01996ab0-0000-7000-8000-000000000001";
    const snapshot = (sequence: number, subtitle: string | null, concealed: string[]): Page[] => [
        {
            reset: true,
            complete: true,
            changes: [
                {
                    table: note[TABLE].sqlName,
                    operation: "insert" as const,
                    row: note[TABLE].encode({ ...first, subtitle }),
                    ...(concealed.length === 0 ? {} : { concealed }),
                },
            ],
            position: { log: LOG, epoch, sequence },
        },
    ];
    const empty = (sequence: number): Page[] => [
        { reset: true, complete: true, changes: [], position: { log: LOG, epoch, sequence } },
    ];
    const subtitle = async () =>
        (await local.select({ subtitle: note.subtitle }).from(note)).map((row) => row.subtitle);
    const shown = new Replica({ name: "open", scope: "inbox", tables: [note] });
    const closed = new Replica({ name: "closed", scope: "inbox", tables: [note] });

    // show the subtitle through one replica, and retain it as the other hides it
    const stored = [];
    await replicate(closed, local, snapshot(1, null, ["subtitle"]));
    stored.push(await subtitle());
    await replicate(shown, local, snapshot(1, "Plan", []));
    stored.push(await subtitle());
    await replicate(closed, local, snapshot(2, null, ["subtitle"]));
    stored.push(await subtitle());

    // clear the subtitle once the replica showing it leaves the row to the one hiding it
    await replicate(shown, local, empty(2));
    stored.push(await subtitle());
    expect(stored).toEqual([[null], ["Plan"], ["Plan"], [null]]);
});

/** An entry of any scope with sensitive and binary columns, which its log leaves out. */
const entry = defineTable(
    "entry",
    {
        id: text("id").primaryKey(),
        scope: text("scope").notNull(),
        title: text("title").notNull(),
        secret: text("secret").sensitive(),
        data: binary("data"),
    },
    { log: {} },
);

/** The tables of a space a capture reads, as many as a stack's space stores. */
const SPACE_TABLES: readonly Table[] = Array.from({ length: 40 }, (_, index) =>
    defineTable(
        `space_${index}`,
        {
            id: text("id").primaryKey(),
            scope: text("scope").notNull(),
            title: text("title").notNull(),
        },
        { log: {} },
    ),
);

/** An unlogged cache, which only a capture replicates. */
const cache = defineTable("cache", {
    id: text("id").primaryKey(),
    value: text("value").notNull(),
});

/** A key wrapped under the root key of the machine that stores it. */
const machineKey = defineTable(
    "machine_key",
    {
        id: text("id").primaryKey(),
        scope: text("scope").notNull(),
        wrapped: text("wrapped").notNull(),
    },
    { log: {} },
);

/** Build some rows of a space's table. */
function rowsOf(table: Table, count: number) {
    return Array.from({ length: count }, (_, index) => ({
        id: `${table[TABLE].sqlName}-${index}`,
        scope: "space",
        title: "Row",
    }));
}

/** Rewrap a machine key row's wrapped value from one machine to another. */
function rewrapped(table: Table, row: Row, from: string, to: string): Row {
    // leave rows of other tables
    if (table !== machineKey) {
        return row;
    }

    // replace the machine prefix of the wrapped value
    const wrapped = row["wrapped"];
    if (typeof wrapped !== "string") {
        throw new TypeError("a machine key row has no wrapped value");
    }

    return { ...row, wrapped: wrapped.replace(from, to) };
}

/** Build the change inserting a note. */
function noteInsert(id: string) {
    return {
        table: note[TABLE].sqlName,
        operation: "insert" as const,
        row: note[TABLE].encode({ ...first, id }),
    };
}

/** Build the subscription a named replica of the inbox follows. */
function inboxSubscription(name: string) {
    return { name, publication: "notes", scope: "inbox", target: "reader", parameters: {} };
}

/** Build a task of a project in a state. */
function taskOf(projectId: string, state: string, rank: number) {
    return {
        id: `${projectId}-${state}-${rank}`,
        scope: "inbox",
        projectId,
        state,
        rank,
        points: rank * 2,
        title: null,
        isSecret: false,
    };
}

/** Sort an item's groups by their group values. */
function byGroup(groups: readonly AggregateRow[]): AggregateRow[] {
    return groups.toSorted((left, right) =>
        JSON.stringify(left.group) < JSON.stringify(right.group) ? -1 : 1,
    );
}

test("resume a subscription from the replica's position, reshape one of other parameters from those the replica reflects, and snapshot another", async () => {
    const local = await openReplica();
    const notes = new Replica({ name: "notes", scope: "inbox", tables: [note] });
    const subscription = {
        name: "notes",
        publication: "queries",
        scope: "inbox",
        target: "inbox",
        parameters: { queries: { open: { object: "note" } } },
    };
    const position = { log: LOG, epoch: "01996ab0-0000-7000-8000-000000000001", sequence: 4 };

    // complete a snapshot of the subscription
    await notes.register(local, { clock: Clock.system });
    const snapshot: Page = { reset: true, complete: true, changes: [], position };
    await Array.fromAsync(
        notes.apply(local, [snapshot], {
            clock: Clock.system,
            subscription,
            audience: "contained",
        }),
    );

    // resume it, reshape it under other parameters, and snapshot another publication or name, each with the replica's log
    const reshaped = { ...subscription, parameters: { queries: { all: { object: "note" } } } };
    const origin = (await local.log.position()).log;
    expect([
        await notes.resume(local, { ...subscription, after: { ...position, sequence: 1 } }),
        await notes.resume(local, reshaped),
        await notes.resume(local, { ...subscription, publication: "chain" }),
        await notes.resume(local),
    ]).toEqual([
        { after: position, origin },
        { after: position, previous: subscription.parameters, origin },
        { origin },
        { after: position, origin },
    ]);
});

test("read when the home last confirmed a replica's rows, absent before its first page, renewed by each later page", async () => {
    const local = await openReplica();
    const notes = new Replica({ name: "notes", scope: "inbox", tables: [note] });
    const subscription = {
        name: "notes",
        publication: "queries",
        scope: "inbox",
        target: "inbox",
        parameters: { queries: { open: { object: "note" } } },
    };
    const clock = new VirtualClock(1000);
    const page = (sequence: number): Page => ({
        reset: sequence === 4,
        complete: true,
        changes: [],
        position: { log: LOG, epoch: "01996ab0-0000-7000-8000-000000000001", sequence },
    });
    const apply = (sequence: number) =>
        Array.fromAsync(
            notes.apply(local, [page(sequence)], { clock, subscription, audience: "contained" }),
        );
    const confirmed = () => Replica.confirmedAt(local, subscription);

    // confirm at the snapshot's time, then renew with a later page
    await notes.register(local, { clock });
    const registered = await confirmed();
    await apply(4);
    const atSnapshot = await confirmed();
    clock.advance(30_000);
    await apply(5);
    expect({ registered, atSnapshot, renewed: await confirmed() }).toEqual({
        registered: undefined,
        atSnapshot: 1000,
        renewed: 31_000,
    });
});

test("read a replica's confirmation for the request it completed a run of, and for a reshaped request only once a run of it completes", async () => {
    const local = await openReplica();
    const notes = new Replica({ name: "notes", scope: "inbox", tables: [note] });
    const subscription = {
        name: "notes",
        publication: "queries",
        scope: "inbox",
        target: "inbox",
        parameters: { queries: { open: { object: "note" } } },
    };
    const reshaped = { ...subscription, parameters: { queries: { all: { object: "note" } } } };
    const position = { log: LOG, epoch: "01996ab0-0000-7000-8000-000000000001", sequence: 4 };
    const snapshot: Page = { reset: true, complete: true, changes: [], position };
    const current = async () => [
        (await Replica.confirmedAt(local, subscription)) !== undefined,
        (await Replica.confirmedAt(local, reshaped)) !== undefined,
    ];

    // report neither before the first run, then only the completed request
    await notes.register(local, { clock: Clock.system });
    const registered = await current();
    await Array.fromAsync(
        notes.apply(local, [snapshot], {
            clock: Clock.system,
            subscription,
            audience: "contained",
        }),
    );
    const completed = await current();

    // report the reshaped request once a run of it completes, and no longer the first
    const next = { ...snapshot, position: { ...position, sequence: 5 } };
    await Array.fromAsync(
        notes.apply(local, [next], {
            clock: Clock.system,
            subscription: reshaped,
            audience: "contained",
        }),
    );
    expect({ registered, completed, reshaped: await current() }).toEqual({
        registered: [false, false],
        completed: [true, false],
        reshaped: [false, true],
    });
});

/** Follow a replica of the inbox's labels until it arrives at the source's position. */
async function syncLabels(
    labels: Replica,
    source: DatabaseConnection,
    local: DatabaseConnection,
    feed: Feed,
): Promise<void> {
    // follow the replica's queries from the feed until it arrives at the source's position
    const controller = new AbortController();
    const following = labels.follow(
        local,
        (from, signal) => feed.subscribe(labels.queries, from.after, signal),
        controller.signal,
        { synced: () => {}, clock: Clock.system },
    );
    const isSynced = await Replica.wait(
        local,
        labels.scope,
        await source.log.position(),
        AbortSignal.timeout(5000),
    );
    controller.abort();
    await following;
    expect(isSynced).toBe(true);
}

/** Units with a predefined row every database stores. */
const unit = defineTable(
    "unit",
    {
        id: text("id").primaryKey(),
        scope: text("scope").notNull(),
        name: text("name").notNull(),
    },
    { log: {}, predefined: [{ id: "metre", scope: "inbox", name: "Metre" }] },
);

test("leave a predefined row out of a replica, which retains it after dropping the replica", async () => {
    const source = await open([unit, ...replicaTables]);
    const local = await openReplica(
        defineDatabase({ name: "replica", tables: replicaTables, replicated: [unit] }),
    );
    const feed = new Feed(source, [unit, replicaPosition], { clock: Clock.system });
    await source.insert(unit).values({ id: "foot", scope: "inbox", name: "Foot" });

    // replicate the units, including only the written one
    const units = new Replica({ name: "work", scope: "inbox", tables: [unit] });
    await syncLabels(units, source, local, feed);
    const included = await local.select({ key: replicaKey.key }).from(replicaKey);
    const replicated = await local.select().from(unit).orderBy(asc(unit.id));

    // drop the replica, which deletes the written unit and retains the predefined one
    await Replica.drop(local, units);
    expect({
        included: included.map((row) => row.key),
        replicated,
        stored: await local.select().from(unit),
    }).toEqual({
        included: [Key.name(unit, { id: "foot" })],
        replicated: [
            { id: "foot", scope: "inbox", name: "Foot" },
            { id: "metre", scope: "inbox", name: "Metre" },
        ],
        stored: [{ id: "metre", scope: "inbox", name: "Metre" }],
    });
});

test("drop a replica without its tables, with the rows of every table its record lists", async () => {
    const source = await open([unit, ...replicaTables]);
    const local = await openReplica(
        defineDatabase({ name: "replica", tables: replicaTables, replicated: [unit] }),
    );
    const feed = new Feed(source, [unit, replicaPosition], { clock: Clock.system });
    await source.insert(unit).values({ id: "foot", scope: "inbox", name: "Foot" });
    const units = new Replica({ name: "work", scope: "inbox", tables: [unit] });
    await syncLabels(units, source, local, feed);

    // drop it without its tables, retaining the predefined unit
    await Replica.drop(local, { name: "work", scope: "inbox", tables: [] });
    expect({
        stored: await local.select().from(unit),
        held: await local.select().from(replicaPosition),
        isReplicated: await Replica.isReplicated(Snapshot.live(local), "inbox"),
    }).toEqual({
        stored: [{ id: "metre", scope: "inbox", name: "Metre" }],
        held: [],
        isReplicated: false,
    });
});

/** Follow a replica from a feed until aborted, confirming the source's head on each wait. */
function followConfirmed(
    replica: Replica,
    local: DatabaseConnection,
    source: DatabaseConnection,
    feed: Feed,
    signal: AbortSignal,
) {
    // follow the feed, recording the stream it opens
    let stream: Stream | undefined;
    const following = replica.follow(
        local,
        (from, subscribed) =>
            feed.subscribe(replica.queries, from.after, subscribed, {
                opened: (opened) => {
                    stream = opened;
                },
            }),
        signal,
        { synced: () => {}, clock: Clock.system },
    );

    // ask the stream to confirm the source's head, and wait for the replica to arrive at it
    const confirmed = async () => {
        const position = await source.log.position();
        stream?.confirm(position);

        return Replica.wait(local, replica.scope, position, AbortSignal.timeout(5000));
    };

    return { following, confirmed };
}

/** Follow a replica's queries from a feed. */
function followFrom(feed: Feed, replica: Replica) {
    return (from: Resumption, signal: AbortSignal) =>
        feed.subscribe(replica.queries, from.after, signal);
}

/** List the identifiers each page changes. */
function changedIds(pages: readonly Page[]): unknown[][] {
    return pages.map((page) => page.changes.map((change) => change.row["id"]));
}

/** Follow a replica's queries from a feed as its database does, recording each page it receives. */
function recordFrom(feed: Feed, replica: Replica, received: Page[]) {
    return async function* (from: Resumption, signal: AbortSignal): AsyncGenerator<Page> {
        for await (const page of followFrom(feed, replica)(from, signal)) {
            received.push(page);
            yield page;
        }
    };
}

/** Links from one row to another of any scope, as subject sets point at sets. */
const link = defineTable(
    "replica_link",
    {
        /** The link's key. */
        id: text("id").primaryKey(),
        /** The scope the link lives in. */
        scope: text("scope").notNull(),
        /** The link it points at, absent for an end. */
        target: text("target"),
    },
    { log: {} },
);

/** A link's targets: the links whose key it points at. */
const TARGETS: Relation = {
    table: link,
    cardinality: "many",
    on: { kind: "key", column: "id", parent: "target" },
};

test("include the links two steps from the links a replica stores in its scope, and follow a relinked one to its new targets", async () => {
    // chain a scope's links, beside one more step and a spare
    const tables = [link, ...replicaTables];
    const source = await open(tables);
    const local = await openReplica(tables);
    await source.insert(link).values([
        { id: "a1", scope: "a", target: "a2" },
        { id: "a2", scope: "a", target: "a3" },
        { id: "a3", scope: "a", target: "a4" },
        { id: "a4", scope: "a", target: null },
        { id: "a5", scope: "a", target: null },
    ]);

    // replicate the first link with its targets two steps deep
    const feed = new Feed(source, [link, replicaPosition], { clock: Clock.system });
    const links = new Replica({
        name: "links",
        scope: "a",
        tables: [link],
        where: new Map([[link, { id: "a1" }]]),
        includes: new Map([[link, { targets: { with: { targets: true } } }]]),
        relations: new Map([[link, { targets: TARGETS }]]),
    });
    const controller = new AbortController();
    const { following, confirmed } = followConfirmed(links, local, source, feed, controller.signal);
    const linkedRows = async () =>
        (await local.select({ id: link.id }).from(link).orderBy(asc(link.id))).map((row) => row.id);
    await confirmed();
    const linked = await linkedRows();

    // point a2 at a5, retaining a5 and letting a3 go
    await source.update(link).set({ target: "a5" }).where(eq(link.id, "a2"));
    await confirmed();
    const relinked = await linkedRows();
    controller.abort();
    await following;
    expect({ linked, relinked }).toEqual({
        linked: ["a1", "a2", "a3"],
        relinked: ["a1", "a2", "a5"],
    });
});
