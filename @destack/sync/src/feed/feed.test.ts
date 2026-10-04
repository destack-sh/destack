import { expect, test } from "@destack/test";
import { aligned, schema } from "@destack/schema";
import { TEST_DIALECTS } from "@destack/db/test";
import { defineRelations, defineTable, eq, json, TABLE, text } from "@destack/db";
import { Feed } from "./feed.ts";
import type { Page } from "../query/page.ts";
import {
    first,
    nextValue,
    note,
    open,
    project,
    relations,
    take,
    task,
    until,
} from "../test/fixture.ts";
import type { Item, Query } from "../query/query.ts";

test.for(TEST_DIALECTS)("keep a query of one scope's matching rows on %s", async (dialect) => {
    const database = await open(dialect);
    const feed = new Feed(database, [note]);
    const query = { table: note, scopes: ["inbox"], where: { title: "Open" } };
    await database.insert(note).values([
        { ...first, id: "a", title: "Open" },
        { ...first, id: "b", title: "Done" },
        { ...first, id: "c", title: "Open", scope: "archive" },
    ]);

    // snapshot the matching rows of the scope
    const signal = AbortSignal.timeout(5000);
    const pages = feed.subscribe({ notes: query }, undefined, signal);
    const snapshot = aligned(await take(pages, (page) => page.complete), 0);
    expect(snapshot).toEqual({
        reset: true,
        complete: true,
        changes: [
            {
                table: note[TABLE].sqlName,
                operation: "insert",
                row: {
                    id: "a",
                    title: "Open",
                    scope: "inbox",
                    summary: null,
                    views: "9007199254740993",
                    labels: ["draft"],
                    editedAt: 1790244000123,
                    attachment: "AQID",
                },
            },
        ],
        position: await database.log.position(),
    });

    // enter, leave and move between scopes
    const following = feed.subscribe({ notes: query }, snapshot.position, signal);
    const next = take(following, (page) => page.changes.some((change) => change.row["id"] === "c"));
    await database.update(note).set({ title: "Open" }).where(eq(note.id, "b"));
    await database.update(note).set({ title: "Done" }).where(eq(note.id, "a"));
    await database.update(note).set({ summary: "Unrelated" }).where(eq(note.id, "a"));
    await database.update(note).set({ scope: "inbox" }).where(eq(note.id, "c"));
    const changes = (await next).flatMap((page) =>
        page.changes.map((change) => [change.operation, change.row["id"]]),
    );
    expect(changes).toEqual([
        ["insert", "b"],
        ["delete", "a"],
        ["insert", "c"],
    ]);
});

test.for(TEST_DIALECTS)(
    "page a large snapshot and continue after its first page on %s",
    async (dialect) => {
        const database = await open(dialect);
        const feed = new Feed(database, [note]);
        const ids = Array.from(
            { length: 2500 },
            (_, index) => `n${String(index).padStart(4, "0")}`,
        );
        await database.insert(note).values(ids.map((id) => ({ ...first, id })));

        // read three key-ordered pages
        const signal = AbortSignal.timeout(5000);
        const pages = await take(
            feed.subscribe({ notes: { table: note, scopes: ["inbox"] } }, undefined, signal),
            (page) => page.complete,
        );
        expect(pages.map((page) => [page.reset, page.complete, page.changes.length])).toEqual([
            [true, false, 1000],
            [false, false, 1000],
            [false, true, 500],
        ]);
        expect(pages.flatMap((page) => page.changes.map((change) => change.row["id"]))).toEqual(
            ids,
        );
        expect(new Set(pages.map((page) => page.position.sequence)).size).toBe(1);
    },
);

test.for(TEST_DIALECTS)(
    "serve subscribers behind the feed from the log and current ones from memory on %s",
    async (dialect) => {
        const database = await open(dialect);
        const feed = new Feed(database, [note]);
        const query = { notes: { table: note, scopes: ["inbox"] } };
        const signal = AbortSignal.timeout(5000);
        const start = await database.log.position();

        // commit while a second subscriber catches up
        const current = feed.subscribe(query, start, signal);
        const seen = take(current, (page) =>
            page.changes.some((change) => change.row["id"] === "b"),
        );
        await database.insert(note).values(first);
        const behind = feed.subscribe(query, start, signal);
        const caught = take(behind, (page) =>
            page.changes.some((change) => change.row["id"] === "b"),
        );
        await database.insert(note).values({ ...first, id: "b" });

        // deliver the same changes to both
        expect([idsOf(await seen), idsOf(await caught)]).toEqual([
            ["a", "b"],
            ["a", "b"],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "snapshot again when the source starts a new epoch on %s",
    async (dialect) => {
        const database = await open(dialect);
        const feed = new Feed(database, [note]);
        const query = { notes: { table: note, scopes: ["inbox"] } };
        await database.insert(note).values(first);
        const signal = AbortSignal.timeout(5000);

        // continue within the epoch without a snapshot
        const before = await database.log.position();
        const resumed = feed.subscribe(query, before, signal);
        const continued = take(resumed, (page) => page.changes.length > 0);
        await database.insert(note).values({ ...first, id: "b" });
        const changed = (await continued).filter((page) => page.reset || page.changes.length > 0);
        expect(changed.map((page) => [page.reset, page.changes.length])).toEqual([[false, 1]]);

        // start over after a restore
        const renewed = await database.log.renew();
        const snapshot = aligned(
            await take(feed.subscribe(query, before, signal), (page) => page.complete),
            0,
        );
        expect([snapshot.reset, snapshot.position.epoch, snapshot.changes.length]).toEqual([
            true,
            renewed,
            2,
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "decide a write once for every stream of the same queries and audience at the head on %s",
    async (dialect) => {
        const database = await open(dialect);
        const feed = new Feed(database, [note]);
        const query = { table: note, scopes: ["inbox"] };
        await database.insert(note).values({ ...first, id: "a" });

        // follow one query in two streams
        const signal = AbortSignal.timeout(5000);
        const streams = [0, 1].map(() => feed.subscribe({ notes: query }, undefined, signal));
        for (const pages of streams) {
            await until(pages, (page) => page.complete);
        }

        // decide a write once for both
        await database.update(note).set({ title: "Renamed" }).where(eq(note.id, "a"));
        const lasts = await Promise.all(
            streams.map(async (pages) => {
                const read = await until(pages, (page) => page.changes.length > 0);

                return aligned(read, read.length - 1);
            }),
        );
        const left = aligned(lasts, 0);
        const right = aligned(lasts, 1);
        expect([left === right, left.changes.map((change) => change.row["title"])]).toEqual([
            true,
            ["Renamed"],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "follow the head through pages merged within an interval on %s",
    async (dialect) => {
        const database = await open(dialect);
        const feed = new Feed(database, [note]);
        await database.insert(note).values({ ...first, id: "a" });

        // follow with merged pages
        const pages = feed.subscribe(
            { notes: { table: note, scopes: ["inbox"] } },
            undefined,
            AbortSignal.timeout(4000),
            { every: 10 },
        );
        const snapshot = await until(pages, (page) => page.complete);

        // rename, add, remove and add notes
        await database.update(note).set({ title: "Renamed" }).where(eq(note.id, "a"));
        await database.insert(note).values({ ...first, id: "b" });
        await database.delete(note).where(eq(note.id, "b"));
        await database.insert(note).values({ ...first, id: "c" });
        const following = await until(pages, (page) =>
            page.changes.some((change) => change.row["id"] === "c"),
        );

        // keep each note as the pages leave it
        const notes = new Map<unknown, unknown>();
        for (const change of [...snapshot, ...following].flatMap((page) => page.changes)) {
            // forget a deleted note
            if (change.operation === "delete") {
                notes.delete(change.row["id"]);
            }
            // keep an entered or changed note unchanged
            else {
                notes.set(change.row["id"], change.row["title"]);
            }
        }
        expect([...notes]).toEqual([
            ["a", "Renamed"],
            ["c", "First"],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "describe the evaluation streams share with its pipelines and the cost of its last run on %s",
    async (dialect) => {
        const database = await open(dialect);
        const feed = new Feed(database, [note]);
        const query = { table: note, scopes: ["inbox"] };
        await database.insert(note).values({ ...first, id: "a" });

        // follow one query in two streams
        const controller = new AbortController();
        const streams = [0, 1].map(() =>
            feed.subscribe({ notes: query }, undefined, controller.signal),
        );
        await Promise.all(streams.map((pages) => readUntil(pages, (page) => page.complete)));
        const followed = Promise.all(
            streams.map((pages) => readUntil(pages, (page) => page.changes.length > 0)),
        );
        await database.insert(note).values({ ...first, id: "b" });
        await followed;

        // describe the shared evaluation
        const inspection = feed.inspect();
        expect([
            inspection.subscribers,
            inspection.evaluations.map(({ streams: shared, dataflow }) => [
                shared,
                dataflow.pipelines,
                dataflow.last?.changes,
                dataflow.last?.sent,
            ]),
        ]).toEqual([
            2,
            [
                [
                    2,
                    [
                        {
                            node: "notes",
                            kind: "selection",
                            path: "root",
                            partitions: 1,
                            members: 2,
                            size: 0,
                        },
                    ],
                    1,
                    1,
                ],
            ],
        ]);
        controller.abort();
        await Promise.all(streams.map((pages) => pages.return(undefined)));
    },
);

test.for(TEST_DIALECTS)(
    "keep serving a subscriber while another stops reading on %s",
    async (dialect) => {
        const database = await open(dialect);
        const feed = new Feed(database, [note]);
        const query = { table: note, scopes: ["inbox"] };
        const signal = AbortSignal.timeout(5000);

        // let one subscriber stop after its snapshot
        const idle = feed.subscribe({ query }, undefined, signal);
        await idle.next();
        const busy = feed.subscribe({ query }, undefined, signal);
        await busy.next();
        for (let index = 0; index < 20; index += 1) {
            await database.insert(note).values({ ...first, id: `n${index}` });
            expect((await busy.next()).done).toBe(false);
        }
    },
);

test.for(TEST_DIALECTS)(
    "watch a query's rows again only when a commit changes them on %s",
    async (dialect) => {
        const database = await open(dialect);
        const feed = new Feed(database, [note]);
        const query = { table: note, scopes: ["inbox"], where: { title: "Open" } };
        await database.insert(note).values([
            { ...first, id: "a", title: "Open" },
            { ...first, id: "b", title: "Done" },
        ]);

        // read the first result, then commit a change outside it and one inside it
        const watching = feed.watch("notes", query, AbortSignal.timeout(5000));
        const initial = await nextValue(watching);
        await database.update(note).set({ summary: "Unrelated" }).where(eq(note.id, "b"));
        await database.update(note).set({ title: "Open" }).where(eq(note.id, "b"));
        const changed = await nextValue(watching);
        await watching.return(undefined);

        // skip the commit that left the rows as they were
        const ids = [initial, changed].map((items) => items.map((item) => item.row["id"]));
        expect(ids).toEqual([["a"], ["a", "b"]]);
    },
);

test.for(TEST_DIALECTS)(
    "watch an ordered query keeping unchanged rows and placing a changed row by its order on %s",
    async (dialect) => {
        const database = await open(dialect);
        const feed = new Feed(database, [note]);
        const query = {
            table: note,
            scopes: ["inbox"],
            orderBy: { title: "asc" },
        } satisfies Query;
        await database.insert(note).values([
            { ...first, id: "a", title: "Apples" },
            { ...first, id: "b", title: "Bread" },
            { ...first, id: "c", title: "Cheese" },
        ]);

        // read the rows, then rename the first one past the others
        const watching = feed.watch("notes", query, AbortSignal.timeout(5000));
        const initial = await nextValue(watching);
        await database.update(note).set({ title: "Dates" }).where(eq(note.id, "a"));
        const renamed = await nextValue(watching);
        await watching.return(undefined);

        // keep the unchanged rows as the same objects, and place the renamed one last
        expect([
            renamed.map((item) => [item.row["id"], item.row["title"]]),
            renamed[0] === initial[1],
            renamed[1] === initial[2],
            renamed[2] === initial[0],
        ]).toEqual([
            [
                ["b", "Bread"],
                ["c", "Cheese"],
                ["a", "Dates"],
            ],
            true,
            true,
            false,
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "watch projects with their tasks, reordering one project's tasks and keeping the other project as it was on %s",
    async (dialect) => {
        const database = await open(dialect);
        const feed = new Feed(database, [project, task]);
        const query = {
            table: project,
            scopes: ["inbox"],
            relations,
            orderBy: { name: "asc" },
            with: { tasks: { orderBy: { rank: "asc" } } },
        } satisfies Query;
        await database.insert(project).values([
            { id: "home", scope: "inbox", name: "Home" },
            { id: "work", scope: "inbox", name: "Work" },
        ]);
        await database
            .insert(task)
            .values([
                taskOf("dishes", "home", 1),
                taskOf("laundry", "home", 2),
                taskOf("report", "work", 1),
            ]);

        // read both projects, then move the first home task last
        const watching = feed.watch("projects", query, AbortSignal.timeout(5000));
        const initial = await nextValue(watching);
        await database.update(task).set({ rank: 3 }).where(eq(task.id, "dishes"));
        const moved = await nextValue(watching);
        await watching.return(undefined);

        // reorder the home project's tasks, and keep the work project and its tasks as the same objects
        expect([
            moved.map((item) => tasksOf(item).map((entry) => entry.row["title"])),
            moved[1] === initial[1],
            moved[0] === initial[0],
            tasksOf(aligned(moved, 0))[0] === tasksOf(aligned(initial, 0))[1],
        ]).toEqual([[["laundry", "dishes"], ["report"]], true, false, true]);
    },
);

test.for(TEST_DIALECTS)(
    "watch a board whose JSON columns hold an object and a list of objects apart from its included cards on %s",
    async (dialect) => {
        const database = await open(dialect, [board, card]);
        const feed = new Feed(database, [board, card]);
        const query = {
            table: board,
            scopes: ["inbox"],
            relations: BOARD_RELATIONS,
            with: { cards: { orderBy: { id: "asc" } } },
        } satisfies Query;
        await database.insert(board).values({
            id: "plan",
            scope: "inbox",
            layout: { id: "grid", columns: 3 },
            steps: [{ id: "draft" }, { id: "review" }],
        });
        await database.insert(card).values([
            { id: "one", scope: "inbox", boardId: "plan" },
            { id: "two", scope: "inbox", boardId: "plan" },
        ]);

        // read the board with its cards
        const watching = feed.watch("boards", query, AbortSignal.timeout(5000));
        const [read] = await nextValue(watching);
        await watching.return(undefined);

        // keep the JSON values in the row and the cards apart from them
        expect([
            read?.row,
            Object.keys(read?.with ?? {}),
            (read?.with["cards"] ?? []).map((entry) => entry.row["id"]),
        ]).toEqual([
            {
                id: "plan",
                scope: "inbox",
                layout: { id: "grid", columns: 3 },
                steps: [{ id: "draft" }, { id: "review" }],
            },
            ["cards"],
            ["one", "two"],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "resume at once past commits of tables the feed leaves out on %s",
    async (dialect) => {
        const database = await open(dialect);
        const feed = new Feed(database, [note]);
        const query = { table: note, scopes: ["inbox"] };
        await database.insert(note).values(first);

        // keep the feed reading from the snapshot's position
        const signal = AbortSignal.timeout(5000);
        const reading = feed.subscribe({ notes: query }, undefined, signal);
        const snapshot = await nextValue(reading);

        // commit to another table, then resume from the snapshot's position
        await database.insert(project).values({ id: "p1", scope: "inbox", name: "Plan" });
        const [resumed] = await take(
            feed.subscribe({ notes: query }, snapshot.position, signal),
            (page) => page.complete,
        );
        await reading.return(undefined);
        expect([snapshot.complete, resumed]).toEqual([
            true,
            {
                reset: false,
                complete: true,
                changes: [],
                position: await database.log.position(),
            },
        ]);
    },
);

/** Boards whose JSON columns hold an object and a list of objects. */
const board = defineTable(
    "board",
    {
        /** The board identity. */
        id: text("id").primaryKey(),
        /** The folder the board lives in. */
        scope: text("scope").notNull(),
        /** The layout, an object. */
        layout: json(
            "layout",
            schema.object({ id: schema.string(), columns: schema.number() }),
        ).notNull(),
        /** The steps, a list of objects. */
        steps: json("steps", schema.array(schema.object({ id: schema.string() }))).notNull(),
    },
    { log: {} },
);

/** Cards of boards. */
const card = defineTable(
    "card",
    {
        /** The card identity. */
        id: text("id").primaryKey(),
        /** The folder the card lives in. */
        scope: text("scope").notNull(),
        /** The board the card is on. */
        boardId: text("board_id").notNull(),
    },
    { log: {} },
);

/** The cards of a board. */
const BOARD_RELATIONS = defineRelations({ board, card }, (relate) => ({
    board: { cards: relate.many.card({ from: relate.board.id, to: relate.card.boardId }) },
}));

/** List the row identifiers some pages change. */
function idsOf(pages: readonly Page[]): unknown[] {
    return pages.flatMap((page) => page.changes.map((change) => change.row["id"]));
}

/** Read pages until one satisfies a condition. */
async function readUntil(
    pages: AsyncGenerator<Page>,
    isLast: (page: Page) => boolean,
): Promise<void> {
    while (!isLast(await nextValue(pages))) {
        // read on
    }
}

/** Build an open task of a project. */
function taskOf(id: string, projectId: string, rank: number) {
    return {
        id,
        scope: "inbox",
        projectId,
        state: "open",
        rank,
        points: null,
        title: id,
        isSecret: false,
    };
}

/** Read the included tasks of a project item. */
function tasksOf(item: Item): readonly Item[] {
    return item.with["tasks"] ?? [];
}
