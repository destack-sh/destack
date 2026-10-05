import { expect, test, onTestFinished } from "@destack/test";
import { Expression, type Path } from "@destack/db";
import { TEST_DIALECTS } from "@destack/db/test";
import { RequestId } from "@destack/service/request";
import { Device, serveNotes, spaceId } from "./fixture/device.ts";
import { note, notebook } from "./fixture/note.ts";
import { page } from "./fixture/page.ts";
import { ObjectType } from "../src/index.ts";
import { QueriesParameters } from "../src/replica/replica.ts";
import { present, schema } from "@destack/schema";
import { included, type Query } from "@destack/sync";

/** The rows an include adds, read for their titles. */
const TITLED_ROWS = schema.array(schema.looseObject({ title: schema.string() }));

test.for(TEST_DIALECTS)(
    "read a query's rows, includes and aggregates with predictions before the server confirms them on %s",
    async (dialect) => {
        const { endpoint } = await serveNotes(dialect);
        const device = await Device.open("alice", endpoint("alice"), []);

        // read notebooks with their first two notes and note counts
        const books = device.client.query.notebook
            .findMany({
                orderBy: { name: "asc" },
                with: { notes: { orderBy: { title: "asc" }, limit: 2 } },
                extras: { size: Expression.rollup("count", "notes") },
            })
            .subscribe();
        const counts = device.client.query.note
            .aggregate({
                where: { title: { ne: "" } },
                groupBy: ["parentId"],
                values: { notes: { function: "count" } },
            })
            .subscribe();
        const shelf = async () =>
            (await books.read()).map((book) => [
                book.name,
                book.notes.map((entry) => entry.title),
                { notes: book.size },
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

        // keep the same once the server executed them and the copy has its rows
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
        const books = device.client.query.notebook
            .findMany({
                orderBy: { name: "asc" },
                with: { notes: { orderBy: { title: "asc" } } },
                extras: { size: Expression.rollup("count", "notes") },
            })
            .subscribe();
        const controller = new AbortController();
        const watched = books.watch(controller.signal);
        const shelf = async () => {
            const next = await watched.next();
            if (next.done === true) {
                throw new TypeError("the watch ended");
            }

            return next.value.map((book) => [
                book.name,
                book.notes.map((entry) => entry.title),
                { notes: book.size },
            ]);
        };

        // read nothing before the predictions and each notebook after its own
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
            orderBy: { name: "asc" } as const,
            limit: 1,
            with: { notes: { orderBy: { title: "asc" } as const, limit: 2 } },
            extras: { size: Expression.rollup("count", "notes") },
        };
        const first = await alice.notebook.list(query);
        const second = await alice.notebook.list({
            ...query,
            cursor: present(first.cursor, "the first page's cursor"),
        });
        expect([
            first.items.map((item) => item.name),
            includedTitles(first.included, "notes", home),
            { notes: extrasOf(first.extras, home)["size"] },
            second.items.map((item) => item.name),
            includedTitles(second.included, "notes", travel),
            second.cursor,
        ]).toEqual([["Home"], ["Budget", "Chores"], { notes: 3 }, ["Travel"], ["Maps"], null]);

        // measure the notes per notebook instead of listing them
        const counted = await alice.note.list({
            spaceId,
            aggregate: { groupBy: ["parentId"], values: { notes: { function: "count" } } },
        });
        expect(
            present(counted.groups, "the note groups").toSorted((left, right) =>
                present(left.values["notes"], "a group's note count") >
                present(right.values["notes"], "a group's note count")
                    ? -1
                    : 1,
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

        // file notebooks with three, one and no notes
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

        // page the notebooks with notes, fullest first, with their share of ten notes
        const extras = {
            share: Expression.divide(Expression.column("noteCount"), Expression.literal(10)),
            label: Expression.coalesce(Expression.column("name"), Expression.literal("untitled")),
        };
        const query = {
            spaceId,
            extras,
            where: { share: { gt: 0 } },
            orderBy: { share: "desc" } as const,
            limit: 1,
        };
        const first = await alice.notebook.list(query);
        const second = await alice.notebook.list({
            ...query,
            cursor: present(first.cursor, "the first page's cursor"),
        });
        expect([
            first.items.map((item) => item.name),
            first.extras,
            second.items.map((item) => item.name),
            second.extras,
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
        const local = device.client.query.notebook
            .findMany({
                extras,
                where: { share: { gt: 0 } },
                orderBy: query.orderBy,
            })
            .subscribe();
        await local.ready;
        expect(
            (await local.read()).map((row) => [row["name"], row["share"], row["label"]]),
        ).toEqual([
            ["Home", 0.3, "Home"],
            ["Travel", 0.1, "Travel"],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "list and read the notebooks with a pinned note outside the trash on %s",
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
            where: { notes: { pinned: true } },
            orderBy: { name: "asc" } as const,
        };
        const listed = await alice.notebook.list({ spaceId, ...query });
        const device = await Device.open("alice", endpoint("alice"), []);
        device.online();
        const local = device.client.query.notebook.findMany(query).subscribe();
        await local.ready;
        expect([
            listed.items.map((item) => item.name),
            (await local.read()).map((row) => row["name"]),
        ]).toEqual([["Home"], ["Home"]]);
    },
);

test.for(TEST_DIALECTS)(
    "follow the newest notebooks and notes as one feed on %s",
    async (dialect) => {
        const { endpoint } = await serveNotes(dialect);
        const device = await Device.open("alice", endpoint("alice"), []);

        // follow notebooks and notes together, newest first, three at most
        const { query } = device.client.of({ notebook, note });
        const activity = device.client.union(
            { notebooks: query.notebook.findMany(), notes: query.note.findMany() },
            { orderBy: { createdAt: "desc" }, limit: 3 },
        );
        const controller = new AbortController();
        const watched = activity.watch(controller.signal);
        const next = async () => {
            const read = await watched.next();
            if (read.done === true) {
                throw new TypeError("the watched activity ended");
            }

            return unionNames(read.value);
        };

        // file a notebook with two notes, a millisecond apart, and read the newest three
        const book = await device.client.mutate(notebook).create({ name: "Home" }).predicted;
        for (const title of ["Chores", "Budget"]) {
            await new Promise((resolve) => {
                setTimeout(resolve, 2);
            });
            await device.client.mutate(note).create({ parentId: book.id, title }).predicted;
        }
        await new Promise((resolve) => {
            setTimeout(resolve, 2);
        });
        await device.client.mutate(notebook).create({ name: "Travel" }).predicted;
        expect(unionNames(await activity.read())).toEqual([
            "notebooks:Travel",
            "notes:Budget",
            "notes:Chores",
        ]);

        // watch the same feed as local commits change it
        let latest = await next();
        while (latest.length < 3) {
            latest = await next();
        }
        expect(latest).toEqual(["notebooks:Travel", "notes:Budget", "notes:Chores"]);
        controller.abort();
        await activity.close();

        // refuse a member ordering itself, which the union's own order replaces
        expect(() =>
            device.client.union(
                { notebooks: query.notebook.findMany({ orderBy: { name: "asc" } }) },
                { orderBy: { createdAt: "desc" }, limit: 3 },
            ),
        ).toThrow("union member notebooks orders or limits itself");
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

        // sort by the notebook's name and the title, on the server and in a device's copy
        const query = {
            extras: { notebook: Expression.lookup("parent", "name") },
            orderBy: { notebook: "asc", title: "asc" } as const,
        };
        const listed = await alice.note.list({ spaceId, ...query });
        const device = await Device.open("alice", endpoint("alice"), []);
        device.online();
        const local = device.client.query.note.findMany(query).subscribe();
        await local.ready;
        expect([
            listed.items.map((item) => [extrasOf(listed.extras, item.id)["notebook"], item.title]),
            (await local.read()).map((row) => [row["notebook"], row["title"]]),
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
    "list and read notebooks by how many pinned notes outside the trash they have on %s",
    async (dialect) => {
        const { connect, endpoint } = await serveNotes(dialect);
        const alice = connect("alice");

        // file notebooks with two, one and no pinned notes, one of Work's in the trash
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
        await alice.note.delete({
            spaceId,
            id: present(work[0], "the first work note"),
            requestId: RequestId.create(),
        });

        // order them by their pinned notes and by name, on the server and in a device's copy
        const query = {
            extras: {
                pinned: Expression.rollup("count", "notes", undefined, { pinned: true }),
            },
            orderBy: { pinned: "desc", name: "asc" } as const,
        };
        const listed = await alice.notebook.list({ spaceId, ...query });
        const device = await Device.open("alice", endpoint("alice"), []);
        device.online();
        const local = device.client.query.notebook.findMany(query).subscribe();
        await local.ready;
        expect([
            listed.items.map((item) => [item.name, extrasOf(listed.extras, item.id)["pinned"]]),
            (await local.read()).map((row) => [row["name"], row["pinned"]]),
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
        const kept = device.client.query.notebook.findMany({}).subscribe({ keep: { minutes: 1 } });
        await kept.ready;
        await kept.close();

        // receive a notebook the server creates after the close into the local copy
        await alice.notebook.create({ spaceId, requestId: RequestId.create(), name: "Later" });
        const database = device.client.database;
        const isReceived = await database.log.until(
            async () =>
                (await database.select().from(notebook.table)).some((row) => row.name === "Later"),
            AbortSignal.timeout(4000),
        );
        expect(isReceived).toBe(true);
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
            (await database.select().from(notebook.table)).map((row) => row.name).toSorted();

        // keep home always and every notebook for a minute, and close both
        const home = { where: { name: "Home" } };
        const always = device.client.query.notebook.findMany(home).subscribe({ keep: "always" });
        const minute = device.client.query.notebook
            .findMany({})
            .subscribe({ keep: { minutes: 1 } });
        await Promise.all([always.ready, minute.ready]);
        await Promise.all([always.close(), minute.close()]);

        // evict notebooks beyond the budget, and home once released
        const waiting = new AbortController();
        onTestFinished(() => waiting.abort());
        const isEvicted = await database.log.until(
            async () => (await names()).length === 1,
            waiting.signal,
        );
        const kept = await names();
        await device.client.release(notebook, home);
        const isReleased = await database.log.until(
            async () => (await names()).length === 0,
            waiting.signal,
        );
        expect([isEvicted, kept, isReleased]).toEqual([true, ["Home"], true]);
    },
);

test.for(TEST_DIALECTS)(
    "describe what a client's copy has, the queries it follows and what waits for the server on %s",
    async (dialect) => {
        const { connect, endpoint } = await serveNotes(dialect);
        const alice = connect("alice");
        await alice.notebook.create({ spaceId, requestId: RequestId.create(), name: "Home" });
        const device = await Device.open("alice", endpoint("alice"), [], { storage: { rows: 10 } });
        device.online();

        // follow the notebooks and keep them always once closed
        const notebooks = device.client.query.notebook.findMany({}).subscribe({ keep: "always" });
        await notebooks.ready;
        await notebooks.close();

        // describe the budgeted copy, the query kept always, and an empty outbox
        const inspection = await device.client.inspect();
        expect([
            inspection.rows,
            inspection.storage,
            inspection.subscriptions.map((entry) => entry.keep),
            inspection.replica.isLaidOut,
            Object.keys(
                QueriesParameters.parse(
                    present(inspection.replica.subscription, "the copy's subscription").parameters,
                ).queries ?? {},
            ).length,
            inspection.mutations,
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
            tree: { object: "page", with: { descendants: {} } },
            filed: { object: "notebook", with: { notes: {} } },
        },
        [spaceId],
    );

    // join the tree's descendants and the notebook's notes
    expect([joinOf(queries, "tree", "descendants"), joinOf(queries, "filed", "notes")]).toEqual([
        { kind: "descendants", column: "parentId" },
        { kind: "key", column: "parentId", parent: "id" },
    ]);
});

test("watch a query keeping each unchanged row as the same object after another row changes", async () => {
    const { endpoint } = await serveNotes("sqlite");
    const device = await Device.open("alice", endpoint("alice"), []);

    // predict a notebook with three notes, and watch them by title
    const created = device.client.mutation(async (mutation) => {
        const book = await mutation.call(notebook).create({ name: "Kitchen" });
        const ids: string[] = [];
        for (const title of ["Apples", "Bread", "Cheese"]) {
            ids.push((await mutation.call(note).create({ parentId: book.id, title })).id);
        }

        return ids;
    });
    const [apples] = await created.predicted;
    const stop = new AbortController();
    onTestFinished(() => stop.abort());
    const watched = device.client.query.note
        .findMany({ orderBy: { title: "asc" } })
        .subscribe()
        .watch(stop.signal);
    const next = async () => {
        const read = await watched.next();
        if (read.done === true) {
            throw new Error("the watch ended");
        }

        return read.value;
    };
    const initial = await next();

    // rename the first note past the others
    await device.client
        .mutate(note)
        .update({ id: present(apples, "the apples note"), title: "Dates" }).predicted;
    const renamed = await next();

    // keep the two unchanged notes as the same objects, and present the renamed one anew
    expect([
        renamed.map((row) => row["title"]),
        renamed[0] === initial[1],
        renamed[1] === initial[2],
        renamed[2] === initial[0],
    ]).toEqual([["Bread", "Cheese", "Dates"], true, true, false]);
});

/** Read what a listed row includes under an include's name. */
function includedOf(
    includes: Readonly<Record<string, Readonly<Record<string, unknown>>>> | undefined,
    name: string,
    id: string,
): unknown {
    return present(present(includes, "the listed rows' includes")[name], `include ${name}`)[id];
}

/** Read the titles of the rows a listed row includes under an include's name. */
function includedTitles(
    includes: Readonly<Record<string, Readonly<Record<string, unknown>>>> | undefined,
    name: string,
    id: string,
): string[] {
    return TITLED_ROWS.parse(includedOf(includes, name, id)).map((row) => row.title);
}

/** Read the extras a listed query computes for one row. */
function extrasOf<Value>(
    extras: Readonly<Record<string, Readonly<Record<string, Value>>>> | undefined,
    id: string,
): Readonly<Record<string, Value>> {
    return present(present(extras, "the extras")[id], `the extras of ${id}`);
}

/** Read how a compiled query joins one of its includes. */
function joinOf(queries: Readonly<Record<string, Query>>, name: string, include: string): Path {
    return included(present(queries[name], `query ${name}`), include).relation.on;
}

/** Name each entry of a union by its member and its notebook's name or note's title. */
function unionNames(
    entries: readonly (
        | { readonly name: "notebooks"; readonly row: { readonly name: string } }
        | { readonly name: "notes"; readonly row: { readonly title: string } }
    )[],
): string[] {
    return entries.map((entry) =>
        entry.name === "notebooks" ? `notebooks:${entry.row.name}` : `notes:${entry.row.title}`,
    );
}
