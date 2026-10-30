import { expect, test } from "@destack/test";
import { nth, schema } from "@destack/schema";
import { DatabaseError, defineRelations, defineTable, eq, json, TABLE, text } from "@destack/db";
import { Feed } from "./feed.ts";
import type { Stream } from "./stream.ts";
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
    START,
} from "../test/fixture.ts";
import type { Item, Query } from "../query/query.ts";
import { Simulation } from "@destack/environment/simulation";

/** The commits the stress test writes. */
const STRESS_COMMITS = 100;

/** The streams the stress test follows one query with. */
const STRESS_STREAMS = 50;

test("repeat a caught-up stream's position at each heartbeat as the clock advances", async () => {
    // follow the notes of a scope to their snapshot on a stopped clock
    const simulation = new Simulation({ seed: 1, start: 1000 });
    const { clock } = simulation;
    const database = await open({ simulation });
    const feed = new Feed(database, [note], { heartbeat: 60_000 });
    const signal = AbortSignal.timeout(5000);
    const pages = feed.subscribe({ notes: { table: note, scopes: ["inbox"] } }, undefined, signal);
    let snapshot = await nextValue(pages);
    while (!snapshot.complete) {
        snapshot = await nextValue(pages);
    }

    // advance the clock past one heartbeat, and read the repeated position as the next page
    const repeated = nextValue(pages);
    await simulation.jump(clock.now() + 60_000);
    expect(await repeated).toEqual({
        reset: false,
        complete: true,
        changes: [],
        position: snapshot.position,
    });
    await pages.return(undefined);
});

test("report a stream that ends by draining as closed, the same stream it opened", async () => {
    const simulation = new Simulation({ seed: 1, start: START });
    // follow the inbox's notes, draining the stream before it starts
    const database = await open({ simulation });
    const feed = new Feed(database, [note]);
    const drain = new AbortController();
    drain.abort();
    const streams: [string, Stream][] = [];
    const pages = feed.subscribe(
        { notes: { table: note, scopes: ["inbox"] } },
        undefined,
        AbortSignal.timeout(5000),
        {
            drain: drain.signal,
            opened: (stream) => streams.push(["opened", stream]),
            closed: (stream) => streams.push(["closed", stream]),
        },
    );

    // read the stream to its end, which its first completed page arrives at
    const read: Page[] = [];
    for await (const page of pages) {
        read.push(page);
    }
    expect([
        read.at(-1)?.complete,
        streams.map(([event]) => event),
        streams[0]?.[1] === streams[1]?.[1],
    ]).toEqual([true, ["opened", "closed"], true]);
});

test("publish nothing past a commit outside a caught-up stream's queries until the next heartbeat", async () => {
    // follow the inbox's notes to their snapshot
    const simulation = new Simulation({ seed: 1, start: 1000 });
    const { clock } = simulation;
    const database = await open({ simulation });
    const feed = new Feed(database, [note], { heartbeat: 60_000 });
    const signal = AbortSignal.timeout(5000);
    const pages = feed.subscribe({ notes: { table: note, scopes: ["inbox"] } }, undefined, signal);
    let snapshot = await nextValue(pages);
    while (!snapshot.complete) {
        snapshot = await nextValue(pages);
    }

    // commit an archived note, and see the stream publish nothing once the feed read it
    await database.transaction(
        async (transaction) =>
            await transaction.insert(note).values({ ...first, id: "archived", scope: "archive" }),
    );
    const next = nextValue(pages);
    let isSettled = false;
    void next.then(() => {
        isSettled = true;
    });
    await simulation.idle();
    const isQuiet = !isSettled;

    // commit another, and read the current position at the next heartbeat
    await database.transaction(
        async (transaction) =>
            await transaction.insert(note).values({ ...first, id: "later", scope: "archive" }),
    );
    const later = await database.log.position();
    const beat = await next;
    expect({ isQuiet, beat, at: clock.now() - 1000 }).toEqual({
        isQuiet: true,
        beat: { reset: false, complete: true, changes: [], position: later },
        at: 60_000,
    });
    await pages.return(undefined);
});

test("maintain a query of one scope's matching rows", async () => {
    const simulation = new Simulation({ seed: 1, start: START });
    const database = await open({ simulation });
    const feed = new Feed(database, [note]);
    const query = { table: note, scopes: ["inbox"], where: { title: "Open" } };
    await database.transaction(
        async (transaction) =>
            await transaction.insert(note).values([
                { ...first, id: "a", title: "Open" },
                { ...first, id: "b", title: "Done" },
                { ...first, id: "c", title: "Open", scope: "archive" },
            ]),
    );

    // snapshot the matching rows of the scope
    const signal = AbortSignal.timeout(5000);
    const pages = feed.subscribe({ notes: query }, undefined, signal);
    const snapshot = nth(await take(pages, (page) => page.complete), 0);
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
                    subtitle: null,
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
    await database.transaction(
        async (transaction) =>
            await transaction.update(note).set({ title: "Open" }).where(eq(note.id, "b")),
    );
    await database.transaction(
        async (transaction) =>
            await transaction.update(note).set({ title: "Done" }).where(eq(note.id, "a")),
    );
    await database.transaction(
        async (transaction) =>
            await transaction.update(note).set({ subtitle: "Unrelated" }).where(eq(note.id, "a")),
    );
    await database.transaction(
        async (transaction) =>
            await transaction.update(note).set({ scope: "inbox" }).where(eq(note.id, "c")),
    );
    const changes = (await next).flatMap((page) =>
        page.changes.map((change) => [change.operation, change.row["id"]]),
    );
    expect(changes).toEqual([
        ["insert", "b"],
        ["delete", "a"],
        ["insert", "c"],
    ]);
});

test("feed a table declared while reading, its changes reaching a stream from then on", async () => {
    // read notes for a caught-up stream, then feed tasks as well
    const simulation = new Simulation({ seed: 1, start: START });
    const database = await open({ simulation });
    const feed = new Feed(database, [note]);
    const signal = AbortSignal.timeout(5000);
    const notes = feed.subscribe({ notes: { table: note, scopes: ["inbox"] } }, undefined, signal);
    await take(notes, (page) => page.complete);
    feed.declare([task]);

    // stream a task added after the declaration
    const query = { tasks: { table: task, scopes: ["inbox"] } };
    const tasks = feed.subscribe(query, undefined, signal);
    const snapshot = nth(await take(tasks, (page) => page.complete), 0);
    const following = feed.subscribe(query, snapshot.position, signal);
    const next = take(following, (page) => page.changes.length > 0);
    await database.transaction(
        async (transaction) =>
            await transaction.insert(task).values({
                id: "t",
                scope: "inbox",
                projectId: null,
                state: "open",
                rank: 1,
                points: null,
                title: "Task",
                isSecret: false,
            }),
    );
    const changes = (await next).flatMap((page) =>
        page.changes.map((change) => [change.table, change.operation, change.row["id"]]),
    );
    expect({ tables: feed.tables.length, snapshot: snapshot.changes, changes }).toEqual({
        tables: 2,
        snapshot: [],
        changes: [[task[TABLE].sqlName, "insert", "t"]],
    });
});

test("page a snapshot of more rows than a page and continue after its first page", async () => {
    const simulation = new Simulation({ seed: 1, start: START });
    const database = await open({ simulation });
    const feed = new Feed(database, [note], { pageSize: 4 });
    const ids = Array.from({ length: 10 }, (_, index) => `n${String(index).padStart(2, "0")}`);
    await database.transaction(
        async (transaction) =>
            await transaction.insert(note).values(ids.map((id) => ({ ...first, id }))),
    );

    // read three key-ordered pages
    const signal = AbortSignal.timeout(5000);
    const pages = await take(
        feed.subscribe({ notes: { table: note, scopes: ["inbox"] } }, undefined, signal),
        (page) => page.complete,
    );
    expect(pages.map((page) => [page.reset, page.complete, page.changes.length])).toEqual([
        [true, false, 4],
        [false, false, 4],
        [false, true, 2],
    ]);
    expect(pages.flatMap((page) => page.changes.map((change) => change.row["id"]))).toEqual(ids);
    expect(new Set(pages.map((page) => page.position.sequence)).size).toBe(1);
});

test("serve subscribers behind the feed from the log and current ones from memory", async () => {
    const simulation = new Simulation({ seed: 1, start: START });
    const database = await open({ simulation });
    const feed = new Feed(database, [note]);
    const query = { notes: { table: note, scopes: ["inbox"] } };
    const signal = AbortSignal.timeout(5000);
    const start = await database.log.position();

    // commit while a second subscriber catches up
    const current = feed.subscribe(query, start, signal);
    const seen = take(current, (page) => page.changes.some((change) => change.row["id"] === "b"));
    await database.transaction(async (transaction) => await transaction.insert(note).values(first));
    const behind = feed.subscribe(query, start, signal);
    const caught = take(behind, (page) => page.changes.some((change) => change.row["id"] === "b"));
    await database.transaction(
        async (transaction) => await transaction.insert(note).values({ ...first, id: "b" }),
    );

    // deliver the same changes to both
    expect([idsOf(await seen), idsOf(await caught)]).toEqual([
        ["a", "b"],
        ["a", "b"],
    ]);
});

test("snapshot again when the source starts a new epoch", async () => {
    const simulation = new Simulation({ seed: 1, start: START });
    const database = await open({ simulation });
    const feed = new Feed(database, [note]);
    const query = { notes: { table: note, scopes: ["inbox"] } };
    await database.transaction(async (transaction) => await transaction.insert(note).values(first));
    const signal = AbortSignal.timeout(5000);

    // continue within the epoch without a snapshot
    const before = await database.log.position();
    const resumed = feed.subscribe(query, before, signal);
    const continued = take(resumed, (page) => page.changes.length > 0);
    await database.transaction(
        async (transaction) => await transaction.insert(note).values({ ...first, id: "b" }),
    );
    const changed = (await continued).filter((page) => page.reset || page.changes.length > 0);
    expect(changed.map((page) => [page.reset, page.changes.length])).toEqual([[false, 1]]);

    // start over after a restore
    const renewed = await database.log.renew();
    const snapshot = nth(
        await take(feed.subscribe(query, before, signal), (page) => page.complete),
        0,
    );
    expect([snapshot.reset, snapshot.position.epoch, snapshot.changes.length]).toEqual([
        true,
        renewed,
        2,
    ]);
});

test("follow the head through pages merged within an interval", async () => {
    const simulation = new Simulation({ seed: 1, start: START });
    const database = await open({ simulation });
    const feed = new Feed(database, [note]);
    await database.transaction(
        async (transaction) => await transaction.insert(note).values({ ...first, id: "a" }),
    );

    // follow with merged pages
    const pages = feed.subscribe(
        { notes: { table: note, scopes: ["inbox"] } },
        undefined,
        AbortSignal.timeout(4000),
        { every: 10 },
    );
    const snapshot = await until(pages, (page) => page.complete);

    // rename, add, remove and add notes
    await database.transaction(
        async (transaction) =>
            await transaction.update(note).set({ title: "Renamed" }).where(eq(note.id, "a")),
    );
    await database.transaction(
        async (transaction) => await transaction.insert(note).values({ ...first, id: "b" }),
    );
    await database.transaction(
        async (transaction) => await transaction.delete(note).where(eq(note.id, "b")),
    );
    await database.transaction(
        async (transaction) => await transaction.insert(note).values({ ...first, id: "c" }),
    );
    const following = await until(pages, (page) =>
        page.changes.some((change) => change.row["id"] === "c"),
    );

    // store each note as the pages leave it
    const notes = new Map<unknown, unknown>();
    for (const change of [...snapshot, ...following].flatMap((page) => page.changes)) {
        // forget a deleted note
        if (change.operation === "delete") {
            notes.delete(change.row["id"]);
        }
        // leave an entered or changed note unchanged
        else {
            notes.set(change.row["id"], change.row["title"]);
        }
    }
    expect([...notes]).toEqual([
        ["a", "Renamed"],
        ["c", "First"],
    ]);
});

test("continue serving a subscriber while another stops reading", async () => {
    const simulation = new Simulation({ seed: 1, start: START });
    const database = await open({ simulation });
    const feed = new Feed(database, [note]);
    const query = { table: note, scopes: ["inbox"] };
    const signal = AbortSignal.timeout(5000);

    // let one subscriber stop after its snapshot
    const idle = feed.subscribe({ query }, undefined, signal);
    await idle.next();
    const busy = feed.subscribe({ query }, undefined, signal);
    await busy.next();
    for (let index = 0; index < 20; index += 1) {
        await database.transaction(
            async (transaction) =>
                await transaction.insert(note).values({ ...first, id: `n${index}` }),
        );
        expect((await busy.next()).done).toBe(false);
    }
});

test("end every subscriber with the failure once the database closes", async () => {
    const simulation = new Simulation({ seed: 1, start: START });
    const database = await open({ simulation });
    const feed = new Feed(database, [note]);
    const query = { table: note, scopes: ["inbox"] };
    const signal = AbortSignal.timeout(5000);
    const closed = ["CONNECTION_CLOSED", "the connection is closing or closed"];

    // follow the head with one subscriber page by page and one through merged pages
    const plain = feed.subscribe({ query }, undefined, signal);
    await until(plain, (page) => page.complete);
    const merged = feed.subscribe({ query }, undefined, signal, { every: 10 });
    await until(merged, (page) => page.complete);
    await database.transaction(async (transaction) => await transaction.insert(note).values(first));
    await until(plain, (page) => page.changes.length > 0);
    await until(merged, (page) => page.changes.length > 0);

    // close the database under both, wait for the feed to fail, and let rejections propagate
    await database.close();
    const failure = await failureOf(feed.next(feed.sequence, signal));
    await simulation.idle();

    // end both subscribers with the closed connection, any page read first carrying no change
    const ended = await Promise.all([plain, merged].map((pages) => endOf(pages)));
    expect([
        failure,
        ended.map(({ failure: end }) => end),
        ended.flatMap(({ changes }) => changes),
    ]).toEqual([closed, [closed, closed], []]);
});

test("watch an ordered query reusing unchanged rows and placing a changed row by its order", async () => {
    const simulation = new Simulation({ seed: 1, start: START });
    const database = await open({ simulation });
    const feed = new Feed(database, [note]);
    const query = {
        table: note,
        scopes: ["inbox"],
        orderBy: { title: "asc" },
    } satisfies Query;
    await database.transaction(
        async (transaction) =>
            await transaction.insert(note).values([
                { ...first, id: "a", title: "Apples" },
                { ...first, id: "b", title: "Bread" },
                { ...first, id: "c", title: "Cheese" },
            ]),
    );

    // read the rows and rename the first one past the others
    const watching = feed.watch("notes", query, AbortSignal.timeout(5000));
    const initial = await nextValue(watching);
    await database.transaction(
        async (transaction) =>
            await transaction.update(note).set({ title: "Dates" }).where(eq(note.id, "a")),
    );
    const renamed = await nextValue(watching);
    await watching.return(undefined);

    // reuse the unchanged rows as the same objects, and place the renamed one last
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
});

test("watch projects with their tasks, reordering one project's tasks and leaving the other project as it was", async () => {
    const simulation = new Simulation({ seed: 1, start: START });
    const database = await open({ simulation });
    const feed = new Feed(database, [project, task]);
    const query = {
        table: project,
        scopes: ["inbox"],
        relations,
        orderBy: { name: "asc" },
        with: { tasks: { orderBy: { rank: "asc" } } },
    } satisfies Query;
    await database.transaction(
        async (transaction) =>
            await transaction.insert(project).values([
                { id: "home", scope: "inbox", name: "Home" },
                { id: "work", scope: "inbox", name: "Work" },
            ]),
    );
    await database.transaction(
        async (transaction) =>
            await transaction
                .insert(task)
                .values([
                    taskOf("dishes", "home", 1),
                    taskOf("laundry", "home", 2),
                    taskOf("report", "work", 1),
                ]),
    );

    // read both projects and move the first home task last
    const watching = feed.watch("projects", query, AbortSignal.timeout(5000));
    const initial = await nextValue(watching);
    await database.transaction(
        async (transaction) =>
            await transaction.update(task).set({ rank: 3 }).where(eq(task.id, "dishes")),
    );
    const moved = await nextValue(watching);
    await watching.return(undefined);

    // reorder the home project's tasks, and reuse the work project and its tasks as before
    expect([
        moved.map((item) => tasksOf(item).map((entry) => entry.row["title"])),
        moved[1] === initial[1],
        moved[0] === initial[0],
        tasksOf(nth(moved, 0))[0] === tasksOf(nth(initial, 0))[1],
    ]).toEqual([[["laundry", "dishes"], ["report"]], true, false, true]);
});

test("watch a board whose JSON columns store an object and a list of objects apart from its included cards", async () => {
    const simulation = new Simulation({ seed: 1, start: START });
    const database = await open({ simulation, tables: [board, card] });
    const feed = new Feed(database, [board, card]);
    const query = {
        table: board,
        scopes: ["inbox"],
        relations: BOARD_RELATIONS,
        with: { cards: { orderBy: { id: "asc" } } },
    } satisfies Query;
    await database.transaction(
        async (transaction) =>
            await transaction.insert(board).values({
                id: "plan",
                scope: "inbox",
                layout: { id: "grid", columns: 3 },
                steps: [{ id: "draft" }, { id: "review" }],
            }),
    );
    await database.transaction(
        async (transaction) =>
            await transaction.insert(card).values([
                { id: "one", scope: "inbox", boardId: "plan" },
                { id: "two", scope: "inbox", boardId: "plan" },
            ]),
    );

    // read the board with its cards
    const watching = feed.watch("boards", query, AbortSignal.timeout(5000));
    const [read] = await nextValue(watching);
    await watching.return(undefined);

    // store the JSON values in the row and the cards apart from them
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
});

test("resume at once past commits of tables the feed leaves out", async () => {
    const simulation = new Simulation({ seed: 1, start: START });
    const database = await open({ simulation });
    const feed = new Feed(database, [note]);
    const query = { table: note, scopes: ["inbox"] };
    await database.transaction(async (transaction) => await transaction.insert(note).values(first));

    // continue the feed reading from the snapshot's position
    const signal = AbortSignal.timeout(5000);
    const reading = feed.subscribe({ notes: query }, undefined, signal);
    const snapshot = await nextValue(reading);

    // commit to another table and resume from the snapshot's position
    await database.transaction(
        async (transaction) =>
            await transaction.insert(project).values({ id: "p1", scope: "inbox", name: "Plan" }),
    );
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
});

test("serve many streams of one query from the one read of the log each commit takes", async () => {
    const simulation = new Simulation({ seed: 1, start: START });
    // count a sole writer's statements, which knows each commit's head without reading it
    const database = await open({ simulation, dialect: "sqlite" });
    const query = { table: note, scopes: ["inbox"] };

    // snapshot the query in some streams, then count the source's statements per commit
    const cost = async (streams: number, prefix: string) => {
        const feed = new Feed(database, [note]);
        const controller = new AbortController();
        const received = Array.from({ length: streams }, () => new Set<unknown>());
        const snapshots = received.map(() => Promise.withResolvers<void>());
        const following = received.map(async (ids, index) => {
            const pages = feed.subscribe({ notes: query }, undefined, controller.signal);
            for await (const page of pages) {
                page.changes.forEach((change) => ids.add(change.row["id"]));
                if (page.complete) {
                    nth(snapshots, index).resolve();
                }
                if (
                    [...ids].filter((id) => String(id).startsWith(prefix)).length === STRESS_COMMITS
                ) {
                    return;
                }
            }
        });
        await Promise.all(snapshots.map((snapshot) => snapshot.promise));
        const before = database.driver.state.statements;
        for (let index = 0; index < STRESS_COMMITS; index++) {
            await database.transaction(
                async (transaction) =>
                    await transaction.insert(note).values({ ...first, id: `${prefix}-${index}` }),
            );
        }
        await Promise.all(following);
        controller.abort();

        return database.driver.state.statements - before;
    };
    const single = await cost(1, "single");
    const many = await cost(STRESS_STREAMS, "many");

    // spend one transaction, its insert and one feed read per commit, and two to start
    expect([single, many]).toEqual([STRESS_COMMITS * 3 + 2, STRESS_COMMITS * 3 + 2]);
});

/** Boards with JSON columns of an object and a list of objects. */
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

/** Read a subscriber's pages to its end: its failure, if any, and the changes read. */
async function endOf(
    pages: AsyncIterator<Page>,
): Promise<{ readonly failure: unknown; readonly changes: readonly unknown[] }> {
    const changes: unknown[] = [];
    for (;;) {
        // read the next page, or the failure once a read fails
        const read = await pages.next().then(
            (result) => ({ result }),
            (error: unknown) => ({
                failure: DatabaseError.is(error) ? [error.reason, error.message] : error,
            }),
        );

        // answer the failure
        if ("failure" in read) {
            return { failure: read.failure, changes };
        }
        // answer a clean end
        else if (read.result.done === true) {
            return { failure: undefined, changes };
        }
        // take a page's changes and read on
        else {
            changes.push(...read.result.value.changes);
        }
    }
}

/** Read the code and message of a read's database failure, absent once it succeeds. */
function failureOf(read: Promise<unknown>): Promise<unknown> {
    return read.then(
        () => undefined,
        (error: unknown) => (DatabaseError.is(error) ? [error.reason, error.message] : error),
    );
}

test("end a stream drained while it waits at a completed page of its position", async () => {
    const simulation = new Simulation({ seed: 1, start: START });
    // follow the inbox's notes to their snapshot, then resume from its position with a drain
    const database = await open({ simulation });
    const feed = new Feed(database, [note]);
    const { signal } = new AbortController();
    const snapshots = feed.subscribe(
        { notes: { table: note, scopes: ["inbox"] } },
        undefined,
        signal,
    );
    let snapshot = await nextValue(snapshots);
    while (!snapshot.complete) {
        snapshot = await nextValue(snapshots);
    }
    await snapshots.return(undefined);
    const drain = new AbortController();
    const pages = feed.subscribe(
        { notes: { table: note, scopes: ["inbox"] } },
        snapshot.position,
        signal,
        { drain: drain.signal },
    );

    // drain it while it waits, and read it to its end
    const read: Page[] = [];
    const reading = (async () => {
        for await (const page of pages) {
            read.push(page);
        }
    })();
    await simulation.scheduler.yield();
    drain.abort();
    await reading;
    expect(read).toEqual([
        { reset: false, complete: true, changes: [], position: snapshot.position },
    ]);
});

test("place a stream resumed at the log's head at once, with a completed page of its position", async () => {
    // commit a note, then resume the inbox's notes at the head with a heartbeat a minute away
    const simulation = new Simulation({ seed: 1, start: 1000 });
    const database = await open({ simulation });
    const feed = new Feed(database, [note], { heartbeat: 60_000 });
    await database.transaction(async (transaction) => await transaction.insert(note).values(first));
    const head = await database.log.position();
    const pages = feed.subscribe(
        { notes: { table: note, scopes: ["inbox"] } },
        head,
        AbortSignal.timeout(5000),
    );

    // read the first page before the clock moves
    const placed = await nextValue(pages);
    expect({ placed, now: simulation.clock.now() }).toEqual({
        placed: { reset: false, complete: true, changes: [], position: head },
        now: 1000,
    });
});
