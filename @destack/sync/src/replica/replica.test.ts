import { expect, test } from "@destack/test";
import { TEST_DIALECTS } from "@destack/db/test";
import { asc, encodeRow, eq, TABLE } from "@destack/db";
import { Feed } from "../feed/feed.ts";
import { REPLICA_TABLES, Replica, replica } from "./replica.ts";
import { asset, first, note, open, openCopy, project, replicate, task } from "../test/fixture.ts";
import type { Query } from "../query/query.ts";
import type { QueryPage } from "../query/page.ts";
import type { LogPosition } from "@destack/db/log";
import { PAGE_ROWS } from "../dataflow/selection.ts";
import { SyncError } from "../error/error.ts";

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

        // follow until the copy holds the renamed note
        const controller = new AbortController();
        const following = notes.follow(
            copy,
            (after, signal) => feed.subscribe(notes.queries, after, signal),
            controller.signal,
        );
        const signal = AbortSignal.timeout(5000);
        expect(await Replica.reach(copy, "inbox", await source.log.position(), signal)).toBe(true);
        await source.update(note).set({ title: "Renamed" }).where(eq(note.id, "a"));
        await source.delete(note).where(eq(note.id, "b"));
        expect(await Replica.reach(copy, "inbox", await source.log.position(), signal)).toBe(true);
        controller.abort();
        await following;

        // hold the renamed rows and prune the rest
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
    "snapshot again once the copied tables change shape on %s",
    async (dialect) => {
        const source = await open(dialect);
        const copy = await openCopy(dialect);
        const feed = new Feed(source, [project, task, replica]);
        const projects = new Replica({ name: "work", scope: "inbox", tables: [project] });
        await source.insert(project).values({ id: "p1", scope: "inbox", name: "Plan" });

        // copy projects, then projects and tasks
        const controller = new AbortController();
        const following = projects.follow(
            copy,
            (after, signal) => feed.subscribe(projects.queries, after, signal),
            controller.signal,
        );
        const signal = AbortSignal.timeout(5000);
        expect(await Replica.reach(copy, "inbox", await source.log.position(), signal)).toBe(true);
        controller.abort();
        await following;
        const work = new Replica({ name: "work", scope: "inbox", tables: [project, task] });

        // hold a position per shape
        expect([await projects.position(copy), await work.position(copy)]).toEqual([
            await source.log.position(),
            undefined,
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "reach a position only through a copy that holds it on %s",
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
        const insert = (id: string) => ({
            table: note[TABLE].sqlName,
            operation: "insert" as const,
            row: encodeRow(note, { ...first, id }),
        });
        const snapshot: QueryPage[] = [
            {
                reset: true,
                complete: false,
                changes: [insert("a")],
                position: { epoch, sequence: 3 },
            },
            {
                reset: false,
                complete: true,
                changes: [insert("b")],
                position: { epoch, sequence: 4 },
            },
        ];
        const held = async () => [
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
        expect(await held()).toEqual([["b", "stale"], undefined]);

        // drop a broken stream's staged run
        await stream.return(undefined);
        await replicate(notes, copy, [
            {
                reset: false,
                complete: true,
                changes: [insert("c")],
                position: { epoch, sequence: 2 },
            },
        ]);
        expect(await held()).toEqual([["b", "c", "stale"], { epoch, sequence: 2 }]);

        // apply the snapshot and prune the left out rows
        await replicate(notes, copy, snapshot);
        expect(await held()).toEqual([["a", "b"], { epoch, sequence: 4 }]);
    },
);

test.for(TEST_DIALECTS)(
    "nest each held row's measures, whole or per group, in the rows a copy reads on %s",
    async (dialect) => {
        const source = await open(dialect);
        const copy = await openCopy(dialect);
        const feed = new Feed(source, [project, task, replica]);
        const projects = new Replica({ name: "projects", scope: "inbox", tables: [project, task] });
        const tasks = (projectId: string, state: string, rank: number) => ({
            id: `${projectId}-${state}-${rank}`,
            scope: "inbox",
            projectId,
            state,
            rank,
            points: rank * 2,
            title: null,
            isSecret: false,
        });
        await source.insert(project).values([
            { id: "p", scope: "inbox", name: "Plans" },
            { id: "q", scope: "inbox", name: "Quiet" },
        ]);
        await source
            .insert(task)
            .values([tasks("p", "open", 1), tasks("p", "open", 4), tasks("p", "done", 2)]);

        // count tasks per project and state
        const query: Query = {
            table: project,
            scopes: ["inbox"],
            order: [{ column: "name", direction: "asc" }],
            include: {
                size: {
                    table: task,
                    on: { kind: "key", column: "projectId", parent: "id" },
                    aggregate: {
                        values: {
                            tasks: { function: "count" },
                            top: { function: "max", column: "rank" },
                        },
                    },
                },
                states: {
                    table: task,
                    on: { kind: "key", column: "projectId", parent: "id" },
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
            (after, signal) => feed.subscribe({ projects: query }, after, signal),
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
            read.map((row) => [
                row.name,
                row.size,
                (row.states as { group: unknown }[]).toSorted((left, right) =>
                    JSON.stringify(left.group) < JSON.stringify(right.group) ? -1 : 1,
                ),
            ]),
        ).toEqual([
            [
                "Plans",
                { tasks: 3, top: 4 },
                [
                    { group: { state: "done" }, values: { points: 4 } },
                    { group: { state: "open" }, values: { points: 10 } },
                ],
            ],
            ["Quiet", { tasks: 0, top: null }, []],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "copy a table whose required column the log leaves out, holding nothing for it, on %s",
    async (dialect) => {
        // follow an asset whose binary content never enters the log
        const source = await open(dialect, [asset, ...REPLICA_TABLES]);
        const copy = await openCopy(dialect, [asset, ...REPLICA_TABLES]);
        const feed = new Feed(source, [asset, replica]);
        const assets = new Replica({ name: "assets", scope: "inbox", tables: [asset] });
        await source
            .insert(asset)
            .values({ id: "a", scope: "inbox", content: new Uint8Array([1]) });
        const controller = new AbortController();
        const following = assets.follow(
            copy,
            (after, signal) => feed.subscribe(assets.queries, after, signal),
            controller.signal,
        );
        const signal = AbortSignal.timeout(5000);
        expect(await Replica.reach(copy, "inbox", await source.log.position(), signal)).toBe(true);
        controller.abort();
        await following;

        // hold the row without its content
        expect(await copy.select().from(asset)).toEqual([
            { id: "a", scope: "inbox", content: null },
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
            async function* (after, signal) {
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

        // resume from the snapshot's position and hold the renamed note
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
            async function* (after, signal) {
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

        // complete the snapshot, then resume twice from its position
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
