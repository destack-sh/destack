import { schema } from "@destack/schema";
import { sql } from "../../sql/index.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { TEST_DIALECTS, TestDatabase } from "../../test/database.ts";
import { Change, eq, LogPosition, TABLE, Transaction } from "../../index.ts";
import { changeTables, lease, note, reading, revision } from "./fixture.ts";
import { setting } from "../../bun/test/setting.ts";

/** The concurrent writers of the PostgreSQL numbering test: four per pool, filling both pools. */
const WRITERS = 8;

/** The commits each writer makes. */
const COMMITS = 10;

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
    const now = Date.now();
    await database.log.compact(now + 1, now);
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

    // yield another table's commit to an advancing follow alone
    const advancing = database.log.follow(
        { tables: [note], after: (await database.log.position()).sequence },
        controller.signal,
        { isAdvancing: true },
    );
    const [skipped, advanced] = [following.next(), advancing.next()];
    await database
        .insert(revision)
        .values({ scope: "inbox", noteId: "c", number: 1, title: "First" });
    const other = (await database.log.position()).sequence;
    const keepalive = await advanced;

    // yield the next note to the follow that skipped the other table's commit
    await database.insert(note).values({ ...first, id: "d" });
    const noted = await skipped;
    expect(
        [keepalive, noted].map((page) =>
            page.done === true
                ? undefined
                : [page.value.changes.map((change) => change.key), page.value.sequence],
        ),
    ).toEqual([
        [[], other],
        [[{ id: "d" }], (await database.log.position()).sequence],
    ]);
    controller.abort();
    expect([(await following.next()).done, (await advancing.next()).done]).toEqual([true, true]);
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
    "keep the log's identifier across a restore's new epoch, and refuse comparing positions of two logs, on %s",
    async (dialect) => {
        // renew the epoch as a restore does, beside another database's log
        const { database } = await open(dialect);
        const { database: other } = await open(dialect);
        const before = await database.log.position();
        const renewed = await database.log.renew();
        const after = await database.log.position();
        const foreign = await other.log.position();

        // keep the identifier, which is the first epoch, and order the epochs of one log
        expect([after.log, after.epoch, LogPosition.isAfter(after, before)]).toEqual([
            before.epoch,
            renewed,
            true,
        ]);
        expect(() => LogPosition.isAfter(after, foreign)).toThrow(
            new TypeError(`positions of logs ${after.log} and ${foreign.log} do not compare`),
        );
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
        const now = Date.now();
        await database.log.compact(now + 1, now);
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
    "keep both of two postgresql transactions that read a row together and append to it, running the later again",
    async () => {
        const { database } = await open("postgresql");
        await database.insert(note).values(first);

        // read the row in both before either commits, and append to the title read
        const both = Promise.withResolvers<void>();
        let reads = 0;
        const append = (suffix: string) =>
            Transaction.run(database, async (transaction) => {
                const [row] = await transaction.read(note).findMany({ where: { id: "a" } });
                if (row === undefined) {
                    throw new TypeError("the note is missing");
                }
                reads += 1;
                if (reads === 2) {
                    both.resolve();
                }
                await both.promise;
                await transaction.write({
                    table: note,
                    before: row,
                    after: { ...row, title: row.title + suffix },
                });
            });
        await Promise.all([append("!"), append("?")]);

        // keep both appends, the later one read again after the first, in one of the two orders
        const [row] = await database
            .select({ title: note.title })
            .from(note)
            .where(eq(note.id, "a"));
        expect([[{ title: "First!?" }], [{ title: "First?!" }]]).toContainEqual([row]);
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

test.skipIf(!TEST_DIALECTS.includes("postgresql"))(
    "number concurrent postgresql commits of two pools without gaps, each key's changes in commit order, at once for their writer",
    async () => {
        const storage = await open("postgresql");
        const other = await storage.connect(changeTables);
        await storage.database.insert(note).values({ ...first, editedAt: 0 });

        // follow every change from the start through the other pool
        const total = 1 + WRITERS * COMMITS * 2;
        const followed: Change[] = [];
        const following = (async () => {
            const selection = { tables: [note, revision], after: 0 };
            for await (const page of other.log.follow(selection, AbortSignal.timeout(10_000))) {
                followed.push(...page.changes);
                if (followed.length >= total) {
                    return;
                }
            }
        })();

        // commit from both pools at once, each commit counting the shared note up and adding a revision
        await Promise.all(
            Array.from({ length: WRITERS }, async (_, writer) => {
                const database = writer % 2 === 0 ? storage.database : other;
                for (let number = 0; number < COMMITS; number++) {
                    await database.transaction(
                        async (transaction) => {
                            await transaction
                                .update(note)
                                .set({ editedAt: sql`${note.editedAt} + 1` })
                                .where(eq(note.id, "a"));
                            await transaction.insert(revision).values({
                                scope: "inbox",
                                noteId: `writer-${writer}`,
                                number,
                                title: "Counted",
                            });
                        },
                        { isolationLevel: "read committed" },
                    );
                }
            }),
        );

        // read the last commit at once through its writer's pool
        expect((await storage.database.log.position()).sequence).toBe(total);

        // number every change once, without gaps
        await following;
        expect(followed.map((change) => change.sequence)).toEqual(
            Array.from({ length: total }, (_, index) => index + 1),
        );

        // place each later update of the shared note at a higher position, with its higher count
        const counts = followed.flatMap((change) =>
            Change.of(change, note) ? [Change.after(change)?.editedAt] : [],
        );
        expect(counts).toEqual(Array.from({ length: WRITERS * COMMITS + 1 }, (_, index) => index));
    },
);

test.for(TEST_DIALECTS)(
    "move a consumer's slot only from the position it read, refusing a consumer whose read another one moved past, on %s",
    async (dialect) => {
        const { database } = await open(dialect);
        const expiresAt = Date.now() + 60_000;

        // place a slot from the start, then move it from where it stands
        await database.log.move("rating", 0, 3, expiresAt);
        await database.log.move("rating", 3, 5, expiresAt);

        // refuse a second consumer that read the start, and one that read the slot before the last move
        const refused = await Promise.all(
            [0, 3].map((from) => refusal(database.log.move("rating", from, 4, expiresAt))),
        );
        expect({ refused, slot: await database.log.slot("rating") }).toEqual({
            refused: [
                ["CONCURRENT_UPDATE", "slot rating moved past 0"],
                ["CONCURRENT_UPDATE", "slot rating moved past 3"],
            ],
            slot: 5,
        });
    },
);

test.for(TEST_DIALECTS)(
    "place a consumer's slot at the log's start once, and list the slots of a prefix on %s",
    async (dialect) => {
        const { database } = await open(dialect);
        const expiresAt = Date.now() + 60_000;

        // place three slots, placing a moved one again
        await database.log.place("usage:a", expiresAt);
        await database.log.move("usage:a", 0, 3, expiresAt);
        await database.log.place("usage:a", expiresAt);
        await database.log.place("usage:b", expiresAt);
        await database.log.place("rating", expiresAt);

        // list the prefix's slots, the moved one where it moved and the new one at the start
        expect({
            slots: await database.log.slots("usage:"),
            positions: [await database.log.slot("usage:a"), await database.log.slot("usage:b")],
        }).toEqual({ slots: ["usage:a", "usage:b"], positions: [3, 0] });
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
            await database.all(sql`SELECT name FROM ${sql.identifier("__destack_log_slot")}`),
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

test("migrate after a transaction that created the log rolled back, as no log remains", async () => {
    // create the log and the table in a transaction that rolls back
    const storage = await TestDatabase.create("sqlite", [setting]);
    onTestFinished(() => storage.close());
    const { database } = storage;
    const rolledBack = await database
        .transaction(async (transaction) => {
            await transaction.log.create("space-a");
            await transaction.migrate([setting]);
            throw new Error("roll back");
        })
        .catch((error: unknown) => (error instanceof Error ? error.message : "unknown"));

    // migrate the table again without a log
    await database.migrate([setting]);
    expect({ rolledBack, tables: await database.unapplied([setting]) }).toEqual({
        rolledBack: "roll back",
        tables: [],
    });
});

test("log a transaction's writes once it creates the log after stamping without one", async () => {
    // stamp a transaction before the log exists, create the log, then write a logged row
    const storage = await TestDatabase.create("sqlite", [setting]);
    onTestFinished(() => storage.close());
    const { database } = storage;
    await database.transaction(async (transaction) => {
        await transaction.log.create("space-a");
        await transaction.migrate([setting]);
        await transaction.insert(setting).values({ name: "theme", value: "dark" });
    });

    // log the write under its transaction
    const changes = await database.log.read({ tables: [setting], after: 0 });
    expect(changes.changes.map((change) => [change.operation, change.scope])).toEqual([
        ["insert", "space-a"],
    ]);
});

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

test.for(TEST_DIALECTS)(
    "read the position of a scope's latest change to some tables, past later changes to others, on %s",
    async (dialect) => {
        // write a note and a revision in the inbox, then a note in the archive
        const { database } = await open(dialect);
        await database.insert(note).values({ ...first, id: "n" });
        const noted = await database.log.position();
        await database
            .insert(revision)
            .values({ scope: "inbox", noteId: "n", number: 1, title: "R" });
        const revised = await database.log.position();
        await database.insert(note).values({ ...first, id: "m", scope: "archive" });
        const archived = await database.log.position();
        const changed = {
            notes: await database.log.changed([note], ["inbox"]),
            both: await database.log.changed([note, revision], ["inbox"]),
            archive: await database.log.changed([note], ["archive"]),
            none: await database.log.changed([], ["inbox"]),
            own: await database.transaction(async (transaction) => {
                // leave out the transaction's own uncommitted write
                await transaction.insert(note).values({ ...first, id: "o" });

                return await transaction.log.changed([note], ["inbox"]);
            }),
        };

        // place the inbox's notes at the horizon once compaction removed the notes' changes
        const owned = await database.log.position();
        await database.log.compact(Date.now() + 1000, Date.now());
        expect({ ...changed, compacted: await database.log.changed([note], ["inbox"]) }).toEqual({
            notes: noted,
            both: revised,
            archive: archived,
            none: { ...archived, sequence: 0 },
            own: noted,
            compacted: owned,
        });
    },
);
