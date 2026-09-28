import { expect, test } from "@destack/test";
import { TEST_DIALECTS } from "@destack/db/test";
import { eq, TABLE } from "@destack/db";
import { Condition } from "@destack/db/query";
import { Feed } from "./feed.ts";
import type { QueryPage } from "../query/page.ts";
import { first, note, open, take, until } from "../test/fixture.ts";

test.for(TEST_DIALECTS)("keep a query of one scope's matching rows on %s", async (dialect) => {
    const database = await open(dialect);
    const feed = new Feed(database, [note]);
    const query = { table: note, scopes: ["inbox"], where: Condition.eq("title", "Open") };
    await database.insert(note).values([
        { ...first, id: "a", title: "Open" },
        { ...first, id: "b", title: "Done" },
        { ...first, id: "c", title: "Open", scope: "archive" },
    ]);

    // snapshot the matching rows of the scope, as JSON of their logged columns
    const signal = AbortSignal.timeout(5000);
    const pages = feed.subscribe({ notes: query }, undefined, signal);
    const [snapshot] = await take(pages, (page) => page.complete);
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
                },
            },
        ],
        position: await database.log.position(),
    });

    // enter, leave and move between scopes, never revealing rows outside the query
    const following = feed.subscribe({ notes: query }, snapshot!.position, signal);
    const next = take(following, (page) => page.changes.some((change) => change.row.id === "c"));
    await database.update(note).set({ title: "Open" }).where(eq(note.id, "b"));
    await database.update(note).set({ title: "Done" }).where(eq(note.id, "a"));
    await database.update(note).set({ summary: "Unrelated" }).where(eq(note.id, "a"));
    await database.update(note).set({ scope: "inbox" }).where(eq(note.id, "c"));
    const changes = (await next).flatMap((page) =>
        page.changes.map((change) => [change.operation, change.row.id]),
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

        // read three key-ordered pages, starting fresh and completing with the last
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
        expect(pages.flatMap((page) => page.changes.map((change) => change.row.id))).toEqual(ids);
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

        // keep the feed reading, then commit before and while a second subscriber catches up
        const current = feed.subscribe(query, start, signal);
        const seen = take(current, (page) => page.changes.some((change) => change.row.id === "b"));
        await database.insert(note).values(first);
        const behind = feed.subscribe(query, start, signal);
        const caught = take(behind, (page) => page.changes.some((change) => change.row.id === "b"));
        await database.insert(note).values({ ...first, id: "b" });

        // deliver the same changes to both
        const ids = async (pages: Promise<QueryPage[]>) =>
            (await pages).flatMap((page) => page.changes.map((change) => change.row.id));
        expect([await ids(seen), await ids(caught)]).toEqual([
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
        const carrying = (await continued).filter((page) => page.reset || page.changes.length > 0);
        expect(carrying.map((page) => [page.reset, page.changes.length])).toEqual([[false, 1]]);

        // start over from a snapshot of the restored history
        const renewed = await database.log.renew();
        const [snapshot] = await take(
            feed.subscribe(query, before, signal),
            (page) => page.complete,
        );
        expect([snapshot!.reset, snapshot!.position.epoch, snapshot!.changes.length]).toEqual([
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

        // follow the same query in two streams until both hold the snapshot
        const signal = AbortSignal.timeout(5000);
        const streams = [0, 1].map(() => feed.subscribe({ notes: query }, undefined, signal));
        for (const pages of streams) {
            await until(pages, (page) => page.complete);
        }

        // receive the same page for a write, decided once
        await database.update(note).set({ title: "Renamed" }).where(eq(note.id, "a"));
        const [left, right] = await Promise.all(
            streams.map(async (pages) =>
                (await until(pages, (page) => page.changes.length > 0)).at(-1),
            ),
        );
        expect([left === right, left!.changes.map((change) => change.row.title)]).toEqual([
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

        // follow the notes with pages merged within a short interval
        const pages = feed.subscribe(
            { notes: { table: note, scopes: ["inbox"] } },
            undefined,
            AbortSignal.timeout(4000),
            { every: 10 },
        );
        const snapshot = await until(pages, (page) => page.complete);

        // rename one note, add and remove another, and add a third
        await database.update(note).set({ title: "Renamed" }).where(eq(note.id, "a"));
        await database.insert(note).values({ ...first, id: "b" });
        await database.delete(note).where(eq(note.id, "b"));
        await database.insert(note).values({ ...first, id: "c" });
        const following = await until(pages, (page) =>
            page.changes.some((change) => change.row.id === "c"),
        );

        // hold each note as the pages leave it, however they merged
        const held = new Map<unknown, unknown>();
        for (const change of [...snapshot, ...following].flatMap((page) => page.changes)) {
            // forget a deleted note
            if (change.operation === "delete") {
                held.delete(change.row.id);
            }
            // hold an entered or changed note as it is
            else {
                held.set(change.row.id, change.row.title);
            }
        }
        expect([...held]).toEqual([
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

        // follow one query in two open streams, which share one evaluation at the head
        const controller = new AbortController();
        const streams = [0, 1].map(() =>
            feed.subscribe({ notes: query }, undefined, controller.signal),
        );
        const read = async (
            pages: AsyncGenerator<QueryPage>,
            until: (page: QueryPage) => boolean,
        ) => {
            while (!until((await pages.next()).value as QueryPage)) {
                // read on
            }
        };
        await Promise.all(streams.map((pages) => read(pages, (page) => page.complete)));
        const followed = Promise.all(
            streams.map((pages) => read(pages, (page) => page.changes.length > 0)),
        );
        await database.insert(note).values({ ...first, id: "b" });
        await followed;

        // describe the one shared evaluation: its root pipeline holding both notes, and its last run's one change and decision
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

        // let one subscriber read its snapshot and stop, while the other reads a page per write
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
