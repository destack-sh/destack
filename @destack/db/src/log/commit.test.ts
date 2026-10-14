import { schema } from "@destack/schema";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { asc, eq, sql } from "../sql/index.ts";
import { TEST_DIALECTS, TestDatabase } from "../test/database.ts";
import { defineTable, TABLE } from "../table/table.ts";
import { integer, json, text } from "../table/column.ts";
import { unique, uniqueIndex } from "../table/constraint.ts";
import { ChangeSet, Transaction, ReadSet } from "./commit.ts";
import { LOG } from "./schema.ts";

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
        /** The secret, kept out of the log. */
        secret: text("secret").notNull().sensitive(),
    },
    { log: {} },
);

/** Pins a person keeps, unique by the person and the place they pin. */
const pin = defineTable(
    "commit_pin",
    {
        /** The pin's key. */
        id: text("id").primaryKey(),
        /** The board the pin lives in. */
        scope: text("scope").notNull(),
        /** The person pinning. */
        person: text("person").notNull(),
        /** The pinned place. */
        place: json("place", schema.object({ x: schema.number(), y: schema.number() })).notNull(),
    },
    { log: {}, constraints: (entry) => [unique("commit_pin_place").on(entry.person, entry.place)] },
);

/** Open a database with three tasks, two in the inbox and one archived. */
async function open(dialect: (typeof TEST_DIALECTS)[number]) {
    const storage = await TestDatabase.create(dialect, [task], { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await database.insert(task).values([
        { id: "a", scope: "inbox", priority: 1 },
        { id: "b", scope: "inbox", priority: 2 },
        { id: "c", scope: "archive", priority: 3 },
    ]);

    return { database, base: await database.log.position() };
}

/** Read every task in key order. */
async function tasks(database: Awaited<ReturnType<typeof open>>["database"]) {
    return database.select().from(task).orderBy(asc(task.id));
}

test.each(TEST_DIALECTS)(
    "commit a transaction's changes, seeing its own writes through its overlay, keeping each row's first and last image and leaving rows it put back unchanged on %s",
    async (dialect) => {
        const { database, base } = await open(dialect);

        // read and write through the transaction's view
        const reads = new ReadSet();
        const changes = new ChangeSet();
        const view = database.log.at(base).recording(reads);
        const first = await view.row(task, { id: "a" });
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
        const second = await view.row(task, { id: "b" });
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
        const seen = await view.layer(changes.overlay()).rows(task, { scope: "inbox" });

        // commit, keeping only the changed rows
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
    },
);

test.each(TEST_DIALECTS)(
    "refuse a transaction another commit raced on a row it read or a row its scan selects, and commit one whose reads no commit touched on %s",
    async (dialect) => {
        const { database, base } = await open(dialect);

        // read one row by key, scan the inbox, and read the archive apart
        const byKey = new ReadSet();
        await database.log.at(base).recording(byKey).row(task, { id: "a" });
        const byScan = new ReadSet();
        await database.log.at(base).recording(byScan).rows(task, { scope: "inbox" });
        const apart = new ReadSet();
        await database.log.at(base).recording(apart).rows(task, { scope: "archive" });

        // change a task and add one to the inbox after the base, as other commits do
        await database.update(task).set({ priority: 7 }).where(eq(task.id, "a"));
        await database.insert(task).values({ id: "e", scope: "inbox", priority: 1 });
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
            ["CONCURRENT_UPDATE", "another commit changed what the transaction read"],
            ["CONCURRENT_UPDATE", "another commit changed what the transaction read"],
            "committed",
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "read a transaction's rows with its own writes in order up to a limit, count them, and commit its records beside its changes on %s",
    async (dialect) => {
        const { database } = await open(dialect);

        // write through the transaction and read the inbox with its writes
        const transaction = await Transaction.open(database);
        const first = await transaction.row(task, { id: "a" });
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
        const inbox = await transaction.rows(
            task,
            { scope: "inbox" },
            { orderBy: { priority: "desc" }, limit: 2 },
        );
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
    },
);

test("keep only logged columns in a transaction's images and its view, committed or written", async () => {
    const storage = await TestDatabase.create("sqlite", [token], { isMigrated: true });
    onTestFinished(() => storage.close());
    await storage.database.insert(token).values({ id: "a", scope: "inbox", secret: "first" });

    // write a second token, then read both as images and through the view
    const transaction = await Transaction.open(storage.database);
    await transaction.write({
        table: token,
        before: null,
        after: { id: "b", scope: "inbox", secret: "second" },
    });
    const seen = {
        image: await transaction.image(token, { id: "a" }),
        images: await transaction.images(token, { scope: "inbox" }, { orderBy: { id: "asc" } }),
        view: await transaction.view().rows(token, { scope: "inbox" }),
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

test("drop a failed step's writes and records as a savepoint does, keeping the writes before it", async () => {
    const { database } = await open("sqlite");

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

test.each(TEST_DIALECTS)(
    "refuse a write sharing a unique key with a committed row, the transaction's own or its batch, nulls being equal and archived rows outside the key, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, [label], { isMigrated: true });
        onTestFinished(() => storage.close());
        const database = storage.database;
        await database.insert(label).values([
            { id: "a", scope: "board", folder: null, name: "red", archivedAt: null },
            { id: "b", scope: "board", folder: null, name: "blue", archivedAt: 1 },
        ]);

        // write labels clashing with a committed one, an own write and a key, and ones outside the key
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
            await refusal(
                transaction.write(row("g", "pink"), row("h", "pink")).then(() => "written"),
            ),
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

        // keep the writes outside every key
        expect({
            written,
            batches,
            committed: (await database.select().from(label).orderBy(asc(label.id))).map((each) => [
                each.id,
                each.name,
            ]),
        }).toEqual({
            batches: [["DUPLICATE", "a record with the same unique key exists"], "written"],
            written: [
                ["DUPLICATE", "a record with the same unique key exists"],
                "written",
                ["DUPLICATE", "a record with the same unique key exists"],
                ["DUPLICATE", "a record with the same unique key exists"],
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
    },
);

test("rehearse writes over a transaction's rows, answering the rows they change, and drop them after", async () => {
    const { database } = await open("sqlite");

    // write once, then rehearse a change of that row and of another
    const transaction = await Transaction.open(database);
    await transaction.write({
        table: task,
        before: null,
        after: { id: "d", scope: "inbox", priority: 4 },
    });
    const first = await transaction.row(task, { id: "a" });
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

        return (await transaction.rows(task, { scope: "inbox" })).length;
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
    const storage = await TestDatabase.create("sqlite", [pin], { isMigrated: true });
    onTestFinished(() => storage.close());
    const transaction = await Transaction.open(storage.database);

    // pin a place, then the same place again and another
    const write = (id: string, place: { x: number; y: number }) =>
        transaction
            .write({
                table: pin,
                before: null,
                after: { id, scope: "board", person: "ada", place },
            })
            .then(() => "written");
    expect([
        await write("a", { x: 1, y: 2 }),
        await refusal(write("b", { x: 1, y: 2 })),
        await write("c", { x: 2, y: 1 }),
    ]).toEqual(["written", ["DUPLICATE", "a record with the same unique key exists"], "written"]);
});

test.each(TEST_DIALECTS)(
    "refuse a write sharing a stored row's unique JSON key in another key order, on %s",
    async (dialect) => {
        // store a pinned place as the database keeps its JSON
        const storage = await TestDatabase.create(dialect, [pin], { isMigrated: true });
        onTestFinished(() => storage.close());
        await storage.database.insert(pin).values({
            id: "a",
            scope: "board",
            person: "ada",
            place: { x: 1, y: 2 },
        });

        // pin the same place with its keys in the other order
        const transaction = await Transaction.open(storage.database);
        const written = transaction.write({
            table: pin,
            before: null,
            after: { id: "b", scope: "board", person: "ada", place: { y: 2, x: 1 } },
        });

        expect(await refusal(written)).toEqual([
            "DUPLICATE",
            "a record with the same unique key exists",
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "refuse a unique key a commit after the transaction's base took as a lost race, which a run repeats to see the key taken, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, [label], { isMigrated: true });
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

        // refuse the late transaction's same name as a lost race, and refuse it plainly once a run starts after the commit
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
            ["CONCURRENT_UPDATE", "another commit changed what the transaction read"],
            ["DUPLICATE", "a record with the same unique key exists"],
        ]);
    },
);

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

test.each(TEST_DIALECTS)(
    "commit runs of inserts, plain updates and deletes in a few statements on %s, shifting unique slugs in write order",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, [entry], { isMigrated: true });
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

        // keep the shifted slugs, real nulls for cleared details, and a statement per group of rows
        const extra = dialect === "postgresql" ? 1 : 0;
        const rows = await database.select().from(entry).orderBy(asc(entry.id));
        const cleared = await database.execute(
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
    },
);

test.each(TEST_DIALECTS)(
    "read a committed row once per transaction, and see the transaction's own write of it after on %s",
    async (dialect) => {
        const { database } = await open(dialect);

        // read one task twice, write it, then read it again
        const transaction = await Transaction.open(database);
        const before = database.driver.state.statements;
        const first = await transaction.row(task, { id: "a" });
        const second = await transaction.row(task, { id: "a" });
        const reads = database.driver.state.statements - before;
        const written = { id: "a", scope: "inbox", priority: 7 };
        await transaction.write({ table: task, before: first, after: written });
        const third = await transaction.row(task, { id: "a" });

        // spend one statement on the first read alone, the write showing over it
        expect({ reads, first, second, third }).toEqual({
            reads: 1,
            first: { id: "a", scope: "inbox", priority: 1 },
            second: { id: "a", scope: "inbox", priority: 1 },
            third: written,
        });
    },
);

test.each(TEST_DIALECTS)(
    "read each writing transaction's own identity as its log entries keep it, a rolled back one leaving none behind, on %s",
    async (dialect) => {
        const { database } = await open(dialect);

        // write in a transaction, roll one back, and write in another, reading each identity
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

        // keep each identity on its own entry, the two apart
        const entries = await database.execute(
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
    },
);

/** Units with one predefined row. */
const unit = defineTable(
    "unit",
    { id: text("id").primaryKey(), name: text("name").notNull() },
    { predefined: [{ id: "metre", name: "Metre" }] },
);

test.each(TEST_DIALECTS)(
    "refuse writing a predefined row, and write other rows of its table, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, [unit], { isMigrated: true });
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
                "INVALID_RECORD",
                `cannot write predefined row ["${unit[TABLE].sqlName}","metre"] of unit`,
            ],
            rows: [
                { id: "foot", name: "Foot" },
                { id: "metre", name: "Metre" },
            ],
        });
    },
);
