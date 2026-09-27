import { expect, test } from "@destack/test";
import { TEST_DIALECTS } from "@destack/db/test";
import { asc, encodeRow, eq, TABLE } from "@destack/db";
import { Feed } from "../feed/feed.ts";
import { Replica, replica } from "./replica.ts";
import { asset, first, note, open, openCopy, project, replicate, task } from "../test/fixture.ts";
import type { Query } from "../query/query.ts";
import type { QueryPage } from "../query/page.ts";

test.for(TEST_DIALECTS)(
    "follow a scope of another database into a local copy on %s",
    async (dialect) => {
        const source = await open(dialect);
        const copy = await openCopy(dialect);
        const feed = new Feed(source, [note, replica]);
        const notes = new Replica({ name: "notes", scope: "inbox", tables: [note] });
        await source.insert(note).values([first, { ...first, id: "b", scope: "archive" }]);
        await copy.insert(note).values({ ...first, id: "stale" });

        // follow the source until the copy holds the renamed note
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

        // hold the scope's rows as renamed, pruned of what the snapshot left out
        expect((await copy.select().from(note)).map((row) => [row.id, row.title])).toEqual([
            ["a", "Renamed"],
        ]);
        expect(await notes.position(copy)).toEqual(await source.log.position());
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

        // hold the projects, then read the same copy as one of projects and tasks
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

        // hold a position only as the shape that copied it, so that the new shape snapshots again
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

        // never reach a position of a scope the database does not copy
        expect([await Replica.isCopied(copy, "inbox"), await reach()]).toEqual([false, false]);

        // wait once registered, until a complete page reaches the position
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

        // refuse a watermark of an epoch older than the copy's
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

        // keep the copy as it was while a run is staged
        const stream = notes.apply(copy, snapshot);
        await stream.next();
        expect(await held()).toEqual([["b", "stale"], undefined]);

        // drop what a broken stream staged, so a later run never completes it
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

        // apply the whole snapshot at once, pruning the rows it left out
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

        // count each project's tasks, and measure them per state
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

        // nest a project's measures, the measures of no rows for an empty one, and each state's group
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

test("refuse copying a table whose required column the log leaves out", () => {
    expect(() => new Replica({ name: "assets", scope: "inbox", tables: [asset] })).toThrow(
        "replica table asset requires unlogged column content",
    );
});
