import { nth, present, schema } from "@destack/schema";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { asc, eq, sql } from "../sql/index.ts";
import { TestDatabase, type TestDatabaseOptions } from "../test/database.ts";
import { defineTable, TABLE } from "../table/table.ts";
import { integer, json, text } from "../table/column.ts";
import { unique, uniqueIndex } from "../table/constraint.ts";
import { ChangeSet, Transaction, ReadSet } from "./commit.ts";
import { LOG } from "./schema.ts";
import { STATE } from "../migration/state.ts";
import { testSimulation } from "@destack/test/world";

/** The writers and readers the stress test interleaves: 500 each, about 75 ms on SQLite. */
const STRESS_TRANSACTIONS = 500;

/** The members the shared-read stress test commits while readers share reads: 200, tens of ms. */
const STRESS_MEMBERS = 200;

/** Tasks in folders, with a priority. */
const task = defineTable(
    "commit_task",
    {
        /** The task's key. */
        id: text("id").primaryKey(),
        /** The folder the task lives in. */
        scope: text("scope").notNull(),
        /** The priority, highest first. */
        priority: integer("priority").notNull(),
    },
    { log: {} },
);

/** Labels unique by name among the live ones of a folder, a missing folder being one folder. */
const label = defineTable(
    "commit_label",
    {
        /** The label's key. */
        id: text("id").primaryKey(),
        /** The board the label lives in. */
        scope: text("scope").notNull(),
        /** The folder, absent for the top. */
        folder: text("folder"),
        /** The name. */
        name: text("name").notNull(),
        /** When the label was archived, absent while live. */
        archivedAt: integer("archived_at"),
    },
    {
        log: {},
        constraints: (entry) => [
            uniqueIndex("commit_label_name")
                .on(entry.folder, entry.name)
                .where({ archivedAt: { isNull: true } })
                .nullsNotDistinct(),
        ],
    },
);

/** Tokens in folders, with a sensitive secret. */
const token = defineTable(
    "commit_token",
    {
        /** The token's key. */
        id: text("id").primaryKey(),
        /** The folder the token lives in. */
        scope: text("scope").notNull(),
        /** The secret, left out of the log. */
        secret: text("secret").notNull().sensitive(),
    },
    { log: {} },
);

/** A person's bookmarks, unique by the person and the place. */
const bookmark = defineTable(
    "commit_pin",
    {
        /** The bookmark's key. */
        id: text("id").primaryKey(),
        /** The board the bookmark lives in. */
        scope: text("scope").notNull(),
        /** The person pinning. */
        person: text("person").notNull(),
        /** The bookmarked place. */
        place: json("place", schema.object({ x: schema.number(), y: schema.number() })).notNull(),
    },
    { log: {}, constraints: (entry) => [unique("commit_pin_place").on(entry.person, entry.place)] },
);

/** Members of folders whose reads transactions share while the log of members stands. */
const member = defineTable(
    "commit_member",
    {
        /** The member's key. */
        id: text("id").primaryKey(),
        /** The folder the member belongs to. */
        scope: text("scope").notNull(),
        /** The member's role in the folder. */
        role: text("role").notNull(),
    },
    { log: { isShared: true } },
);

/** Roles of folders, in the log's history, whose reads transactions share. */
const role = defineTable(
    "commit_role",
    {
        /** The role's key. */
        id: text("id").primaryKey(),
        /** The folder the role belongs to. */
        scope: text("scope").notNull(),
    },
    { log: { retention: "history", isShared: true } },
);

/** Declare shared units with one predefined row of a name. */
function unitTable(name: string) {
    return defineTable(
        "commit_unit",
        {
            /** The unit's key. */
            id: text("id").primaryKey(),
            /** The system the unit belongs to. */
            scope: text("scope").notNull(),
            /** The unit's name. */
            name: text("name").notNull(),
        },
        { log: { isShared: true }, predefined: [{ id: "metre", scope: "si", name }] },
    );
}

/** Open a database with three tasks, two in the inbox and one archived. */
async function open(options: Pick<TestDatabaseOptions, "dialect"> = {}) {
    const storage = await TestDatabase.create([task], {
        environment: testSimulation().environment("test"),
        ...options,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await database.transaction((transaction) =>
        transaction
            .insert(task)
            .values([
                { id: "a", scope: "inbox", priority: 1 },
                { id: "b", scope: "inbox", priority: 2 },
                { id: "c", scope: "archive", priority: 3 },
            ])
            .execute(),
    );

    return { database, base: await database.log.position() };
}

/** Read every task in key order. */
async function tasks(database: Awaited<ReturnType<typeof open>>["database"]) {
    return database.select().from(task).orderBy(asc(task.id));
}

test("commit a transaction's changes, seeing its writes through its overlay, recording each row's first and last image and leaving rows it put back unchanged", async () => {
    const { database, base } = await open();

    // read and write through the transaction's view
    const reads = new ReadSet();
    const changes = new ChangeSet();
    const view = database.log.at(base).recording(reads);
    const first = present(await view.read(task).findFirst({ where: { id: "a" } }), "task a");
    changes.write({
        table: task,
        before: first,
        after: { id: "a", scope: "inbox", priority: 5 },
    });
    changes.write({
        table: task,
        before: { id: "a", scope: "inbox", priority: 5 },
        after: { id: "a", scope: "inbox", priority: 6 },
    });
    changes.write({
        table: task,
        before: null,
        after: { id: "d", scope: "inbox", priority: 4 },
    });
    const second = present(await view.read(task).findFirst({ where: { id: "b" } }), "task b");
    changes.write({
        table: task,
        before: second,
        after: { id: "b", scope: "inbox", priority: 9 },
    });
    changes.write({
        table: task,
        before: { id: "b", scope: "inbox", priority: 9 },
        after: second,
    });
    const seen = await view
        .layer(changes.overlay())
        .read(task)
        .findMany({ where: { scope: "inbox" } });

    // commit, writing only the changed rows
    await changes.commit(database, base, reads, async () => {});
    expect({
        seen: seen
            .map((row) => [row.id, row.priority])
            .toSorted((left, right) => String(left[0]).localeCompare(String(right[0]))),
        changes: changes.changes.map((change) => [
            change.before?.["priority"] ?? null,
            change.after?.["priority"] ?? null,
        ]),
        committed: (await tasks(database)).map((row) => [row.id, row.priority]),
    }).toEqual({
        seen: [
            ["a", 6],
            ["b", 2],
            ["d", 4],
        ],
        changes: [
            [1, 6],
            [null, 4],
        ],
        committed: [
            ["a", 6],
            ["b", 2],
            ["c", 3],
            ["d", 4],
        ],
    });
});

test("touch a merged read set with a change of either set's reads", async () => {
    const { database, base } = await open();

    // read one task by key and scan the archive into two sets, then merge the second into the first
    const merged = new ReadSet();
    await database.log
        .at(base)
        .recording(merged)
        .read(task)
        .findFirst({ where: { id: "a" } });
    const archived = new ReadSet();
    await database.log
        .at(base)
        .recording(archived)
        .read(task)
        .findMany({ where: { scope: "archive" } });
    merged.add(archived);

    // change the read task and add to the archive and the inbox, reading the changes
    await database.transaction((transaction) =>
        transaction.update(task).set({ priority: 9 }).where(eq(task.id, "a")).execute(),
    );
    await database.transaction((transaction) =>
        transaction.insert(task).values({ id: "f", scope: "archive", priority: 1 }).execute(),
    );
    await database.transaction((transaction) =>
        transaction.insert(task).values({ id: "g", scope: "inbox", priority: 1 }).execute(),
    );
    const { changes } = await database.log.read({ tables: [task], after: base.sequence });

    // touch the merged set with the first two changes alone
    expect(changes.map((change) => merged.touches(change))).toEqual([true, true, false]);
});

test("count each write of a change set to some tables, two of one row among them", () => {
    // write a row twice and another once
    const writes = new ChangeSet();
    const row = { id: "h", scope: "inbox", priority: 1 };
    writes.write({ table: task, before: null, after: row });
    writes.write({ table: task, before: row, after: { ...row, priority: 2 } });
    writes.write({ table: task, before: null, after: { ...row, id: "i" } });

    // count three writes of the tasks and none of the labels
    expect({ tasks: writes.writes([task]), labels: writes.writes([label]) }).toEqual({
        tasks: 3,
        labels: 0,
    });
});

test("refuse a transaction another commit raced on a row it read or a row its scan selects, and commit one whose reads no commit touched", async () => {
    const { database, base } = await open();

    // read one row by key, scan the inbox, and read the archive apart
    const byKey = new ReadSet();
    await database.log
        .at(base)
        .recording(byKey)
        .read(task)
        .findFirst({ where: { id: "a" } });
    const byScan = new ReadSet();
    await database.log
        .at(base)
        .recording(byScan)
        .read(task)
        .findMany({ where: { scope: "inbox" } });
    const apart = new ReadSet();
    await database.log
        .at(base)
        .recording(apart)
        .read(task)
        .findMany({ where: { scope: "archive" } });

    // change a task and add one to the inbox after the base, as other commits do
    await database.transaction((transaction) =>
        transaction.update(task).set({ priority: 7 }).where(eq(task.id, "a")).execute(),
    );
    await database.transaction((transaction) =>
        transaction.insert(task).values({ id: "e", scope: "inbox", priority: 1 }).execute(),
    );
    const write = () => {
        const changes = new ChangeSet();
        changes.write({
            table: task,
            before: null,
            after: { id: "z", scope: "archive", priority: 0 },
        });

        return changes;
    };

    // refuse the two mutations whose reads those commits touched, and commit the third
    expect([
        await refusal(write().commit(database, base, byKey, async () => {})),
        await refusal(write().commit(database, base, byScan, async () => {})),
        await write()
            .commit(database, base, apart, async () => {})
            .then(() => "committed"),
    ]).toEqual([
        [
            "SERVICE_UNAVAILABLE",
            "CONCURRENT_UPDATE",
            "another commit changed what the transaction read",
        ],
        [
            "SERVICE_UNAVAILABLE",
            "CONCURRENT_UPDATE",
            "another commit changed what the transaction read",
        ],
        "committed",
    ]);
});

test("read a transaction's rows with its writes in order up to a limit, count them, and commit its records beside its changes", async () => {
    const { database } = await open();

    // write through the transaction and read the inbox with its writes
    const transaction = await Transaction.open(database);
    const first = present(await transaction.read(task).findFirst({ where: { id: "a" } }), "task a");
    await transaction.write({
        table: task,
        before: first,
        after: { id: "a", scope: "archive", priority: 1 },
    });
    await transaction.write({
        table: task,
        before: null,
        after: { id: "d", scope: "inbox", priority: 9 },
    });
    const inbox = await transaction
        .read(task)
        .findMany({ where: { scope: "inbox" }, orderBy: { priority: "desc" }, limit: 2 });
    const archived = await transaction.count(task, { scope: "archive" });

    // commit with a record written once beside the changes
    transaction.record(async (recording) => {
        await recording.insert(task).values({ id: "r", scope: "records", priority: 0 });
    });
    await transaction.commit();
    expect({
        inbox: inbox.map((row) => row.id),
        archived,
        committed: (await tasks(database)).map((row) => [row.id, row.scope]),
    }).toEqual({
        inbox: ["d", "b"],
        archived: 2,
        committed: [
            ["a", "archive"],
            ["b", "inbox"],
            ["c", "archive"],
            ["d", "inbox"],
            ["r", "records"],
        ],
    });
});

test("show only logged columns in a transaction's images and its view, committed or written", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create([token], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await storage.database.transaction((transaction) =>
        transaction.insert(token).values({ id: "a", scope: "inbox", secret: "first" }).execute(),
    );

    // write a second token, then read both as images and through the view
    const transaction = await Transaction.open(storage.database);
    await transaction.write({
        table: token,
        before: null,
        after: { id: "b", scope: "inbox", secret: "second" },
    });
    const seen = {
        image: await transaction
            .view()
            .read(token)
            .findFirst({ where: { id: "a" } }),
        images: await transaction
            .view()
            .read(token)
            .findMany({ where: { scope: "inbox" }, orderBy: { id: "asc" } }),
        view: await transaction
            .view()
            .read(token)
            .findMany({ where: { scope: "inbox" } }),
    };

    expect(seen).toEqual({
        image: { id: "a", scope: "inbox" },
        images: [
            { id: "a", scope: "inbox" },
            { id: "b", scope: "inbox" },
        ],
        view: [
            { id: "a", scope: "inbox" },
            { id: "b", scope: "inbox" },
        ],
    });
});

test("drop a failed step's writes and records as a savepoint does, leaving the writes before it", async () => {
    const { database } = await open();

    // write once, then fail a step that writes and records
    const transaction = await Transaction.open(database);
    await transaction.write({
        table: task,
        before: null,
        after: { id: "d", scope: "inbox", priority: 4 },
    });
    const failed = await transaction
        .savepoint(async () => {
            await transaction.write({
                table: task,
                before: null,
                after: { id: "e", scope: "inbox", priority: 5 },
            });
            transaction.record(async (recording) => {
                await recording.insert(task).values({ id: "r", scope: "records", priority: 0 });
            });
            throw new Error("refused");
        })
        .catch((error: unknown) => (error instanceof Error ? error.message : "unknown"));
    await transaction.commit();

    expect({ failed, committed: (await tasks(database)).map((row) => row.id) }).toEqual({
        failed: "refused",
        committed: ["a", "b", "c", "d"],
    });
});

test("refuse a write sharing a unique key with a committed row, the transaction's or its batch's, nulls being equal and archived rows outside the key", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create([label], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await database.transaction((transaction) =>
        transaction
            .insert(label)
            .values([
                { id: "a", scope: "board", folder: null, name: "red", archivedAt: null },
                { id: "b", scope: "board", folder: null, name: "blue", archivedAt: 1 },
            ])
            .execute(),
    );

    // write labels clashing with committed, written and key values, and ones outside the key
    const transaction = await Transaction.open(database);
    const write = (row: { id: string; folder: string | null; name: string }) =>
        transaction
            .write({
                table: label,
                before: null,
                after: { ...row, scope: "board", archivedAt: null },
            })
            .then(() => "written");
    const written = [
        await refusal(write({ id: "c", folder: null, name: "red" })),
        await write({ id: "d", folder: null, name: "blue" }),
        await refusal(write({ id: "e", folder: null, name: "blue" })),
        await refusal(write({ id: "a", folder: "inbox", name: "green" })),
        await write({ id: "f", folder: "inbox", name: "red" }),
    ];

    // write a batch whose rows clash, then one moving a key from one row to another
    const row = (id: string, name: string) => ({
        table: label,
        before: null,
        after: { id, scope: "board", folder: "done", name, archivedAt: null },
    });
    const batches = [
        await refusal(transaction.write(row("g", "pink"), row("h", "pink")).then(() => "written")),
        await transaction
            .write(
                {
                    table: label,
                    before: {
                        id: "d",
                        scope: "board",
                        folder: null,
                        name: "blue",
                        archivedAt: null,
                    },
                    after: {
                        id: "d",
                        scope: "board",
                        folder: null,
                        name: "teal",
                        archivedAt: null,
                    },
                },
                { ...row("i", "blue"), after: { ...row("i", "blue").after, folder: null } },
            )
            .then(() => "written"),
    ];
    await transaction.commit();

    // write outside every key
    expect({
        written,
        batches,
        committed: (await database.select().from(label).orderBy(asc(label.id))).map((each) => [
            each.id,
            each.name,
        ]),
    }).toEqual({
        batches: [
            ["CONFLICT", "DUPLICATE", "a record with the same unique key already exists"],
            "written",
        ],
        written: [
            ["CONFLICT", "DUPLICATE", "a record with the same unique key already exists"],
            "written",
            ["CONFLICT", "DUPLICATE", "a record with the same unique key already exists"],
            ["CONFLICT", "DUPLICATE", "a record with the same unique key already exists"],
            "written",
        ],
        committed: [
            ["a", "red"],
            ["b", "blue"],
            ["d", "teal"],
            ["f", "red"],
            ["i", "blue"],
        ],
    });
});

test("rehearse writes over a transaction's rows, answering the rows they change, and drop them after", async () => {
    const { database } = await open();

    // write once, then rehearse a change of that row and of another
    const transaction = await Transaction.open(database);
    await transaction.write({
        table: task,
        before: null,
        after: { id: "d", scope: "inbox", priority: 4 },
    });
    const first = present(await transaction.read(task).findFirst({ where: { id: "a" } }), "task a");
    const rehearsed = await transaction.rehearse(async () => {
        await transaction.write({
            table: task,
            before: first,
            after: { id: "a", scope: "inbox", priority: 7 },
        });
        await transaction.write({
            table: task,
            before: { id: "d", scope: "inbox", priority: 4 },
            after: { id: "d", scope: "inbox", priority: 8 },
        });

        return (await transaction.read(task).findMany({ where: { scope: "inbox" } })).length;
    });
    await transaction.commit();

    // see the rehearsal's rows inside it, and commit only the writes before it
    expect({
        value: rehearsed.value,
        changes: rehearsed.changes.map((change) => [
            change.before?.["id"] ?? null,
            change.after?.["priority"] ?? null,
        ]),
        committed: (await tasks(database)).map((row) => [row.id, row.priority]),
    }).toEqual({
        value: 3,
        changes: [
            [null, 8],
            ["a", 7],
        ],
        committed: [
            ["a", 1],
            ["b", 2],
            ["c", 3],
            ["d", 4],
        ],
    });
});

test("refuse a write sharing a unique key that includes a JSON column, comparing its JSON values", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create([bookmark], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const transaction = await Transaction.open(storage.database);

    // bookmark a place, then the same place again and another
    const write = (id: string, place: { x: number; y: number }) =>
        transaction
            .write({
                table: bookmark,
                before: null,
                after: { id, scope: "board", person: "ada", place },
            })
            .then(() => "written");
    expect([
        await write("a", { x: 1, y: 2 }),
        await refusal(write("b", { x: 1, y: 2 })),
        await write("c", { x: 2, y: 1 }),
    ]).toEqual([
        "written",
        ["CONFLICT", "DUPLICATE", "a record with the same unique key already exists"],
        "written",
    ]);
});

test("refuse a write sharing a stored row's unique JSON key in another key order", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // store a bookmarked place as the database stores its JSON
    const storage = await TestDatabase.create([bookmark], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await storage.database.transaction((transaction) =>
        transaction
            .insert(bookmark)
            .values({
                id: "a",
                scope: "board",
                person: "ada",
                place: { x: 1, y: 2 },
            })
            .execute(),
    );

    // bookmark the same place with its keys in the other order
    const transaction = await Transaction.open(storage.database);
    const written = transaction.write({
        table: bookmark,
        before: null,
        after: { id: "b", scope: "board", person: "ada", place: { y: 2, x: 1 } },
    });

    expect(await refusal(written)).toEqual([
        "CONFLICT",
        "DUPLICATE",
        "a record with the same unique key already exists",
    ]);
});

test("refuse a unique key a commit after the transaction's base took as a lost race, which a run repeats to see the key taken", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create([label], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const { database } = storage;

    // start a transaction, then let another commit take the label's name
    const late = await Transaction.open(database);
    await Transaction.run(database, async (transaction) => {
        await transaction.write({
            table: label,
            before: null,
            after: { id: "a", scope: "board", folder: null, name: "urgent", archivedAt: null },
        });
    });

    // refuse the late transaction's name as a lost race, then plainly in a run after the commit
    const write = (transaction: Transaction) =>
        transaction.write({
            table: label,
            before: null,
            after: { id: "b", scope: "board", folder: null, name: "urgent", archivedAt: null },
        });
    expect([
        await refusal(write(late)),
        await refusal(Transaction.run(database, (transaction) => write(transaction))),
    ]).toEqual([
        [
            "SERVICE_UNAVAILABLE",
            "CONCURRENT_UPDATE",
            "another commit changed what the transaction read",
        ],
        ["CONFLICT", "DUPLICATE", "a record with the same unique key already exists"],
    ]);
});

/** Entries with slugs unique across them, optional details and a count. */
const entry = defineTable(
    "commit_entry",
    {
        /** The entry's key. */
        id: text("id").primaryKey(),
        /** The board the entry lives in. */
        scope: text("scope").notNull(),
        /** The slug, unique among entries. */
        slug: text("slug").notNull(),
        /** The details, absent for none. */
        details: json("details", schema.object({ note: schema.string() })),
        /** How often the entry was seen. */
        seen: integer("seen").notNull(),
    },
    { log: {}, constraints: (row) => [unique("commit_entry_slug").on(row.slug)] },
);

/** Build an entry's row on the board, with a note in its details when given. */
function entryRow(id: string, slug: string, seen: number, note?: string) {
    return { id, scope: "board", slug, details: note === undefined ? null : { note }, seen };
}

test("commit runs of inserts, plain updates and deletes in a few statements, shifting unique slugs in write order", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create([entry], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const { database } = storage;
    const commit = async (write: (changes: ChangeSet) => void) => {
        const changes = new ChangeSet();
        write(changes);
        const before = database.driver.state.statements;
        await changes.commit(
            database,
            await database.log.position(),
            new ReadSet(),
            async () => {},
        );

        return database.driver.state.statements - before;
    };

    // insert four entries, then update them, one clearing its details beside the count
    const written = [
        entryRow("a", "x", 0, "first"),
        entryRow("b", "y", 0),
        entryRow("c", "z", 0),
        entryRow("d", "q", 0),
    ];
    const inserting = await commit((changes) => {
        for (const after of written) {
            changes.write({ table: entry, before: null, after });
        }
    });
    const updating = await commit((changes) => {
        for (const before of written) {
            changes.write({
                table: entry,
                before,
                after: { ...before, seen: 1, details: null },
            });
        }
    });

    // shift the slugs along a chain in write order, then delete two entries
    const seen = (id: string, slug: string) => entryRow(id, slug, 1);
    await commit((changes) => {
        changes.write({ table: entry, before: seen("c", "z"), after: seen("c", "w") });
        changes.write({ table: entry, before: seen("b", "y"), after: seen("b", "z") });
        changes.write({ table: entry, before: seen("a", "x"), after: seen("a", "y") });
    });
    const deleting = await commit((changes) => {
        changes.write({ table: entry, before: seen("a", "y"), after: null });
        changes.write({ table: entry, before: seen("d", "q"), after: null });
    });

    // write the shifted slugs and real nulls, with a statement per group of rows
    const extra = TestDatabase.target() === "postgresql" ? 1 : 0;
    const rows = await database.select().from(entry).orderBy(asc(entry.id));
    const cleared = await database.all(
        sql`SELECT count(*) AS count FROM ${entry} WHERE ${entry.details} IS NULL`,
    );
    expect({
        rows: rows.map((each) => [each.id, each.slug, each.seen, each.details]),
        cleared: Number(cleared[0]?.["count"]),
        statements: { inserting, updating, deleting },
    }).toEqual({
        rows: [
            ["b", "z", 1, null],
            ["c", "w", 1, null],
        ],
        cleared: 2,
        statements: { inserting: 3 + extra, updating: 4 + extra, deleting: 3 + extra },
    });
});

test("read a committed row once per transaction, and see the transaction's write of it after", async () => {
    const { database } = await open();

    // read one task twice, write it, then read it again
    const transaction = await Transaction.open(database);
    const before = database.driver.state.statements;
    const first = present(await transaction.read(task).findFirst({ where: { id: "a" } }), "task a");
    const second = await transaction.read(task).findFirst({ where: { id: "a" } });
    const reads = database.driver.state.statements - before;
    const written = { id: "a", scope: "inbox", priority: 7 };
    await transaction.write({ table: task, before: first, after: written });
    const third = await transaction.read(task).findFirst({ where: { id: "a" } });

    // spend one statement on the first read alone, the write showing over it
    expect({ reads, first, second, third }).toEqual({
        reads: 1,
        first: { id: "a", scope: "inbox", priority: 1 },
        second: { id: "a", scope: "inbox", priority: 1 },
        third: written,
    });
});

test("read each writing transaction's identity as its log entries record it, a rolled back one leaving none behind", async () => {
    const { database } = await open();

    // write in two transactions with one rollback between, reading each identity
    const write = (id: string) =>
        database.transaction(async (writing) => {
            await writing.insert(task).values({ id, scope: "inbox", priority: 4 });

            return writing.log.transaction();
        });
    const first = await write("d");
    const refused = await database
        .transaction(async (writing) => {
            await writing.insert(task).values({ id: "e", scope: "inbox", priority: 5 });
            throw new TypeError("roll back");
        })
        .catch((error: unknown) => error);
    const second = await write("f");

    // record each identity on its entries, the two apart
    const entries = await database.all(
        sql`SELECT "transaction" AS id FROM ${sql.relation(LOG)} ORDER BY sequence DESC LIMIT 2`,
    );
    expect({
        refused,
        isApart: first !== second,
        entries: entries.map((row) => row["id"]).toReversed(),
    }).toEqual({
        refused: new TypeError("roll back"),
        isApart: true,
        entries: [first, second],
    });
});

test("stamp no identity for a transaction that only reads, and the one its entries record for a writer", async () => {
    const { database, base } = await open();
    const changes = async () =>
        (await database.all(sql`SELECT total_changes() AS changes`))[0]?.["changes"];

    // read in a transaction, with SQLite's change count around it and PostgreSQL's id in it
    const before = TestDatabase.target() === "sqlite" ? await changes() : undefined;
    const assigned = await database.transaction(async (transaction) => {
        await transaction.select().from(task);
        const [row] =
            TestDatabase.target() === "postgresql"
                ? await transaction.all(sql`SELECT pg_current_xact_id_if_assigned() AS id`)
                : [{ id: null }];

        return row?.["id"];
    });
    const after = TestDatabase.target() === "sqlite" ? await changes() : undefined;

    // write in another, reading its identity after its write
    const identity = await database.transaction(async (transaction) => {
        await transaction.insert(task).values({ id: "g", scope: "inbox", priority: 6 });

        return transaction.log.transaction();
    });
    const entries = await database.all(
        sql`SELECT "transaction" AS id FROM ${sql.relation(LOG)} WHERE sequence > ${base.sequence}`,
    );

    // change no row and assign no id for the reader, and give the writer's entry its identity
    expect({ assigned, isUnchanged: after === before, entries }).toEqual({
        assigned: null,
        isUnchanged: true,
        entries: [{ id: identity }],
    });
});

test("stamp a transaction again after a rolled back savepoint undid its first logged write", async () => {
    const { database, base } = await open();

    // write in a savepoint that rolls back, then write again in the same transaction
    const identity = await database.transaction(async (transaction) => {
        await transaction
            .transaction(async (savepoint) => {
                await savepoint.insert(task).values({ id: "h", scope: "inbox", priority: 7 });
                throw new TypeError("roll back");
            })
            .catch(() => undefined);
        await transaction.insert(task).values({ id: "i", scope: "inbox", priority: 8 });

        return transaction.log.transaction();
    });

    // log the second write alone, under the transaction's identity
    const entries = await database.all(
        sql`SELECT "transaction" AS id FROM ${sql.relation(LOG)} WHERE sequence > ${base.sequence}`,
    );
    expect({ isStamped: identity !== undefined, entries }).toEqual({
        isStamped: true,
        entries: [{ id: identity }],
    });
});

test("record each of many interleaved writers' entries under one identity while readers stamp nothing", async () => {
    // count SQLite's changes per statement, which only SQLite reports
    const { database } = await open({ dialect: "sqlite" });
    const changes = async () =>
        Number((await database.all(sql`SELECT total_changes() AS changes`))[0]?.["changes"]);

    // interleave writing transactions of two rows with reading ones
    const before = await changes();
    const base = (await database.log.position()).sequence;
    await Promise.all(
        Array.from({ length: STRESS_TRANSACTIONS }, async (_, index) => {
            await database.transaction(async (transaction) => {
                await transaction.select().from(task).where(eq(task.id, "a"));
            });
            await database.transaction(async (transaction) => {
                await transaction.insert(task).values([
                    { id: `stress-${index}-1`, scope: "stress", priority: index },
                    { id: `stress-${index}-2`, scope: "stress", priority: index },
                ]);
            });
        }),
    );
    const written = (await changes()) - before;

    // group the log's new entries by their transaction
    const entries = await database.all(
        sql`SELECT "transaction" AS id FROM ${sql.relation(LOG)} WHERE sequence > ${base}`,
    );
    const sizes = new Map<unknown, number>();
    for (const row of entries) {
        sizes.set(row["id"], (sizes.get(row["id"]) ?? 0) + 1);
    }

    // change six rows per writer and none per reader
    expect({ written, writers: sizes.size, sizes: new Set(sizes.values()) }).toEqual({
        written: STRESS_TRANSACTIONS * 6,
        writers: STRESS_TRANSACTIONS,
        sizes: new Set([2]),
    });
});

/** Units with one predefined row. */
const unit = defineTable(
    "unit",
    { id: text("id").primaryKey(), name: text("name").notNull() },
    { predefined: [{ id: "metre", name: "Metre" }] },
);

test("refuse writing a predefined row, and write other rows of its table", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create([unit], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const { database } = storage;

    // refuse changing the predefined row, and insert another
    const write = (after: { readonly id: string; readonly name: string }) =>
        Transaction.run(database, (transaction) =>
            transaction.write({ table: unit, before: null, after }),
        );
    const refused = await refusal(write({ id: "metre", name: "Meter" }));
    await write({ id: "foot", name: "Foot" });
    expect({
        refused,
        rows: await database.select().from(unit).orderBy(asc(unit.id)),
    }).toEqual({
        refused: [
            "BAD_REQUEST",
            "RECORD_INVALID",
            `cannot write predefined row ["${unit[TABLE].sqlName}","metre"] of unit`,
        ],
        rows: [
            { id: "foot", name: "Foot" },
            { id: "metre", name: "Metre" },
        ],
    });
});

test("share reads of a shared table among transactions until a commit writes it, each seeing its writes", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // store two members and a task
    const storage = await TestDatabase.create([member, task], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const { database } = storage;
    const ada = { id: "ada", scope: "s", role: "owner" };
    const bob = { id: "bob", scope: "s", role: "reader" };
    const carol = { id: "carol", scope: "s", role: "reader" };
    await database.transaction((transaction) =>
        transaction.insert(member).values([ada, bob]).execute(),
    );
    await database.transaction((transaction) =>
        transaction.insert(task).values({ id: "a", scope: "inbox", priority: 1 }).execute(),
    );
    const counted = async (read: () => Promise<readonly unknown[]>) => {
        const before = database.driver.state.statements;
        const rows = await read();

        return { rows: rows.length, statements: database.driver.state.statements - before };
    };
    const members = (transaction: Transaction) =>
        transaction
            .view()
            .read(member)
            .findMany({ where: { scope: "s" } });
    const inbox = () => Transaction.reading(database).view().read(task).findMany({});

    // read the members twice at one version, and the unshared tasks twice
    const first = await counted(() => members(Transaction.reading(database)));
    const second = await counted(() => members(Transaction.reading(database)));
    const unshared = [await counted(inbox), await counted(inbox)];

    // commit a member, seen by its transaction, then read the members anew
    const mine = await Transaction.run(database, async (transaction) => {
        await members(transaction);
        await transaction.write({ table: member, before: null, after: carol });

        return (await members(transaction)).map((row) => row.id).toSorted();
    });
    const third = await counted(() => members(Transaction.reading(database)));

    // read the version alone while the members stand
    expect({ first, second, unshared, mine, third }).toEqual({
        first: { rows: 2, statements: 2 },
        second: { rows: 2, statements: 1 },
        unshared: [
            { rows: 1, statements: 1 },
            { rows: 1, statements: 1 },
        ],
        mine: ["ada", "bob", "carol"],
        third: { rows: 3, statements: 2 },
    });
});

test("read a shared table's committed rows anew once compaction removed their log entries", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // store a compacted member and a role in history
    const storage = await TestDatabase.create([member, role], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const { database } = storage;
    const compact = () => database.log.compact(world.clock.now() + 1000, world.clock.now());
    const members = async () =>
        (await Transaction.reading(database).view().read(member).findMany({}))
            .map((row) => row.id)
            .toSorted();
    await database.transaction((transaction) =>
        transaction.insert(member).values({ id: "ada", scope: "s", role: "owner" }).execute(),
    );
    await compact();
    await database.transaction((transaction) =>
        transaction.insert(role).values({ id: "owner", scope: "s" }).execute(),
    );
    const before = await members();

    // add a member and compact its entry away
    await database.transaction((transaction) =>
        transaction.insert(member).values({ id: "bob", scope: "s", role: "reader" }).execute(),
    );
    await compact();

    // see the added member past the compaction
    expect({ before, after: await members() }).toEqual({
        before: ["ada"],
        after: ["ada", "bob"],
    });
});

test("read a shared table's predefined rows anew once a migration changes them", async () => {
    // share a read of the predefined unit, migrating at one frozen millisecond
    const before = unitTable("Metre");
    const environment = testSimulation().environment("store");
    const storage = await TestDatabase.create([before], { environment });
    onTestFinished(() => storage.close());
    const { database } = storage;
    await database.migrate([before]);
    const names = async () =>
        (await Transaction.reading(database).view().read(before).findMany({})).map(
            (row) => row.name,
        );
    const shared = await names();

    // rename the predefined unit through two migrations in one millisecond, reading after each
    const applied = async () =>
        (await database.all(sql`SELECT applied_at FROM ${sql.relation(STATE)}`)).map((row) =>
            Number(row["applied_at"]),
        );
    const versions = [await applied()];
    await database.migrate([unitTable("Meter")]);
    const renamed = await names();
    versions.push(await applied());
    await database.migrate([unitTable("Metre")]);
    versions.push(await applied());

    // see each rename past the cached read, as every apply moves the table's version
    expect({
        shared,
        renamed,
        restored: await names(),
        moves: versions
            .slice(1)
            .map((version, index) => nth(version, 0) - nth(nth(versions, index), 0)),
    }).toEqual({ shared: ["Metre"], renamed: ["Meter"], restored: ["Metre"], moves: [1, 1] });
});

test("see every member committed before a shared read starts, while writers and readers interleave", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // commit members one at a time into four scopes, counting each scope's commits finished
    const storage = await TestDatabase.create([member], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const { database } = storage;
    const finished = [0, 0, 0, 0];
    const scopeOf = (index: number) => index % finished.length;
    const write = async (index: number) => {
        const scope = scopeOf(index);
        await database.transaction((transaction) =>
            transaction
                .insert(member)
                .values({
                    id: `m${String(index)}`,
                    scope: `s${String(scope)}`,
                    role: "reader",
                })
                .execute(),
        );
        finished[scope] = nth(finished, scope) + 1;
    };

    // read the scope each commit writes beside the commit, with members to share
    const read = async (scope: number) => {
        const before = nth(finished, scope);
        const rows = await Transaction.reading(database)
            .view()
            .read(member)
            .findMany({ where: { scope: `s${String(scope)}` } });

        return rows.length >= before;
    };
    const reads: boolean[] = [];
    for (let index = 0; index < STRESS_MEMBERS; index++) {
        const [isCurrent] = await Promise.all([read(scopeOf(index)), write(index)]);
        reads.push(isCurrent);
    }

    // see at least every finished commit in every read, and every member at the end
    expect({
        stale: reads.filter((isCurrent) => !isCurrent).length,
        members: (await database.select().from(member)).length,
    }).toEqual({ stale: 0, members: STRESS_MEMBERS });
});

test("read the shared tables' version again after it failed, through the same transaction", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // fail a shared read before the tables and the log exist
    const storage = await TestDatabase.create([member], { environment: world });
    onTestFinished(() => storage.close());
    const { database } = storage;
    const transaction = Transaction.reading(database);
    const members = () => transaction.view().read(member).findMany({});
    const failed = await members().then(
        () => "read",
        () => "failed",
    );

    // read through the same transaction once migrated
    await database.migrate([member]);
    await database.transaction(
        async (writing) =>
            await writing.insert(member).values({ id: "ada", scope: "s", role: "owner" }),
    );
    expect({ failed, read: (await members()).map((row) => row.id) }).toEqual({
        failed: "failed",
        read: ["ada"],
    });
});

test("share a read across commits writing other shared tables, reading anew once its table changes", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // store a member and a role, both shared
    const storage = await TestDatabase.create([member, role], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const { database } = storage;
    await database.transaction((transaction) =>
        transaction.insert(member).values({ id: "ada", scope: "s", role: "owner" }).execute(),
    );
    const counted = async () => {
        const before = database.driver.state.statements;
        await Transaction.reading(database).view().read(member).findMany({});

        return database.driver.state.statements - before;
    };
    const first = await counted();

    // write a role, then a member
    await database.transaction((transaction) =>
        transaction.insert(role).values({ id: "owner", scope: "s" }).execute(),
    );
    const afterRole = await counted();
    await database.transaction((transaction) =>
        transaction.insert(member).values({ id: "bob", scope: "s", role: "reader" }).execute(),
    );
    const afterMember = await counted();

    // read the members anew after the member alone
    expect({ first, afterRole, afterMember }).toEqual({
        first: 2,
        afterRole: 1,
        afterMember: 2,
    });
});
