import { schema } from "@destack/schema";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { asc, eq } from "../sql/index.ts";
import { TEST_DIALECTS, TestDatabase } from "../test/database.ts";
import { defineTable } from "../table/table.ts";
import { integer, json, text } from "../table/column.ts";
import { unique, uniqueIndex } from "../table/constraint.ts";
import { ChangeSet, Transaction, ReadSet } from "./commit.ts";

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

        // commit, keeping the changed rows alone
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
