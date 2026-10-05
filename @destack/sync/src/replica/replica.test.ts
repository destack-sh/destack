import { expect, test } from "@destack/test";
import { TEST_DIALECTS } from "@destack/db/test";
import {
    and,
    asc,
    binary,
    blob,
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
} from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import type { BlobStore } from "@destack/db/blob";
import { LocalBlobStore } from "@destack/db/blob/local";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { onTestFinished } from "@destack/test";
import { Feed } from "../feed/feed.ts";
import { schema } from "@destack/schema";
import { replicaTables, Replica, replica, type Projector } from "./replica.ts";
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
        expect(await Replica.reach(copy, "inbox", await source.log.position(), signal)).toBe(true);
        await source.update(note).set({ title: "Renamed" }).where(eq(note.id, "a"));
        await source.delete(note).where(eq(note.id, "b"));
        expect(await Replica.reach(copy, "inbox", await source.log.position(), signal)).toBe(true);
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
            await Replica.reach(copy, "inbox", head, signal),
            await Replica.reach(copy, "archive", head, signal),
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
        const stores = await mkdtemp(join(tmpdir(), "destack-replica-blobs-"));
        onTestFinished(() => rm(stores, { recursive: true, force: true }));
        const sourceBlobs = await LocalBlobStore.open(join(stores, "source"));
        const targetBlobs = await LocalBlobStore.open(join(stores, "target"));
        const digest = await sourceBlobs.write(bytesOf("the first entry's content"));
        const blobs = { store: recorded(targetBlobs).store, source: sourceBlobs };
        await source.insert(entry).values({
            id: "a",
            scope: "inbox",
            title: "First",
            secret: "s1",
            data,
            content: digest,
        });

        // follow the logged rows while the source writes
        const controller = new AbortController();
        const following = copy.follow(
            target,
            (from, signal) => feed.subscribe(copy.queries, from.after, signal),
            controller.signal,
            { blobs },
        );
        const signal = AbortSignal.timeout(5000);
        expect(await Replica.reach(target, "database", await source.log.position(), signal)).toBe(
            true,
        );
        await source
            .insert(entry)
            .values({ id: "b", scope: "archive", title: "Second", secret: "s2", data });
        await source.insert(cache).values({ id: "c", value: "cached" });
        await source.insert(hostKey).values({ id: "k", scope: "inbox", wrapped: "source:k" });
        expect(await Replica.reach(target, "database", await source.log.position(), signal)).toBe(
            true,
        );
        controller.abort();
        await following;

        // capture every column once the source stops writing, rewrapping the key for the target
        const wrap = async (table: Table, row: Row) => rewrapped(table, row, "source:", "moving:");
        const unwrap = async (table: Table, row: Row) =>
            rewrapped(table, row, "moving:", "target:");
        const applied = copy.apply(target, feed.capture(copy.captured, signal, { rewrap: wrap }), {
            unwrap,
            blobs,
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

        // keep the content the copied rows reference in the target's store
        const kept: Uint8Array[] = [];
        for await (const chunk of targetBlobs.read(digest)) {
            kept.push(chunk);
        }
        expect(new TextDecoder().decode(Buffer.concat(kept))).toBe("the first entry's content");
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
        expect(await Replica.reach(copy, "inbox", await source.log.position(), signal)).toBe(true);
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
            Replica.reach(copy, "inbox", { epoch, sequence: 5 }, AbortSignal.timeout(20));

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
            await Replica.reach(
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
        expect(await Replica.reach(copy, "inbox", await source.log.position(), signal)).toBe(true);
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
        expect(await Replica.reach(copy, "inbox", await source.log.position(), signal)).toBe(true);
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
            expect(await Replica.reach(copy, "inbox", await source.log.position(), signal)).toBe(
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

test.for(TEST_DIALECTS)(
    "retire the blobs of the rows a copy deletes, replaces or drops, holding fetched blobs until their rows commit, on %s",
    async (dialect) => {
        // keep two entries with content in the source
        const tables = [entry, ...replicaTables];
        const source = (await TestDatabase.create(dialect, tables, { isMigrated: true })).database;
        const target = (
            await TestDatabase.create(dialect, tables, { isMigrated: true, isReplica: true })
        ).database;
        onTestFinished(async () => {
            await source.close();
            await target.close();
        });
        const copy = new Replica({
            name: "database",
            scope: "database",
            tables: [entry],
            everywhere: new Set([entry]),
        });
        const feed = new Feed(source, [entry, replica]);
        const stores = await mkdtemp(join(tmpdir(), "destack-replica-retire-"));
        onTestFinished(() => rm(stores, { recursive: true, force: true }));
        const sourceBlobs = await LocalBlobStore.open(join(stores, "source"));
        const kept = recorded(await LocalBlobStore.open(join(stores, "target")));
        const blobs = { store: kept.store, source: sourceBlobs };
        const [firstContent, secondContent, thirdContent] = await Promise.all(
            ["first", "second", "third"].map((content) => sourceBlobs.write(bytesOf(content))),
        );
        const row = { scope: "inbox", title: "Entry", secret: null, data: null };
        await source.insert(entry).values([
            { ...row, id: "a", content: firstContent },
            { ...row, id: "b", content: secondContent },
        ]);
        const capture = async () => {
            const signal = AbortSignal.timeout(5000);
            await Array.fromAsync(
                copy.apply(target, feed.capture(copy.captured, signal), { blobs }),
            );
        };

        // copy both entries before replacing one's content and deleting the other
        await capture();
        const copied = kept.retired.length;
        await source.update(entry).set({ content: thirdContent }).where(eq(entry.id, "a"));
        await source.delete(entry).where(eq(entry.id, "b"));
        await capture();
        const changed = kept.retired.splice(0);

        // drop the copy with its remaining entry
        await copy.drop(target, kept.store);
        expect({
            copied,
            changed,
            dropped: kept.retired,
            fetched: [...new Set(kept.holds)],
            released: kept.held(),
        }).toEqual({
            copied: 0,
            changed: [firstContent, secondContent],
            dropped: [thirdContent],
            fetched: [1],
            released: 0,
        });
    },
);

/** An entry of any scope with sensitive and binary columns, which its log leaves out, and content a blob store keeps. */
const entry = defineTable(
    "entry",
    {
        id: text("id").primaryKey(),
        scope: text("scope").notNull(),
        title: text("title").notNull(),
        content: blob("content"),
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

/** Keep blobs in a directory as a collected store, recording its holds at each fetch and the blobs writers retire, deleting none. */
function recorded(blobs: LocalBlobStore) {
    const retired: string[] = [];
    const holds: number[] = [];
    let held = 0;
    const store: BlobStore = {
        missing: (digests) => blobs.missing(digests),
        read: (digest) => blobs.read(digest),
        write: (body, expected) => blobs.write(body, expected),
        fetch: (digests, source) => {
            holds.push(held);

            return blobs.fetch(digests, source);
        },
        hold: async () => {
            held += 1;

            return {
                [Symbol.asyncDispose]: async () => {
                    held -= 1;
                },
            };
        },
        retire: async (digests) => {
            retired.push(...digests);
        },
    };

    return { store, retired, holds, held: () => held };
}

/** Stream a text's bytes. */
async function* bytesOf(content: string): AsyncIterable<Uint8Array> {
    yield new TextEncoder().encode(content);
}

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

    // resume it, reshape it under other parameters, and snapshot another shape or name
    const reshaped = { ...subscription, parameters: { queries: { all: { object: "note" } } } };
    expect([
        await notes.resume(copy, { ...subscription, after: { ...position, sequence: 1 } }),
        await notes.resume(copy, reshaped),
        await notes.resume(copy, { ...subscription, shape: "chain" }),
        await notes.resume(copy),
    ]).toEqual([
        { after: position },
        { after: position, previous: subscription.parameters },
        {},
        { after: position },
    ]);
});

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
                await Replica.reach(
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
