import { schema } from "@destack/schema";
import { sql } from "../../sql/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { TEST_DIALECTS, TestDatabase } from "../../test/database.ts";
import { Change, defineTable, eq, TABLE, text } from "../../index.ts";
import { changeTables, lease, note, reading, revision } from "./fixture.ts";

/** Open a migrated test database. */
async function open(dialect: (typeof TEST_DIALECTS)[number], kind?: "memory" | "file") {
    const storage = await TestDatabase.create(
        dialect,
        changeTables,
        kind === undefined ? {} : { storage: kind },
    );
    onTestFinished(() => storage.close());
    await storage.database.migrate(changeTables);

    return storage;
}

/** A note with exact, structured and binary values. */
const first = {
    id: "a",
    title: "First",
    scope: "inbox",
    summary: null,
    views: 9_007_199_254_740_993n,
    labels: ["draft"],
    editedAt: 1790244000123,
    attachment: new Uint8Array([1, 2, 3]),
};

test.for(TEST_DIALECTS)("log committed changes of %s tables in commit order", async (dialect) => {
    const { database } = await open(dialect);
    const tables = [note, revision, lease];
    expect((await database.log.position()).sequence).toBe(0);

    // record an insert with exact and binary values
    await database.transaction(async (transaction) => {
        await transaction.insert(note).values(first);
        await transaction
            .insert(revision)
            .values({ scope: "inbox", noteId: "a", number: 1, title: "First" });
        await transaction.insert(lease).values({ name: "a", expiresAt: 1 });
    });
    const created = await database.log.read({ tables, after: 0 });
    expect(
        created.changes.map(
            ({
                table,
                sequence: _sequence,
                transaction: _transaction,
                changedAt: _changedAt,
                ...change
            }) => ({
                ...change,
                table: table === note,
            }),
        ),
    ).toEqual([
        {
            table: true,
            key: { id: "a" },
            operation: "insert",
            after: first,
            scope: "inbox",
        },
        {
            table: false,
            key: { noteId: "a", number: 1 },
            operation: "insert",
            after: { scope: "inbox", noteId: "a", number: 1, title: "First" },
            scope: "inbox",
        },
    ]);

    // share one transaction across both changes
    expect(new Set(created.changes.map((change) => change.transaction)).size).toBe(1);

    // skip empty updates and record binary-only ones with the bytes before and after
    await database.update(note).set({ title: "First" }).where(eq(note.id, "a"));
    await database
        .update(note)
        .set({ attachment: new Uint8Array([4]) })
        .where(eq(note.id, "a"));
    await database.update(note).set({ title: "Renamed" }).where(eq(note.id, "a"));
    const updated = await database.log.read({ tables: [note], after: created.sequence });
    expect(
        updated.changes.map((change) => [
            change.operation,
            Change.after(change)?.title,
            Change.before(change)?.attachment,
            Change.after(change)?.attachment,
        ]),
    ).toEqual([
        ["update", "First", new Uint8Array([1, 2, 3]), new Uint8Array([4])],
        ["update", "Renamed", new Uint8Array([4]), new Uint8Array([4])],
    ]);

    // record a key change as a deletion and an insertion, then a deletion
    await database.update(note).set({ id: "b" }).where(eq(note.id, "a"));
    await database.delete(note).where(eq(note.id, "b"));
    const moved = await database.log.read({ tables: [note], after: updated.sequence });
    expect(
        moved.changes.map((change) => [change.operation, change.key, Change.image(change).title]),
    ).toEqual([
        ["delete", { id: "a" }, "Renamed"],
        ["insert", { id: "b" }, "Renamed"],
        ["delete", { id: "b" }, "Renamed"],
    ]);

    // advance a filtered cursor past other tables' changes
    await database
        .insert(revision)
        .values({ scope: "inbox", noteId: "b", number: 2, title: "Second" });
    const filtered = await database.log.read({ tables: [note], after: moved.sequence });
    expect(filtered.changes).toEqual([]);
    expect(filtered.sequence).toBe((await database.log.position()).sequence);

    // compact windowed changes, keep history, and fail stale readers
    await database.log.compact(Date.now() + 1);
    await expect(database.log.read({ tables, after: 0 })).rejects.toMatchObject({
        code: "CHANGES_COMPACTED",
    });
    const history = await database.execute(
        sql`SELECT "table" FROM ${sql.identifier("__destack_log")} ORDER BY sequence`,
        schema.object({ table: schema.string() }),
    );
    expect(history.map((entry) => entry.table)).toEqual([
        "destack__db__revision",
        "destack__db__revision",
    ]);

    // follow from the latest sequence
    const controller = new AbortController();
    const following = database.log.follow(
        { tables: [note], after: (await database.log.position()).sequence },
        controller.signal,
    );
    const next = following.next();
    await database.insert(note).values({ ...first, id: "c" });
    const received = await next;
    expect(
        received.done === true ? [] : received.value.changes.map((change) => change.key),
    ).toEqual([{ id: "c" }]);

    // advance past a commit of another table without its changes
    const advancing = following.next();
    await database
        .insert(revision)
        .values({ scope: "inbox", noteId: "c", number: 1, title: "First" });
    expect(await advancing).toEqual({
        done: false,
        value: { changes: [], sequence: (await database.log.position()).sequence },
    });
    controller.abort();
    expect((await following.next()).done).toBe(true);
});

test.for(TEST_DIALECTS)(
    "read the changes the open transaction wrote before its commit on %s",
    async (dialect) => {
        const { database } = await open(dialect);
        await database.insert(note).values(first);

        // read this transaction's own changes in order
        const written = await database.transaction(async (transaction) => {
            await transaction.update(note).set({ title: "Renamed" }).where(eq(note.id, "a"));
            await transaction
                .insert(revision)
                .values({ scope: "inbox", noteId: "a", number: 1, title: "First" });
            await transaction.delete(note).where(eq(note.id, "a"));

            return await transaction.log.written([note]);
        });
        expect(
            written.map((change) => [change.operation, change.key, Change.after(change)?.title]),
        ).toEqual([
            ["update", { id: "a" }, "Renamed"],
            ["delete", { id: "a" }, undefined],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "log each replicated change under the origin whose writes it replicates, and read the latest position another origin's follower reaches, on %s",
    async (dialect) => {
        // write a note here, then rename it as a replica of another database's writes
        const { database } = await open(dialect);
        await database.insert(note).values(first);
        const own = await database.log.position();
        await database.transaction((transaction) =>
            transaction.log.asReplica(async () => {
                await transaction.update(note).set({ title: "Copied" }).where(eq(note.id, "a"));
            }, "other"),
        );

        // tell the replicated change by its origin, which the other database's follower already has
        const { changes } = await database.log.read({ tables: [note], after: 0 });
        expect([
            changes.map((change) => change.origin),
            await database.log.position("other"),
            (await database.log.position()).sequence > own.sequence,
        ]).toEqual([[undefined, "other"], own, true]);
    },
);

test.for(TEST_DIALECTS)(
    "keep the latest sequence once compaction removes every change on %s",
    async (dialect) => {
        const { database } = await open(dialect);
        await database.insert(note).values(first);
        await database.update(note).set({ title: "Renamed" }).where(eq(note.id, "a"));
        const latest = (await database.log.position()).sequence;

        // compact every windowed change
        await database.log.compact(Date.now() + 1);
        expect([
            (await database.log.position()).sequence,
            (await database.log.position()).sequence,
            (await database.log.read({ tables: [note], after: latest })).sequence,
        ]).toEqual([latest, latest, latest]);
    },
);

test.for(TEST_DIALECTS)(
    "read one scope's changes with the rows before and after them on %s",
    async (dialect) => {
        const { database } = await open(dialect);
        await database.insert(note).values(first);
        await database.insert(note).values({ ...first, id: "b", scope: "archive" });
        const start = await database.log.read({ tables: [note], after: 0, scopes: ["inbox"] });
        expect(start.changes.map((change) => change.key)).toEqual([{ id: "a" }]);

        // restore the row before each update, nulls included
        await database
            .update(note)
            .set({ title: "Titled", summary: "Short" })
            .where(eq(note.id, "a"));
        await database.update(note).set({ summary: null }).where(eq(note.id, "a"));
        const updated = await database.log.read({
            tables: [note],
            after: start.sequence,
            scopes: ["inbox"],
        });
        expect(
            updated.changes.map((change) => [
                change.operation,
                [Change.before(change)?.title, Change.before(change)?.summary],
                [Change.after(change)?.title, Change.after(change)?.summary],
            ]),
        ).toEqual([
            ["update", ["First", null], ["Titled", "Short"]],
            ["update", ["Titled", "Short"], ["Titled", null]],
        ]);

        // log a scope change as a deletion and an insertion
        await database.update(note).set({ scope: "archive" }).where(eq(note.id, "a"));
        const moved = await database.log.read({ tables: [note], after: updated.sequence });
        expect(moved.changes.map((change) => [change.operation, change.scope])).toEqual([
            ["delete", "inbox"],
            ["insert", "archive"],
        ]);
        const archived = await database.log.read({
            tables: [note],
            after: 0,
            scopes: ["archive"],
        });
        expect(archived.changes.map((change) => [change.operation, change.key])).toEqual([
            ["insert", { id: "b" }],
            ["insert", { id: "a" }],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "wait for a position through this connection's and another connection's commits on %s",
    async (dialect) => {
        const storage = await open(dialect, "file");
        const { database } = storage;
        const other = await storage.connect(changeTables);
        onTestFinished(() => other.close());
        await database.insert(note).values(first);
        const position = (await database.log.position()).sequence;

        // return at once for a position the log has
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

        // give up once the signal aborts
        expect(await database.log.wait(position + 10, AbortSignal.timeout(20))).toBe(false);
    },
);

test.for(TEST_DIALECTS)("end each page with a whole transaction on %s", async (dialect) => {
    const { database } = await open(dialect);

    // write in a transaction, alone and in one statement, then read pages of two
    await database.transaction(async (transaction) => {
        for (const id of ["a", "b", "c"]) {
            await transaction.insert(note).values({ ...first, id });
        }
    });
    await database.insert(note).values({ ...first, id: "d" });
    await database.insert(note).values(["e", "f", "g"].map((id) => ({ ...first, id })));
    const page = await database.log.read({ tables: [note], after: 0, limit: 2 });
    const next = await database.log.read({ tables: [note], after: page.sequence, limit: 2 });

    // complete the statement's changes as one transaction
    expect([page, next].map(({ changes }) => changes.map((change) => change.key.id))).toEqual([
        ["a", "b", "c"],
        ["d", "e", "f", "g"],
    ]);
});

test.skipIf(!TEST_DIALECTS.includes("postgresql"))(
    "order postgresql changes by commit, not by write",
    async () => {
        const { database } = await open("postgresql");

        // write first in a transaction that commits last
        const released = Promise.withResolvers<void>();
        const written = Promise.withResolvers<void>();
        const late = database.transaction(async (transaction) => {
            await transaction.insert(note).values({ ...first, id: "late" });
            written.resolve();
            await released.promise;
        });
        await written.promise;
        await database.insert(note).values({ ...first, id: "early" });

        // hide the uncommitted write, then order it after the earlier commit
        const before = await database.log.read({ tables: [note], after: 0 });
        expect(before.changes.map((change) => change.key)).toEqual([{ id: "early" }]);
        released.resolve();
        await late;
        const after = await database.log.read({ tables: [note], after: before.sequence });
        expect(after.changes.map((change) => change.key)).toEqual([{ id: "late" }]);
    },
);

test.skipIf(!TEST_DIALECTS.includes("postgresql"))(
    "run a postgresql update that lost to a concurrent commit again after it, keeping its write last",
    async () => {
        const { database } = await open("postgresql");
        await database.insert(note).values(first);

        // commit both of two concurrent updates, the one losing the race again after the other
        const released = Promise.withResolvers<void>();
        const read = Promise.withResolvers<void>();
        const late = database.transaction(async (transaction) => {
            await transaction.select().from(note).where(eq(note.id, "a"));
            read.resolve();
            await released.promise;
            await transaction.update(note).set({ title: "Late" }).where(eq(note.id, "a"));
        });
        await read.promise;
        await database.update(note).set({ title: "Early" }).where(eq(note.id, "a"));
        released.resolve();
        await late;
        expect(
            await database.select({ title: note.title }).from(note).where(eq(note.id, "a")),
        ).toEqual([{ title: "Late" }]);
    },
);

test.for(TEST_DIALECTS)(
    "keep the changes after a consumer's slot until it expires or is dropped, and read its position on %s",
    async (dialect) => {
        const { database } = await open(dialect);
        await database.insert(note).values(first);
        const kept = (await database.log.position()).sequence;
        await database.update(note).set({ title: "Renamed" }).where(eq(note.id, "a"));

        // keep the changes after the slot while it lasts
        const now = Date.now();
        await database.log.advance("subscription", kept, now + 60_000);
        expect([await database.log.slot("subscription"), await database.log.slot("other")]).toEqual(
            [kept, undefined],
        );
        await database.log.compact(now + 1, now);
        expect(
            (await database.log.read({ tables: [note], after: kept })).changes.map(
                (change) => change.operation,
            ),
        ).toEqual(["update"]);

        // compact past an expired and a dropped slot
        await database.log.compact(now + 1, now + 120_000);
        await expect(database.log.read({ tables: [note], after: kept })).rejects.toMatchObject({
            code: "CHANGES_COMPACTED",
        });
        await database.log.drop("subscription");
        expect(
            await database.execute(sql`SELECT name FROM ${sql.identifier("__destack_log_slot")}`),
        ).toEqual([]);
    },
);

test.for(TEST_DIALECTS)(
    "bound a transaction's changes by the positions around it on %s",
    async (dialect) => {
        const { database } = await open(dialect);

        // write two changes in one transaction
        await database.insert(note).values(first);
        const before = (await database.log.position()).sequence;
        await database.transaction(async (transaction) => {
            await transaction.update(note).set({ summary: "u" }).where(eq(note.id, "a"));
            await transaction.update(note).set({ title: "v" }).where(eq(note.id, "a"));
        });

        // read the transaction's positions and times
        const bounds = await database.log.bounds(before + 1);
        expect([bounds.before, bounds.after, bounds.startedAt <= bounds.committedAt]).toEqual([
            before,
            before + 2,
            true,
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "log the rows of a table without a scope column under its database's scope on %s",
    async (dialect) => {
        const setting = defineTable(
            "setting",
            { name: text("name").primaryKey(), value: text("value").notNull() },
            { log: {} },
        );

        // refuse writing the rows while the database has no scope
        const unscoped = await TestDatabase.create(dialect, [setting]);
        onTestFinished(() => unscoped.close());
        await unscoped.database.migrate([setting]);
        await expect(
            unscoped.database.insert(setting).values({ name: "theme", value: "dark" }),
        ).rejects.toMatchObject({
            cause: { message: "the database has no scope for the rows of destack__db__setting" },
        });

        // file every change under the scope the database was created with
        const scoped = await TestDatabase.create(dialect, [setting]);
        onTestFinished(() => scoped.close());
        await scoped.database.log.create("space-a");
        await scoped.database.migrate([setting]);
        await scoped.database.insert(setting).values({ name: "theme", value: "dark" });
        await scoped.database
            .update(setting)
            .set({ value: "light" })
            .where(eq(setting.name, "theme"));
        await scoped.database.delete(setting).where(eq(setting.name, "theme"));
        const changes = await scoped.database.log.read({ tables: [setting], after: 0 });
        expect(changes.changes.map((change) => [change.operation, change.scope])).toEqual([
            ["insert", "space-a"],
            ["update", "space-a"],
            ["delete", "space-a"],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "log the insertions of an append-only %s table alone, refusing its updates",
    async (dialect) => {
        const { database } = await open(dialect);

        // insert two readings, delete the first and try to change the second
        await database.insert(reading).values([
            { id: "a", scope: "inbox", value: 1 },
            { id: "b", scope: "inbox", value: 2 },
        ]);
        await database.delete(reading).where(eq(reading.id, "a"));
        const updated = await database
            .update(reading)
            .set({ value: 3 })
            .where(eq(reading.id, "b"))
            .then(
                () => "updated",
                (error: unknown) => String(error instanceof Error ? (error.cause ?? error) : error),
            );

        // the log keeps both insertions and no deletion, and the second reading is unchanged
        const logged = await database.log.read({ tables: [reading], after: 0 });
        const kept = await database.select({ id: reading.id, value: reading.value }).from(reading);
        expect([
            logged.changes.map((change) => [change.operation, change.key]),
            updated.includes(`append-only table: ${reading[TABLE].sqlName}`),
            kept,
        ]).toEqual([
            [
                ["insert", { id: "a" }],
                ["insert", { id: "b" }],
            ],
            true,
            [{ id: "b", value: 2 }],
        ]);
    },
);
