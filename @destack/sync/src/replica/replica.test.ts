import { expect, test } from "@destack/test";
import { TEST_DIALECTS } from "@destack/db/test";
import {
    and,
    asc,
    binary,
    defineTable,
    eq,
    inArray,
    integer,
    Key,
    TABLE,
    text,
    uniqueIndex,
    type Row,
    type Table,
    type LogPosition,
    type DatabaseConnection,
} from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { setTimeout } from "node:timers/promises";
import { onTestFinished } from "@destack/test";
import { Feed } from "../feed/feed.ts";
import { schema } from "@destack/schema";
import { replicaTables, Replica, replica, replicaRow, type Projector } from "./replica.ts";
import { type Resumption, Uplink } from "./shape.ts";
import {
    asset,
    first,
    note,
    open,
    openCopy,
    project,
    relations,
    replicate,
    tag,
    task,
    TABLES,
} from "../test/fixture.ts";
import type { AggregateRow, Query } from "../query/query.ts";
import type { Page } from "../query/page.ts";
import { PAGE_ROWS } from "../dataflow/selection.ts";
import { SyncError } from "../error/error.ts";
import { Scope } from "../scope/scope.ts";

/** The notes of a snapshot spanning two pages. */
const SNAPSHOT_NOTES = PAGE_ROWS + 500;

/** The rows one insert writes, within every dialect's parameter limit. */
const INSERT_BATCH = 500;

test.for(TEST_DIALECTS)(
    "follow a scope of another database into a local copy, then own it once promoted, on %s",
    async (dialect) => {
        const source = await open(dialect);
        const copy = await openCopy(dialect);
        const feed = new Feed(source, [note, replica]);
        const notes = new Replica({ name: "notes", scope: "inbox", tables: [note] });
        await source.insert(note).values([first, { ...first, id: "b", scope: "archive" }]);
        await copy.insert(note).values({ ...first, id: "stale" });

        // follow until the copy has the renamed note
        const controller = new AbortController();
        const following = notes.follow(
            copy,
            (from, signal) => feed.subscribe(notes.queries, from.after, signal),
            controller.signal,
        );
        const signal = AbortSignal.timeout(5000);
        expect(await Replica.wait(copy, "inbox", await source.log.position(), signal)).toBe(true);

        // record the copy synced, keeping the scope's notes and no other table
        expect([
            await Replica.isSynced(copy, "notes", "inbox"),
            await Replica.isSynced(copy, "notes", "archive"),
            await Replica.isCopied(copy, "inbox", note),
            await Replica.isCopied(copy, "inbox", task),
            await Replica.scopes(copy, note),
            await Replica.scopes(copy, task),
        ]).toEqual([true, false, true, false, ["inbox"], []]);
        await source.update(note).set({ title: "Renamed" }).where(eq(note.id, "a"));
        await source.delete(note).where(eq(note.id, "b"));
        expect(await Replica.wait(copy, "inbox", await source.log.position(), signal)).toBe(true);
        controller.abort();
        await following;

        // keep the renamed rows and prune the rest
        expect((await copy.select().from(note)).map((row) => [row.id, row.title])).toEqual([
            ["a", "Renamed"],
        ]);
        expect(await notes.position(copy)).toEqual(await source.log.position());

        // keep the rows once promoted
        await notes.promote(copy);
        expect([
            await Replica.isCopied(copy, "inbox"),
            (await copy.select().from(note)).map((row) => row.id),
        ]).toEqual([false, ["a"]]);
    },
);

test.for(TEST_DIALECTS)(
    "follow two databases each copying the other's table, reaching each other's rows and going quiet once neither sends the other bare positions of its own replicated writes, on %s",
    async (dialect) => {
        // keep the notes on one database and the projects on the other
        const home = await open(dialect);
        const other = await open(dialect);
        await home.insert(note).values(first);
        await other.insert(project).values({ id: "p", scope: "inbox", name: "Launch" });

        // follow each database's table from the other, naming the follower's own origin
        const notes = new Replica({ name: "notes", scope: "inbox", tables: [note] });
        const projects = new Replica({ name: "projects", scope: "inbox", tables: [project] });
        const homeFeed = new Feed(home, [note, replica]);
        const otherFeed = new Feed(other, [project, replica]);
        const controller = new AbortController();
        const following = Promise.all([
            notes.follow(other, followOf(homeFeed, notes), controller.signal),
            projects.follow(home, followOf(otherFeed, projects), controller.signal),
        ]);

        // reach the latest change of each database the other did not originate
        const signal = AbortSignal.timeout(5000);
        const reach = async (copy: DatabaseConnection, source: DatabaseConnection) =>
            Replica.wait(copy, "inbox", await source.log.position(await copy.log.epoch()), signal);
        expect([await reach(other, home), await reach(home, other)]).toEqual([true, true]);

        // go quiet once the last records of the reached positions commit
        await setTimeout(100);
        const settled = [await home.log.position(), await other.log.position()];
        await setTimeout(300);
        expect([await home.log.position(), await other.log.position()]).toEqual(settled);
        controller.abort();
        await following;
        expect([
            (await other.select().from(note)).map((row) => row.id),
            (await home.select().from(project)).map((row) => row.id),
        ]).toEqual([["a"], ["p"]]);
    },
);

test.for(TEST_DIALECTS)(
    "refuse publishing a copied table back to the database it copies from, with rows or without, and publish it on to any other, on %s",
    async (dialect) => {
        // copy the inbox's notes from one database into another
        const home = await open(dialect);
        const other = await open(dialect);
        await home.insert(note).values(first);
        const inbox = new Replica({ name: "inbox", scope: "inbox", tables: [note] });
        const controller = new AbortController();
        const following = inbox.follow(
            other,
            followOf(new Feed(home, [note, replica]), inbox),
            controller.signal,
        );
        const signal = AbortSignal.timeout(5000);
        expect(await Replica.wait(other, "inbox", await home.log.position(), signal)).toBe(true);
        controller.abort();
        await following;

        // publish the copy on to a third database, never back to the notes' home
        const homeOrigin = await home.log.epoch();
        const archive = new Replica({ name: "archive", scope: "archive", tables: [note] });
        await expect(inbox.requireAcyclic(other, "third")).resolves.toBeUndefined();
        await expect(archive.requireAcyclic(other, homeOrigin)).rejects.toEqual(
            new SyncError(
                "CYCLE",
                `copy archive of archive would publish note back to ${homeOrigin}, its source in copy inbox of inbox`,
            ),
        );

        // refuse publishing the notes back once the copy from their home holds none of them
        await other.delete(note);
        await other.delete(replicaRow);
        await expect(archive.requireAcyclic(other, homeOrigin)).rejects.toEqual(
            new SyncError(
                "CYCLE",
                `copy archive of archive would publish note back to ${homeOrigin}, its source in copy inbox of inbox`,
            ),
        );
    },
);

test.for(TEST_DIALECTS)(
    "copy several scopes in one copy recorded elsewhere, which keeps each scope whose own row it includes, on %s",
    async (dialect) => {
        // keep the inbox's and the archive's rows with their scope rows, and a note of the drafts
        const tables = [Scope.table, ...TABLES];
        const source = await open(dialect, tables);
        const copy = await openCopy(dialect, tables);
        const scopes = ["inbox", "archive"];
        await source.insert(Scope.table).values(
            scopes.map((scope) => ({
                scope,
                parent: Scope.universe.id,
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

        // follow both scopes in one copy recorded at the universe
        const folders = new Replica({
            name: "folders",
            scope: Scope.universe.id,
            tables: [Scope.table, note],
            scopes: new Map<Table, readonly string[]>([
                [Scope.table, scopes],
                [note, scopes],
            ]),
        });
        const feed = new Feed(source, [Scope.table, note, replica]);
        const subscription = { ...subscriptionOf("folders"), scope: Scope.universe.id };
        const controller = new AbortController();
        const following = folders.follow(
            copy,
            (from, signal) => feed.subscribe(folders.queries, from.after, signal),
            controller.signal,
            { subscription },
        );
        const head = await source.log.position();
        const signal = AbortSignal.timeout(5000);
        const reached = [
            await Replica.wait(copy, "inbox", head, signal),
            await Replica.wait(copy, "archive", head, signal),
        ];
        controller.abort();
        await following;

        // keep both scopes' notes, and find the one record as the copy of each scope whose row it includes
        const origins = await Replica.origins(copy, "notes", [...scopes, "drafts", "universe"]);
        expect([
            reached,
            (await copy.select({ id: note.id }).from(note).orderBy(asc(note.id))).map(
                (row) => row.id,
            ),
            [...origins.keys()].toSorted(),
            [...origins.values()].map((origin) => origin.position),
        ]).toEqual([
            [true, true],
            ["a", "b"],
            ["archive", "inbox", "universe"],
            [head, head, head],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "copy a whole database: follow its logged rows, then capture every column once its source stops writing, on %s",
    async (dialect) => {
        // keep entries of several scopes with sensitive and binary columns, an unlogged cache and a host-bound key
        const tables = [entry, cache, hostKey, ...replicaTables];
        const source = (await TestDatabase.create(dialect, tables, { isMigrated: true })).database;
        const target = (
            await TestDatabase.create(dialect, tables, { isMigrated: true, isReplica: true })
        ).database;
        onTestFinished(async () => {
            await source.close();
            await target.close();
        });
        const copied = [entry, cache, hostKey];
        const copy = new Replica({
            name: "database",
            scope: "database",
            tables: copied,
            everywhere: new Set(copied),
        });
        const feed = new Feed(source, [entry, hostKey, replica]);
        const data = new Uint8Array([1, 2, 3]);
        await source
            .insert(entry)
            .values({ id: "a", scope: "inbox", title: "First", secret: "s1", data });

        // follow the logged rows while the source writes
        const controller = new AbortController();
        const following = copy.follow(
            target,
            (from, signal) => feed.subscribe(copy.queries, from.after, signal),
            controller.signal,
        );
        const signal = AbortSignal.timeout(5000);
        expect(await Replica.wait(target, "database", await source.log.position(), signal)).toBe(
            true,
        );
        await source
            .insert(entry)
            .values({ id: "b", scope: "archive", title: "Second", secret: "s2", data });
        await source.insert(cache).values({ id: "c", value: "cached" });
        await source.insert(hostKey).values({ id: "k", scope: "inbox", wrapped: "source:k" });
        expect(await Replica.wait(target, "database", await source.log.position(), signal)).toBe(
            true,
        );
        controller.abort();
        await following;

        // capture every column once the source stops writing
        const seal = async (table: Table, row: Row) => rewrapped(table, row, "source:", "moving:");
        const unseal = async (table: Table, row: Row) =>
            rewrapped(table, row, "moving:", "target:");
        const applied = copy.apply(target, feed.capture(copy.captured, signal, { seal }), {
            open: unseal,
        });
        while ((await applied.next()).done !== true) {
            // apply each captured page
        }
        await copy.promote(target);

        // keep exactly the source's rows with the key rewrapped for the target
        const read = async (database: typeof source) => [
            await database.select().from(entry).orderBy(asc(entry.id)),
            await database.select().from(cache),
            await database.select().from(hostKey),
        ];
        expect(await read(target)).toEqual([
            (await read(source))[0],
            [{ id: "c", value: "cached" }],
            [{ id: "k", scope: "inbox", wrapped: "target:k" }],
        ]);
        expect(await Replica.isCopied(target, "database")).toBe(false);
    },
);

test.for(TEST_DIALECTS)(
    "snapshot again once the copied tables change shape on %s",
    async (dialect) => {
        const source = await open(dialect);
        const copy = await openCopy(dialect);
        const feed = new Feed(source, [project, task, replica]);
        const projects = new Replica({ name: "work", scope: "inbox", tables: [project] });
        await source.insert(project).values({ id: "p1", scope: "inbox", name: "Plan" });

        // copy projects, and projects with tasks
        const controller = new AbortController();
        const following = projects.follow(
            copy,
            (from, signal) => feed.subscribe(projects.queries, from.after, signal),
            controller.signal,
        );
        const signal = AbortSignal.timeout(5000);
        expect(await Replica.wait(copy, "inbox", await source.log.position(), signal)).toBe(true);
        controller.abort();
        await following;
        const work = new Replica({ name: "work", scope: "inbox", tables: [project, task] });

        // keep a position per shape
        expect([await projects.position(copy), await work.position(copy)]).toEqual([
            await source.log.position(),
            undefined,
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "reach a position only through a copy that has it on %s",
    async (dialect) => {
        const copy = await openCopy(dialect);
        const notes = new Replica({ name: "notes", scope: "inbox", tables: [note] });
        const epoch = "01996ab0-0000-7000-8000-000000000001";
        const reach = () =>
            Replica.wait(copy, "inbox", { epoch, sequence: 5 }, AbortSignal.timeout(20));

        // reach no position of an uncopied scope
        expect([await Replica.isCopied(copy, "inbox"), await reach()]).toEqual([false, false]);

        // wait until a complete page reaches the position
        await notes.register(copy);
        expect([await Replica.isCopied(copy, "inbox"), await reach()]).toEqual([true, false]);
        await replicate(notes, copy, [
            {
                reset: true,
                complete: true,
                changes: [],
                position: { epoch, sequence: 5 },
            },
        ]);
        expect(await reach()).toBe(true);

        // refuse an older epoch
        await replicate(notes, copy, [
            {
                reset: true,
                complete: true,
                changes: [],
                position: { epoch: "01996ab0-0000-7000-8000-000000000002", sequence: 1 },
            },
        ]);
        await expect(reach()).rejects.toMatchObject({ code: "STALE_EPOCH" });
    },
);

test.for(TEST_DIALECTS)(
    "stage a run's pages and apply them at once when the run completes on %s",
    async (dialect) => {
        const copy = await openCopy(dialect);
        const notes = new Replica({ name: "notes", scope: "inbox", tables: [note] });
        const epoch = "01996ab0-0000-7000-8000-000000000001";
        const snapshot: Page[] = [
            {
                reset: true,
                complete: false,
                changes: [insertOf("a")],
                position: { epoch, sequence: 3 },
            },
            {
                reset: false,
                complete: true,
                changes: [insertOf("b")],
                position: { epoch, sequence: 4 },
            },
        ];
        const state = async () => [
            (await copy.select({ id: note.id }).from(note).orderBy(asc(note.id))).map(
                (row) => row.id,
            ),
            await notes.position(copy),
        ];
        await copy.insert(note).values([
            { ...first, id: "b" },
            { ...first, id: "stale" },
        ]);

        // keep the copy while a run is staged
        const stream = notes.apply(copy, snapshot);
        await stream.next();
        expect(await state()).toEqual([["b", "stale"], undefined]);

        // drop a broken stream's staged run
        await stream.return(undefined);
        await replicate(notes, copy, [
            {
                reset: false,
                complete: true,
                changes: [insertOf("c")],
                position: { epoch, sequence: 2 },
            },
        ]);
        expect(await state()).toEqual([["b", "c", "stale"], { epoch, sequence: 2 }]);

        // apply the snapshot and prune the left out rows
        await replicate(notes, copy, snapshot);
        expect(await state()).toEqual([["a", "b"], { epoch, sequence: 4 }]);
    },
);

test.for(TEST_DIALECTS)(
    "nest each kept row's measures, whole or per group, in the rows a copy reads on %s",
    async (dialect) => {
        const source = await open(dialect);
        const copy = await openCopy(dialect);
        const feed = new Feed(source, [project, task, replica]);
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
            copy,
            (from, signal) => feed.subscribe({ projects: query }, from.after, signal),
            controller.signal,
        );
        expect(
            await Replica.wait(
                copy,
                "inbox",
                await source.log.position(),
                AbortSignal.timeout(5000),
            ),
        ).toBe(true);
        controller.abort();
        await following;

        // nest the measures of each project and state
        const { scopes: _routes, ...local } = query;
        const read = await projects.rows(copy, "projects", local);
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
    },
);

test.for(TEST_DIALECTS)(
    "copy a table's binary column and keep nothing for a required column the log leaves out, on %s",
    async (dialect) => {
        // follow an asset with binary content and a secret outside the log
        const source = await open(dialect, [asset, ...replicaTables]);
        const copy = await openCopy(dialect, [asset, ...replicaTables]);
        const feed = new Feed(source, [asset, replica]);
        const assets = new Replica({ name: "assets", scope: "inbox", tables: [asset] });
        await source
            .insert(asset)
            .values({ id: "a", scope: "inbox", content: new Uint8Array([1]), secret: "key" });
        const controller = new AbortController();
        const following = assets.follow(
            copy,
            (from, signal) => feed.subscribe(assets.queries, from.after, signal),
            controller.signal,
        );
        const signal = AbortSignal.timeout(5000);
        expect(await Replica.wait(copy, "inbox", await source.log.position(), signal)).toBe(true);
        controller.abort();
        await following;

        // keep the row with its content and without its secret
        expect(await copy.select().from(asset)).toEqual([
            { id: "a", scope: "inbox", content: new Uint8Array([1]), secret: null },
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "resume a stream the source ends from the copy's position at once on %s",
    async (dialect) => {
        const source = await open(dialect);
        const copy = await openCopy(dialect);
        const feed = new Feed(source, [note, replica]);
        const notes = new Replica({ name: "notes", scope: "inbox", tables: [note] });
        await source.insert(note).values(first);

        // end the first two streams after their first completed run, like a lapsed caller
        const requested: (LogPosition | undefined)[] = [];
        const third = Promise.withResolvers<void>();
        const controller = new AbortController();
        const following = notes.follow(
            copy,
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
        );

        // follow a change through the third stream
        await third.promise;
        const snapshot = await source.log.position();
        await source.update(note).set({ title: "Renamed" }).where(eq(note.id, "a"));
        const signal = AbortSignal.timeout(5000);
        expect(await Replica.wait(copy, "inbox", await source.log.position(), signal)).toBe(true);
        controller.abort();
        await following;

        // resume from the snapshot's position and keep the renamed note
        expect({
            requested,
            notes: (await copy.select().from(note)).map((row) => [row.id, row.title]),
        }).toEqual({ requested: [undefined, snapshot, snapshot], notes: [["a", "Renamed"]] });
    },
);

test.for(TEST_DIALECTS)(
    "fail following once the source fails or ends a stream without a page on %s",
    async (dialect) => {
        const copy = await openCopy(dialect);
        const notes = new Replica({ name: "notes", scope: "inbox", tables: [note] });
        const signal = new AbortController().signal;

        // report a failure within a run and an empty stream
        const failing = notes.follow(
            copy,
            async function* () {
                yield {
                    reset: true,
                    complete: false,
                    changes: [],
                    position: { epoch: "01996ab0-0000-7000-8000-000000000001", sequence: 1 },
                };
                throw new Error("source failed");
            },
            signal,
        );
        await expect(failing).rejects.toThrow(new Error("source failed"));
        const empty = notes.follow(
            copy,
            async function* () {
                yield* [];
            },
            signal,
        );
        await expect(empty).rejects.toEqual(
            new SyncError("INVALID_STREAM", "the source of inbox ended a stream without a page"),
        );
    },
);

test.for(TEST_DIALECTS)(
    "finish a snapshot of several pages past a drain, then resume from its completed position on %s",
    async (dialect) => {
        const source = await open(dialect);
        const copy = await openCopy(dialect);
        const feed = new Feed(source, [note, replica]);
        const notes = new Replica({ name: "notes", scope: "inbox", tables: [note] });
        const ids = Array.from({ length: SNAPSHOT_NOTES }, (_, index) => `n${index}`);
        for (let start = 0; start < ids.length; start += INSERT_BATCH) {
            const batch = ids.slice(start, start + INSERT_BATCH);
            await source.insert(note).values(batch.map((id) => ({ ...first, id })));
        }

        // drain the first stream at its first page as at a lapse, and end the second after one page
        const requested: (LogPosition | undefined)[] = [];
        const received: (readonly [number, boolean, boolean, number])[] = [];
        const controller = new AbortController();
        const following = notes.follow(
            copy,
            async function* ({ after }, signal) {
                requested.push(after);
                const stream = requested.length;
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
        );
        const position = await source.log.position();
        await expect.poll(() => requested.length).toBe(3);
        controller.abort();
        await following;

        // complete the snapshot and resume twice from its position
        expect({
            requested,
            received: received.slice(0, 2),
            notes: (await copy.select().from(note)).length,
        }).toEqual({
            requested: [undefined, position, position],
            received: [
                [1, true, false, PAGE_ROWS],
                [1, false, true, SNAPSHOT_NOTES - PAGE_ROWS],
            ],
            notes: SNAPSHOT_NOTES,
        });
    },
);

test.for(TEST_DIALECTS)(
    "match the rows each copy includes, and keep a row two copies include until neither does, on %s",
    async (dialect) => {
        const copy = await openCopy(dialect);
        const epoch = "01996ab0-0000-7000-8000-000000000001";
        const snapshot = (sequence: number, ids: readonly string[]): Page[] => [
            {
                reset: true,
                complete: true,
                changes: ids.map((id) => insertOf(id)),
                position: { epoch, sequence },
            },
        ];
        const stored = async () =>
            (await copy.select({ id: note.id }).from(note).orderBy(asc(note.id))).map(
                (row) => row.id,
            );

        // include a shared row in two copies of the same scope
        const left = new Replica({ name: "left", scope: "inbox", tables: [note] });
        const right = new Replica({ name: "right", scope: "inbox", tables: [note] });
        await replicate(left, copy, snapshot(1, ["a", "shared"]));
        await replicate(right, copy, snapshot(1, ["shared", "z"]));

        // match the rows each copy includes
        const included = async (name: string) =>
            (
                await copy
                    .select({ id: note.id })
                    .from(note)
                    .where(Replica.includes(name, note))
                    .orderBy(asc(note.id))
            ).map((row) => row.id);
        expect([await included("left"), await included("right"), await included("other")]).toEqual([
            ["a", "shared"],
            ["shared", "z"],
            [],
        ]);

        // keep the shared row while one copy still includes it, and delete it after
        await replicate(left, copy, snapshot(2, ["a"]));
        const kept = await stored();
        await replicate(right, copy, snapshot(2, ["z"]));
        expect([kept, await stored()]).toEqual([
            ["a", "shared", "z"],
            ["a", "z"],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "drop a copy: delete the rows no other copy includes, retract its projections and forget its record, on %s",
    async (dialect) => {
        const copy = await openCopy(dialect, [...TABLES, reading]);
        const epoch = "01996ab0-0000-7000-8000-000000000001";
        const snapshot = (ids: readonly string[]): Page[] => [
            {
                reset: true,
                complete: true,
                changes: ids.map((id) => insertOf(id)),
                position: { epoch, sequence: 1 },
            },
        ];
        // keep a shared row in two copies and project the left copy's notes
        const left = new Replica({
            name: "left",
            scope: "inbox",
            tables: [note],
            projectors: [projectNotes("reading")],
        });
        const right = new Replica({ name: "right", scope: "inbox", tables: [note] });
        await left.register(copy, subscriptionOf("left"));
        await right.register(copy, subscriptionOf("right"));
        await replicate(left, copy, snapshot(["a", "shared"]));
        await replicate(right, copy, snapshot(["shared", "z"]));

        // drop the left copy and keep the right copy's rows and record
        await left.drop(copy);
        expect([
            (await copy.select({ id: note.id }).from(note).orderBy(asc(note.id))).map(
                (row) => row.id,
            ),
            await copy.select({ id: reading.id }).from(reading),
            await Replica.subscriptions(copy),
        ]).toEqual([["shared", "z"], [], [subscriptionOf("right")]]);
    },
);

test.for(TEST_DIALECTS)(
    "copy the rows in the scopes of copied rows, and take them out with the row they are in, on %s",
    async (dialect) => {
        const source = await open(dialect);
        const copy = await openCopy(dialect);
        const feed = new Feed(source, [project, task, tag, replica]);
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
            expect(await Replica.wait(copy, "inbox", await source.log.position(), signal)).toBe(
                true,
            );

            return (await copy.select().from(tag).orderBy(asc(tag.id))).map((row) => [
                row.id,
                row.scope,
            ]);
        };

        // copy the tags in the scopes of the copied project and task alone
        const controller = new AbortController();
        const following = work.follow(
            copy,
            (from, stream) => feed.subscribe(work.queries, from.after, stream),
            controller.signal,
        );
        expect(await tags()).toEqual([
            ["g1", "p1"],
            ["g6", "t1"],
        ]);

        // add a tag entering a copied project's scope, and the tags of a project entering the copy
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
        await source.insert(tag).values({ id: "g5", scope: "p2", name: "kept" });
        const again = new AbortController();
        const snapshotting = work.follow(
            copy,
            (_from, stream) => feed.subscribe(work.queries, undefined, stream),
            again.signal,
        );
        expect(await tags()).toEqual([
            ["g5", "p2"],
            ["g6", "t1"],
        ]);
        again.abort();
        await snapshotting;
    },
);

test.for(TEST_DIALECTS)(
    "keep a column one copy hides while another shows it, and clear it once none does, on %s",
    async (dialect) => {
        const copy = await openCopy(dialect);
        const epoch = "01996ab0-0000-7000-8000-000000000001";
        const snapshot = (
            sequence: number,
            summary: string | null,
            concealed: string[],
        ): Page[] => [
            {
                reset: true,
                complete: true,
                changes: [
                    {
                        table: note[TABLE].sqlName,
                        operation: "insert" as const,
                        row: note[TABLE].encode({ ...first, summary }),
                        ...(concealed.length === 0 ? {} : { concealed }),
                    },
                ],
                position: { epoch, sequence },
            },
        ];
        const empty = (sequence: number): Page[] => [
            { reset: true, complete: true, changes: [], position: { epoch, sequence } },
        ];
        const summary = async () =>
            (await copy.select({ summary: note.summary }).from(note)).map((row) => row.summary);
        const shown = new Replica({ name: "open", scope: "inbox", tables: [note] });
        const closed = new Replica({ name: "closed", scope: "inbox", tables: [note] });

        // show the summary through one copy, and keep it as the other hides it
        const stored = [];
        await replicate(closed, copy, snapshot(1, null, ["summary"]));
        stored.push(await summary());
        await replicate(shown, copy, snapshot(1, "Plan", []));
        stored.push(await summary());
        await replicate(closed, copy, snapshot(2, null, ["summary"]));
        stored.push(await summary());

        // clear the summary once the copy showing it leaves the row to the one hiding it
        await replicate(shown, copy, empty(2));
        stored.push(await summary());
        expect(stored).toEqual([[null], ["Plan"], ["Plan"], [null]]);
    },
);

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

/** An unlogged cache, which only a capture copies. */
const cache = defineTable("cache", {
    id: text("id").primaryKey(),
    value: text("value").notNull(),
});

/** A key wrapped under the root key of the host that keeps it. */
const hostKey = defineTable(
    "host_key",
    {
        id: text("id").primaryKey(),
        scope: text("scope").notNull(),
        wrapped: text("wrapped").notNull(),
    },
    { log: {} },
);

/** Rewrap a host key row's wrapped value from one host to another. */
function rewrapped(table: Table, row: Row, from: string, to: string): Row {
    // leave rows of other tables
    if (table !== hostKey) {
        return row;
    }

    // replace the host prefix of the wrapped value
    const wrapped = row["wrapped"];
    if (typeof wrapped !== "string") {
        throw new TypeError("a host key row has no wrapped value");
    }

    return { ...row, wrapped: wrapped.replace(from, to) };
}

/** Build the change inserting a note. */
function insertOf(id: string) {
    return {
        table: note[TABLE].sqlName,
        operation: "insert" as const,
        row: note[TABLE].encode({ ...first, id }),
    };
}

/** Build the subscription a named copy of the inbox follows. */
function subscriptionOf(name: string) {
    return { name, shape: "notes", scope: "inbox", below: "reader", parameters: {} };
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

test("resume a subscription from the copy's position, reshape one of other parameters from those the copy reflects, and snapshot another", async () => {
    const copy = await openCopy("sqlite");
    const notes = new Replica({ name: "notes", scope: "inbox", tables: [note] });
    const subscription = {
        name: "notes",
        shape: "queries",
        scope: "inbox",
        below: "inbox",
        parameters: { queries: { open: { object: "note" } } },
    };
    const position = { epoch: "01996ab0-0000-7000-8000-000000000001", sequence: 4 };

    // complete a snapshot of the subscription
    await notes.register(copy);
    const snapshot: Page = { reset: true, complete: true, changes: [], position };
    await Array.fromAsync(notes.apply(copy, [snapshot], { subscription }));

    // resume it, reshape it under other parameters, and snapshot another shape or name, each naming the copy's own log
    const reshaped = { ...subscription, parameters: { queries: { all: { object: "note" } } } };
    const origin = await copy.log.epoch();
    expect([
        await notes.resume(copy, { ...subscription, after: { ...position, sequence: 1 } }),
        await notes.resume(copy, reshaped),
        await notes.resume(copy, { ...subscription, shape: "chain" }),
        await notes.resume(copy),
    ]).toEqual([
        { after: position, origin },
        { after: position, previous: subscription.parameters, origin },
        { origin },
        { after: position, origin },
    ]);
});

/** Follow a copy's queries from a feed, naming the follower's own origin. */
function followOf(feed: Feed, copy: Replica) {
    return (from: Resumption, signal: AbortSignal) =>
        feed.subscribe(
            copy.queries,
            from.after,
            signal,
            from.origin === undefined ? {} : { origin: from.origin },
        );
}

/** Each note's entry in a reader's list, its own row projected from the note it names. */
const reading = defineTable(
    "reading",
    {
        /** The entry's own identity. */
        id: text("id").primaryKey(),
        /** The list keeping the reading. */
        scope: text("scope").notNull(),
        /** The scope of the projected note. */
        sourceScope: text("source_scope").notNull(),
        /** The projected note. */
        sourceId: text("source_id").notNull(),
        /** The note's title when last projected. */
        title: text("title").notNull(),
        /** When the reader read the note, the entry's own state. */
        readAt: integer("read_at"),
    },
    {
        constraints: (row) => [
            uniqueIndex("reading_source").on(row.scope, row.sourceScope, row.sourceId),
        ],
    },
);

/** The note columns a projection reads. */
const NoteProjection = schema.looseObject({
    id: schema.string(),
    scope: schema.string(),
    title: schema.string(),
});

/** Project the notes of a scope into a reader's list, keeping each entry's own identity and state. */
function projectNotes(list: string): Projector {
    // match the entries of the list's projected notes
    const named = (rows: readonly Row[]) =>
        and(
            eq(reading.scope, list),
            inArray(
                reading.sourceId,
                rows.map((row) => schema.string().parse(row["id"])),
            ),
        );

    return {
        source: note,
        write: async (transaction, _scope, kept, removed) => {
            for (const row of kept) {
                const { id, scope, title } = NoteProjection.parse(row);
                await transaction
                    .insert(reading)
                    .values({
                        id: `entry-${id}`,
                        scope: list,
                        sourceScope: scope,
                        sourceId: id,
                        title,
                        readAt: null,
                    })
                    .onConflictDoUpdate({
                        target: [reading.scope, reading.sourceScope, reading.sourceId],
                        set: { title },
                    });
            }
            if (removed.length > 0) {
                await transaction.delete(reading).where(named(removed));
            }
        },
        prune: async (transaction, scope, delivered) => {
            const entries = await transaction
                .select()
                .from(reading)
                .where(and(eq(reading.scope, list), eq(reading.sourceScope, scope)));
            const stale = entries.filter(
                (row) => !delivered.has(Key.name(note, { id: row.sourceId })),
            );
            if (stale.length > 0) {
                await transaction.delete(reading).where(
                    inArray(
                        reading.id,
                        stale.map((row) => row.id),
                    ),
                );
            }
        },
    };
}

test.for(TEST_DIALECTS)(
    "project a scope's notes into a reader's list, keeping each entry's own state, retracting a deleted note's entry and those a later snapshot leaves out, on %s",
    async (dialect) => {
        const source = await open(dialect);
        const copy = await openCopy(dialect, [...TABLES, reading]);
        const feed = new Feed(source, [note, replica]);
        const notes = new Replica({
            name: "notes",
            scope: "inbox",
            tables: [],
            projectors: [projectNotes("reading")],
        });
        await source.insert(note).values([first, { ...first, id: "b", title: "Second" }]);
        const entries = async () =>
            (await copy.select().from(reading).orderBy(asc(reading.sourceId))).map((row) => [
                row.id,
                row.sourceId,
                row.title,
                row.readAt,
            ]);
        const follow = (from: (resumed: LogPosition | undefined) => LogPosition | undefined) => {
            const controller = new AbortController();
            const following = notes.follow(
                copy,
                ({ after }, signal) =>
                    feed.subscribe(
                        { ...notes.queries, note: { table: note, scopes: ["inbox"] } },
                        from(after),
                        signal,
                    ),
                controller.signal,
            );

            return { stop: async () => (controller.abort(), await following) };
        };
        const reach = async () =>
            expect(
                await Replica.wait(
                    copy,
                    "inbox",
                    await source.log.position(),
                    AbortSignal.timeout(5000),
                ),
            ).toBe(true);

        // project both notes and keep the entry's state across a retitle
        const following = follow((after) => after);
        await reach();
        await copy.update(reading).set({ readAt: 7 }).where(eq(reading.sourceId, "a"));
        await source.update(note).set({ title: "Renamed" }).where(eq(note.id, "a"));
        await reach();
        expect(await entries()).toEqual([
            ["entry-a", "a", "Renamed", 7],
            ["entry-b", "b", "Second", null],
        ]);

        // retract a deleted note's entry
        await source.delete(note).where(eq(note.id, "b"));
        await reach();
        expect(await entries()).toEqual([["entry-a", "a", "Renamed", 7]]);
        await following.stop();

        // retract the entries a later snapshot leaves out
        await source.delete(note).where(eq(note.id, "a"));
        await source.insert(note).values({ ...first, id: "c", title: "Third" });
        const snapshotting = follow(() => undefined);
        await reach();
        expect(await entries()).toEqual([["entry-c", "c", "Third", null]]);
        await snapshotting.stop();
    },
);

test("stream through a client's replica procedures as the follower, and send a forwarded change as its signer", async () => {
    // call a fake client, recording whom each call signs as
    const signers: string[] = [];
    const uplink = Uplink.of((signer) => {
        signers.push(signer === undefined ? "follower" : signer.id);

        return {
            stream: async () => (async function* (): AsyncIterable<Page> {})(),
            receive: async () => ({}),
        };
    });

    // stream a copy, then send a change an installation signs
    const pages = await Array.fromAsync(
        uplink.stream(
            { name: "notes", scope: "inbox", below: "inbox", shape: "notes", parameters: {} },
            AbortSignal.timeout(1000),
        ),
    );
    await uplink.receive(
        {
            id: "019f5530-8000-7000-8000-000000000001",
            calls: [{ method: "note.create", release: "1.0.0", input: { id: "a" } }],
        },
        {
            packageId: schema
                .identifier("package")
                .parse("package-01996ab0-0000-7000-8000-000000000001"),
            type: "installation",
            scope: "inbox",
            id: "installation-a",
        },
    );
    expect([pages, signers]).toEqual([[], ["follower", "installation-a"]]);
});
