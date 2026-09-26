import { expect, onTestFinished, test } from "@destack/test";
import { sql } from "drizzle-orm";
import { migrate } from "../../migration/database.ts";
import { TEST_DIALECTS, TestDatabase } from "../../test/database.ts";
import { eq } from "../../index.ts";
import { changeTables, lease, note, revision } from "./fixture.ts";

/** Open a migrated test database of the change example schema. */
async function open(dialect: (typeof TEST_DIALECTS)[number], storage?: "memory" | "file") {
    const test = await TestDatabase.create(dialect, changeTables, { storage });
    onTestFinished(() => test.close());
    await migrate(test.database, changeTables);

    return test;
}

/** A note with exact, structured and binary values. */
const first = {
    id: "a",
    title: "First",
    folder: "inbox",
    summary: null,
    views: 9_007_199_254_740_993n,
    labels: ["draft"],
    editedAt: new Date("2026-09-24T10:00:00.123Z"),
    attachment: new Uint8Array([1, 2, 3]),
};

test.for(TEST_DIALECTS)("log committed changes of %s tables in commit order", async (dialect) => {
    const { database } = await open(dialect);
    const tables = [note, revision, lease];
    expect(await database.log.latest()).toBe(0);

    // record an insert with its transaction, exact values and no binary column
    await database.transaction(async (transaction) => {
        await transaction.insert(note).values(first);
        await transaction.insert(revision).values({ noteId: "a", number: 1, title: "First" });
        await transaction.insert(lease).values({ name: "a", expiresAt: 1 });
    });
    const created = await database.log.read({ tables, after: 0 });
    const { attachment: _attachment, ...logged } = first;
    expect(
        created.changes.map(({ table, ...change }) => ({ ...change, table: table === note })),
    ).toEqual([
        {
            sequence: expect.any(Number),
            transaction: expect.any(String),
            table: true,
            key: { id: "a" },
            operation: "insert",
            row: logged,
            route: "inbox",
            changedAt: expect.any(Number),
        },
        {
            sequence: expect.any(Number),
            transaction: expect.any(String),
            table: false,
            key: { noteId: "a", number: 1 },
            operation: "insert",
            row: { noteId: "a", number: 1, title: "First" },
            route: null,
            changedAt: expect.any(Number),
        },
    ]);
    expect(created.changes[0]!.transaction).toBe(created.changes[1]!.transaction);

    // skip updates that change nothing and record ones that change a binary column only
    await database.update(note).set({ title: "First" }).where(eq(note.id, "a"));
    await database
        .update(note)
        .set({ attachment: new Uint8Array([4]) })
        .where(eq(note.id, "a"));
    await database.update(note).set({ title: "Renamed" }).where(eq(note.id, "a"));
    const updated = await database.log.read({ tables: [note], after: created.sequence });
    expect(updated.changes.map((change) => [change.operation, change.row.title])).toEqual([
        ["update", "First"],
        ["update", "Renamed"],
    ]);

    // record a key change as a deletion and an insertion, then a deletion
    await database.update(note).set({ id: "b" }).where(eq(note.id, "a"));
    await database.delete(note).where(eq(note.id, "b"));
    const moved = await database.log.read({ tables: [note], after: updated.sequence });
    expect(moved.changes.map((change) => [change.operation, change.key, change.row.title])).toEqual(
        [
            ["delete", { id: "a" }, "Renamed"],
            ["insert", { id: "b" }, "Renamed"],
            ["delete", { id: "b" }, "Renamed"],
        ],
    );

    // advance a filtered cursor past other tables' changes
    await database.insert(revision).values({ noteId: "b", number: 2, title: "Second" });
    const filtered = await database.log.read({ tables: [note], after: moved.sequence });
    expect(filtered.changes).toEqual([]);
    expect(filtered.sequence).toBe(await database.log.latest());

    // compact windowed changes, keep history, and require stale readers to list again
    await database.log.compact(Date.now() + 1);
    await expect(database.log.read({ tables, after: 0 })).rejects.toMatchObject({
        code: "CHANGES_COMPACTED",
    });
    const history = await database.execute<{ table: string }>(
        sql`SELECT "table" FROM ${sql.identifier("__destack_log")} ORDER BY sequence`,
    );
    expect(history.map((entry) => entry.table)).toEqual([
        "destack__db__revision",
        "destack__db__revision",
    ]);

    // follow from the latest sequence and receive the next change
    const controller = new AbortController();
    const following = database.log.follow(
        { tables: [note], after: await database.log.latest() },
        controller.signal,
    );
    const next = following.next();
    await database.insert(note).values({ ...first, id: "c" });
    const received = await next;
    expect(received.done ? [] : received.value.changes.map((change) => change.key)).toEqual([
        { id: "c" },
    ]);
    controller.abort();
    expect((await following.next()).done).toBe(true);
});

test.for(TEST_DIALECTS)("read one route's changes with prior values on %s", async (dialect) => {
    const { database } = await open(dialect);
    await database.insert(note).values(first);
    await database.insert(note).values({ ...first, id: "b", folder: "archive" });
    const start = await database.log.read({ tables: [note], after: 0, routes: ["inbox"] });
    expect(start.changes.map((change) => change.key)).toEqual([{ id: "a" }]);

    // carry the prior values of changed columns, nulls included
    await database.update(note).set({ title: "Titled", summary: "Short" }).where(eq(note.id, "a"));
    await database.update(note).set({ summary: null }).where(eq(note.id, "a"));
    const updated = await database.log.read({
        tables: [note],
        after: start.sequence,
        routes: ["inbox"],
    });
    expect(updated.changes.map((change) => [change.operation, change.previous])).toEqual([
        ["update", { title: "First", summary: null }],
        ["update", { summary: "Short" }],
    ]);

    // log a route change as a deletion from the old route and an insertion into the new one
    await database.update(note).set({ folder: "archive" }).where(eq(note.id, "a"));
    const moved = await database.log.read({ tables: [note], after: updated.sequence });
    expect(moved.changes.map((change) => [change.operation, change.route])).toEqual([
        ["delete", "inbox"],
        ["insert", "archive"],
    ]);
    const archived = await database.log.read({
        tables: [note],
        after: 0,
        routes: ["archive"],
    });
    expect(archived.changes.map((change) => [change.operation, change.key])).toEqual([
        ["insert", { id: "b" }],
        ["insert", { id: "a" }],
    ]);
});

test.for(TEST_DIALECTS)(
    "wait for a position through this connection's and another connection's commits on %s",
    async (dialect) => {
        const test = await open(dialect, "file");
        const { database } = test;
        const other = await test.connect(changeTables);
        onTestFinished(() => other.close());
        await database.insert(note).values(first);
        const position = await database.log.latest();

        // return at once for a position the log holds
        const signal = AbortSignal.timeout(5000);
        expect(await database.log.wait(position, signal)).toBe(true);

        // wake on this connection's next commit
        const own = database.log.wait(position + 1, signal);
        await database.insert(note).values({ ...first, id: "b" });
        expect(await own).toBe(true);

        // wake on another connection's commit
        const foreign = database.log.wait(position + 2, signal);
        await other.insert(note).values({ ...first, id: "c" });
        expect(await foreign).toBe(true);

        // give up once the signal aborts before the log reaches the position
        expect(await database.log.wait(position + 10, AbortSignal.timeout(20))).toBe(false);
    },
);

test.for(TEST_DIALECTS)("end each page with a whole transaction on %s", async (dialect) => {
    const { database } = await open(dialect);

    // write three notes in one transaction and read pages of two
    await database.transaction(async (transaction) => {
        for (const id of ["a", "b", "c"]) {
            await transaction.insert(note).values({ ...first, id });
        }
    });
    await database.insert(note).values({ ...first, id: "d" });
    const page = await database.log.read({ tables: [note], after: 0, limit: 2 });
    const next = await database.log.read({ tables: [note], after: page.sequence, limit: 2 });

    expect([page, next].map(({ changes }) => changes.map((change) => change.key.id))).toEqual([
        ["a", "b", "c"],
        ["d"],
    ]);
});

test.skipIf(!TEST_DIALECTS.includes("postgresql"))(
    "order postgresql changes by commit, not by write",
    async () => {
        const { database } = await open("postgresql");

        // write first in a transaction that commits last
        let release!: () => void;
        const held = new Promise<void>((resolve) => (release = resolve));
        let written!: () => void;
        const isWritten = new Promise<void>((resolve) => (written = resolve));
        const late = database.transaction(async (transaction) => {
            await transaction.insert(note).values({ ...first, id: "late" });
            written();
            await held;
        });
        await isWritten;
        await database.insert(note).values({ ...first, id: "early" });

        // hide the uncommitted write, then order it after the earlier commit
        const before = await database.log.read({ tables: [note], after: 0 });
        expect(before.changes.map((change) => change.key)).toEqual([{ id: "early" }]);
        release();
        await late;
        const after = await database.log.read({ tables: [note], after: before.sequence });
        expect(after.changes.map((change) => change.key)).toEqual([{ id: "late" }]);
    },
);

test.skipIf(!TEST_DIALECTS.includes("postgresql"))(
    "report a postgresql update that lost to a concurrent commit as a concurrent update",
    async () => {
        const { database } = await open("postgresql");
        await database.insert(note).values(first);

        // read the row in two transactions and commit one update before the other writes
        let release!: () => void;
        const held = new Promise<void>((resolve) => (release = resolve));
        let read!: () => void;
        const isRead = new Promise<void>((resolve) => (read = resolve));
        const late = database.transaction(async (transaction) => {
            await transaction.select().from(note).where(eq(note.id, "a"));
            read();
            await held;
            await transaction.update(note).set({ title: "Late" }).where(eq(note.id, "a"));
        });
        await isRead;
        await database.update(note).set({ title: "Early" }).where(eq(note.id, "a"));
        release();
        await expect(late).rejects.toMatchObject({ code: "CONCURRENT_UPDATE" });
    },
);
