import { expect, test, onTestFinished } from "@destack/test";
import { Condition } from "@destack/db/query";
import { Expression } from "@destack/schema/expression";
import { TEST_DIALECTS } from "@destack/db/test";
import { RequestId } from "@destack/service/request";
import { Device, serveNotes, spaceId } from "./fixture/device.ts";
import { note, notebook } from "./fixture/notes.ts";
import { page } from "./fixture/pages.ts";
import { ObjectType } from "../src/index.ts";

test.for(TEST_DIALECTS)(
    "read a query's rows, includes and aggregates with predictions before the server confirms them on %s",
    async (dialect) => {
        const { endpoint } = await serveNotes(dialect);
        const device = await Device.open("alice", endpoint("alice"), []);

        // read notebooks with their first two notes and note counts
        const books = device.client.subscribe(notebook, {
            order: [{ column: "name", direction: "asc" }],
            include: {
                notes: { order: [{ column: "title", direction: "asc" }], limit: 2 },
                size: { via: "notes", aggregate: { values: { notes: { function: "count" } } } },
            },
        });
        const counts = device.client.subscribe(note, {
            where: Condition.ne("title", ""),
            aggregate: { groupBy: ["parentId"], values: { notes: { function: "count" } } },
        });
        const shelf = async () =>
            (await books.read()).map((book) => [
                book.name,
                (book.notes as { title: string }[]).map((entry) => entry.title),
                book.size,
            ]);

        // predict a notebook with three notes offline and read them at once
        const created = device.client.mutation(async (mutation) => {
            const book = await mutation.call(notebook).create({ name: "Travel" });
            for (const title of ["Tickets", "Packing", "Maps"]) {
                await mutation.call(note).create({ parentId: book.id, title });
            }

            return book.id;
        });
        const bookId = await created.predicted;
        expect(await shelf()).toEqual([["Travel", ["Maps", "Packing"], { notes: 3 }]]);
        expect(await counts.read()).toEqual([
            { group: { parentId: bookId }, values: { notes: 3 } },
        ]);

        // hold the same once the server executed them and the copy holds its rows
        device.online();
        await created.confirmed;
        await Promise.all([books.ready, counts.ready]);
        expect([await shelf(), await counts.read()]).toEqual([
            [["Travel", ["Maps", "Packing"], { notes: 3 }]],
            [{ group: { parentId: bookId }, values: { notes: 3 } }],
        ]);

        // count a fourth note at once, before the server counts it
        const fourth = device.client.mutate(note).create({ parentId: bookId, title: "Visa" });
        await fourth.predicted;
        expect([await counts.read(), await shelf()]).toEqual([
            [{ group: { parentId: bookId }, values: { notes: 4 } }],
            [["Travel", ["Maps", "Packing"], { notes: 4 }]],
        ]);
        await fourth.confirmed;
        expect([await counts.read(), await shelf()]).toEqual([
            [{ group: { parentId: bookId }, values: { notes: 4 } }],
            [["Travel", ["Maps", "Packing"], { notes: 4 }]],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "watch a query's rows, includes and counts change with each local commit on %s",
    async (dialect) => {
        const { endpoint } = await serveNotes(dialect);
        const device = await Device.open("alice", endpoint("alice"), []);
        const books = device.client.subscribe(notebook, {
            order: [{ column: "name", direction: "asc" }],
            include: {
                notes: { order: [{ column: "title", direction: "asc" }] },
                size: { via: "notes", aggregate: { values: { notes: { function: "count" } } } },
            },
        });
        const controller = new AbortController();
        const watched = books.watch(controller.signal);
        const shelf = async () =>
            ((await watched.next()).value as Record<string, unknown>[]).map((book) => [
                book.name,
                (book.notes as { title: string }[]).map((entry) => entry.title),
                book.size,
            ]);

        // read nothing, then each notebook after its prediction
        expect(await shelf()).toEqual([]);
        const work = await device.client.mutate(notebook).create({ name: "Work" }).predicted;
        expect(await shelf()).toEqual([["Work", [], { notes: 0 }]]);

        // follow a predicted note into its notebook and its count
        await device.client.mutate(note).create({ parentId: work.id, title: "Plan" }).predicted;
        expect(await shelf()).toEqual([["Work", ["Plan"], { notes: 1 }]]);
        controller.abort();
    },
);

test.for(TEST_DIALECTS)(
    "list a query's rows page by page with what they include beside them, and its groups, on %s",
    async (dialect) => {
        const { connect } = await serveNotes(dialect);
        const alice = connect("alice");

        // file two notebooks with notes
        const book = async (name: string, titles: readonly string[]) => {
            const created = await alice.notebook.create({
                spaceId,
                requestId: RequestId.create(),
                name,
            });
            for (const title of titles) {
                await alice.note.create({
                    spaceId,
                    requestId: RequestId.create(),
                    parentId: created.id,
                    title,
                });
            }

            return created.id;
        };
        const home = await book("Home", ["Chores", "Budget", "Garden"]);
        const travel = await book("Travel", ["Maps"]);

        // list one notebook per page, with its first two notes and its note count beside it
        const query = {
            spaceId,
            order: [{ column: "name", direction: "asc" as const }],
            limit: 1,
            include: {
                notes: { order: [{ column: "title", direction: "asc" as const }], limit: 2 },
                size: {
                    via: "notes",
                    aggregate: { values: { notes: { function: "count" as const } } },
                },
            },
        };
        const first = await alice.notebook.list(query);
        const second = await alice.notebook.list({ ...query, cursor: first.cursor! });
        const titles = (included: unknown) =>
            (included as { title: string }[]).map((note) => note.title);
        expect([
            first.items.map((item) => item.name),
            titles(first.included!.notes![home]),
            first.included!.size![home],
            second.items.map((item) => item.name),
            titles(second.included!.notes![travel]),
            second.cursor,
        ]).toEqual([["Home"], ["Budget", "Chores"], { notes: 3 }, ["Travel"], ["Maps"], null]);

        // measure the notes per notebook instead of listing them
        const counted = await alice.note.list({
            spaceId,
            aggregate: { groupBy: ["parentId"], values: { notes: { function: "count" } } },
        });
        expect(
            [...counted.groups!].sort((left, right) =>
                left.values.notes! > right.values.notes! ? -1 : 1,
            ),
        ).toEqual([
            { group: { parentId: home }, values: { notes: 3 } },
            { group: { parentId: travel }, values: { notes: 1 } },
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "list and read a query filtered, sorted and paged by computed values, with the values beside each row, on %s",
    async (dialect) => {
        const { connect, endpoint } = await serveNotes(dialect);
        const alice = connect("alice");

        // file notebooks holding three, one and no notes
        const book = async (name: string, notes: number) => {
            const created = await alice.notebook.create({
                spaceId,
                requestId: RequestId.create(),
                name,
            });
            for (let index = 0; index < notes; index += 1) {
                await alice.note.create({
                    spaceId,
                    requestId: RequestId.create(),
                    parentId: created.id,
                });
            }

            return created.id;
        };
        const home = await book("Home", 3);
        const travel = await book("Travel", 1);
        await book("Empty", 0);

        // page the notebooks holding notes, fullest first, with their share of ten notes
        const compute = {
            share: Expression.divide(Expression.column("noteCount"), Expression.literal(10)),
            label: Expression.coalesce(Expression.column("name"), Expression.literal("untitled")),
        };
        const query = {
            spaceId,
            compute,
            where: Condition.gt("share", 0),
            order: [{ column: "share", direction: "desc" as const }],
            limit: 1,
        };
        const first = await alice.notebook.list(query);
        const second = await alice.notebook.list({ ...query, cursor: first.cursor! });
        expect([
            first.items.map((item) => item.name),
            first.computed,
            second.items.map((item) => item.name),
            second.computed,
            second.cursor,
        ]).toEqual([
            ["Home"],
            { [home]: { share: 0.3, label: "Home" } },
            ["Travel"],
            { [travel]: { share: 0.1, label: "Travel" } },
            null,
        ]);

        // read the same query from a local copy, the computed values inline
        const device = await Device.open("alice", endpoint("alice"), []);
        device.online();
        const local = device.client.subscribe(notebook, {
            compute,
            where: Condition.gt("share", 0),
            order: query.order,
        });
        await local.ready;
        expect((await local.read()).map((row) => [row.name, row.share, row.label])).toEqual([
            ["Home", 0.3, "Home"],
            ["Travel", 0.1, "Travel"],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "list and read the notebooks holding a pinned note outside the trash on %s",
    async (dialect) => {
        const { connect, endpoint } = await serveNotes(dialect);
        const alice = connect("alice");

        // file notebooks with a pinned, a trashed pinned, and no pinned note
        const book = async (name: string, pinned: boolean) => {
            const created = await alice.notebook.create({
                spaceId,
                requestId: RequestId.create(),
                name,
            });
            const filed = await alice.note.create({
                spaceId,
                requestId: RequestId.create(),
                parentId: created.id,
                pinned,
            });

            return { id: created.id, note: filed.id };
        };
        await book("Home", true);
        const trashed = await book("Travel", true);
        await book("Work", false);
        await alice.note.delete({ spaceId, id: trashed.note, requestId: RequestId.create() });

        // list them on the server, and read them from a device's copy
        const query = {
            where: Condition.exists("notes", Condition.eq("pinned", true)),
            order: [{ column: "name", direction: "asc" as const }],
        };
        const listed = await alice.notebook.list({ spaceId, ...query });
        const device = await Device.open("alice", endpoint("alice"), []);
        device.online();
        const local = device.client.subscribe(notebook, query);
        await local.ready;
        expect([
            listed.items.map((item) => item.name),
            (await local.read()).map((row) => row.name),
        ]).toEqual([["Home"], ["Home"]]);
    },
);

test.for(TEST_DIALECTS)(
    "follow the newest notebooks and notes as one feed on %s",
    async (dialect) => {
        const { endpoint } = await serveNotes(dialect);
        const device = await Device.open("alice", endpoint("alice"), []);

        // follow notebooks and notes together, newest first, three at most
        const activity = device.client.union(
            { notebooks: { object: notebook }, notes: { object: note } },
            { order: [{ column: "createdAt", direction: "desc" }], limit: 3 },
        );
        const controller = new AbortController();
        const watched = activity.watch(controller.signal);
        const names = (entries: readonly Readonly<Record<string, unknown>>[]) =>
            entries.map((entry) => {
                const row = entry.row as { name?: string; title?: string };

                return `${String(entry.name)}:${row.name ?? row.title}`;
            });

        // file a notebook with two notes, a millisecond apart, and read the newest three
        const book = await device.client.mutate(notebook).create({ name: "Home" }).predicted;
        for (const title of ["Chores", "Budget"]) {
            await new Promise((resolve) => setTimeout(resolve, 2));
            await device.client.mutate(note).create({ parentId: book.id, title }).predicted;
        }
        await new Promise((resolve) => setTimeout(resolve, 2));
        await device.client.mutate(notebook).create({ name: "Travel" }).predicted;
        expect(names(await activity.read())).toEqual([
            "notebooks:Travel",
            "notes:Budget",
            "notes:Chores",
        ]);

        // watch the same feed as local commits change it
        let latest = names((await watched.next()).value!);
        while (latest.length < 3) {
            latest = names((await watched.next()).value!);
        }
        expect(latest).toEqual(["notebooks:Travel", "notes:Budget", "notes:Chores"]);
        controller.abort();
        await activity.close();
    },
);

test.for(TEST_DIALECTS)(
    "list and read notes sorted by their notebook's name on %s",
    async (dialect) => {
        const { connect, endpoint } = await serveNotes(dialect);
        const alice = connect("alice");

        // file notes in two notebooks, the later notebook first by name
        const book = async (name: string, titles: readonly string[]) => {
            const created = await alice.notebook.create({
                spaceId,
                requestId: RequestId.create(),
                name,
            });
            for (const title of titles) {
                await alice.note.create({
                    spaceId,
                    requestId: RequestId.create(),
                    parentId: created.id,
                    title,
                });
            }
        };
        await book("Work", ["Plan"]);
        await book("Home", ["Garden", "Budget"]);

        // sort by the notebook's name, then the title, on the server and in a device's copy
        const query = {
            compute: { notebook: Expression.lookup("parent", "name") },
            order: [
                { column: "notebook", direction: "asc" as const },
                { column: "title", direction: "asc" as const },
            ],
        };
        const listed = await alice.note.list({ spaceId, ...query });
        const device = await Device.open("alice", endpoint("alice"), []);
        device.online();
        const local = device.client.subscribe(note, query);
        await local.ready;
        expect([
            listed.items.map((item) => [listed.computed![item.id]!.notebook, item.title]),
            (await local.read()).map((row) => [row.notebook, row.title]),
        ]).toEqual([
            [
                ["Home", "Budget"],
                ["Home", "Garden"],
                ["Work", "Plan"],
            ],
            [
                ["Home", "Budget"],
                ["Home", "Garden"],
                ["Work", "Plan"],
            ],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "list and read notebooks by how many pinned notes outside the trash they hold on %s",
    async (dialect) => {
        const { connect, endpoint } = await serveNotes(dialect);
        const alice = connect("alice");

        // file notebooks holding two, one and no pinned notes, one of Work's in the trash
        const book = async (name: string, pinned: number) => {
            const created = await alice.notebook.create({
                spaceId,
                requestId: RequestId.create(),
                name,
            });
            const notes: string[] = [];
            for (let index = 0; index < pinned; index += 1) {
                const filed = await alice.note.create({
                    spaceId,
                    requestId: RequestId.create(),
                    parentId: created.id,
                    pinned: true,
                });
                notes.push(filed.id);
            }

            return notes;
        };
        await book("Home", 2);
        const work = await book("Work", 2);
        await book("Travel", 0);
        await alice.note.delete({ spaceId, id: work[0]!, requestId: RequestId.create() });

        // order them by their pinned notes, then by name, on the server and in a device's copy
        const query = {
            compute: {
                pinned: Expression.rollup(
                    "count",
                    "notes",
                    undefined,
                    Condition.eq("pinned", true),
                ),
            },
            order: [
                { column: "pinned", direction: "desc" as const },
                { column: "name", direction: "asc" as const },
            ],
        };
        const listed = await alice.notebook.list({ spaceId, ...query });
        const device = await Device.open("alice", endpoint("alice"), []);
        device.online();
        const local = device.client.subscribe(notebook, query);
        await local.ready;
        expect([
            listed.items.map((item) => [item.name, listed.computed![item.id]!.pinned]),
            (await local.read()).map((row) => [row.name, row.pinned]),
        ]).toEqual([
            [
                ["Home", 2],
                ["Work", 1],
                ["Travel", 0],
            ],
            [
                ["Home", 2],
                ["Work", 1],
                ["Travel", 0],
            ],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "keep following a closed query for a while, receiving the server's changes with no query open, on %s",
    async (dialect) => {
        const { connect, endpoint } = await serveNotes(dialect);
        const alice = connect("alice");
        const device = await Device.open("alice", endpoint("alice"), []);
        device.online();

        // close the query and keep it followed for a minute
        const kept = device.client.subscribe(notebook, {}, { keep: { minutes: 1 } });
        await kept.ready;
        await kept.close();

        // receive a notebook the server creates after the close into the local copy
        await alice.notebook.create({ spaceId, requestId: RequestId.create(), name: "Later" });
        const database = device.client.database;
        const held = await database.log.until(
            async () =>
                (await database.select().from(notebook.table)).some((row) => row.name === "Later"),
            AbortSignal.timeout(4000),
        );
        expect(held).toBe(true);
    },
);

test.for(TEST_DIALECTS)(
    "let go of kept queries beyond the storage budget, keeping queries kept always until released, on %s",
    async (dialect) => {
        const { connect, endpoint } = await serveNotes(dialect);
        const alice = connect("alice");
        for (const name of ["Home", "Work", "Travel"]) {
            await alice.notebook.create({ spaceId, requestId: RequestId.create(), name });
        }
        const device = await Device.open("alice", endpoint("alice"), [], { storage: { rows: 2 } });
        device.online();
        const database = device.client.database;
        const names = async () =>
            (await database.select().from(notebook.table)).map((row) => row.name).sort();

        // keep home always and every notebook for a minute, then close both
        const home = { where: Condition.eq("name", "Home") };
        const always = device.client.subscribe(notebook, home, { keep: "always" });
        const minute = device.client.subscribe(notebook, {}, { keep: { minutes: 1 } });
        await Promise.all([always.ready, minute.ready]);
        await Promise.all([always.close(), minute.close()]);

        // evict notebooks beyond the budget, then home once released
        const waiting = new AbortController();
        onTestFinished(() => waiting.abort());
        const isEvicted = await database.log.until(
            async () => (await names()).length === 1,
            waiting.signal,
        );
        const held = await names();
        await device.client.release(notebook, home);
        const isReleased = await database.log.until(
            async () => (await names()).length === 0,
            waiting.signal,
        );
        expect([isEvicted, held, isReleased]).toEqual([true, ["Home"], true]);
    },
);

test.for(TEST_DIALECTS)(
    "describe what a client's copy holds, the queries it follows and what waits for the server on %s",
    async (dialect) => {
        const { connect, endpoint } = await serveNotes(dialect);
        const alice = connect("alice");
        await alice.notebook.create({ spaceId, requestId: RequestId.create(), name: "Home" });
        const device = await Device.open("alice", endpoint("alice"), [], { storage: { rows: 10 } });
        device.online();

        // follow the notebooks, then keep them always once closed
        const notebooks = device.client.subscribe(notebook, {}, { keep: "always" });
        await notebooks.ready;
        await notebooks.close();

        // describe the budgeted copy, the query kept always, and an empty outbox
        const inspection = await device.client.inspect();
        expect([
            inspection.rows,
            inspection.storage,
            inspection.subscriptions.map((entry) => entry.keep),
            inspection.replica.isShaped,
            inspection.replica.queries.length,
            inspection.outbox,
        ]).toEqual([
            { notebook: 1, note: 0 },
            { rows: 10 },
            ["always"],
            true,
            1,
            { pending: 0, executed: 0, rejected: 0 },
        ]);
    },
);

test("compile includes of a handled tree and its parent like the declared types", () => {
    // serve copies with handlers and parents and trees of the declared types
    const handledPage = page.handle({});
    const handledNotebook = notebook.handle({});
    const queries = ObjectType.queries(
        [handledPage, handledNotebook, note],
        {
            tree: { object: "page", include: { descendants: {} } },
            filed: { object: "notebook", include: { notes: {} } },
        },
        [spaceId],
    );

    // join the tree's descendants and the notebook's notes
    expect([queries.tree!.include!.descendants!.on, queries.filed!.include!.notes!.on]).toEqual([
        { kind: "descendants", column: "parentId" },
        { kind: "key", column: "parentId", parent: "id" },
    ]);
});
