import { Simulation } from "@destack/environment/simulation";
import type { Environment } from "@destack/environment";
import { schema } from "@destack/schema";
import { sql } from "../../sql/index.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { TestDatabase } from "../../test/database.ts";
import { Change, DatabaseError, eq, LogPosition, TABLE } from "../../index.ts";
import { changeTables, first, lease, note, reading, revision } from "./fixture.ts";
import { setting } from "../../bun/test/setting.ts";
import { testSimulation } from "@destack/test/world";

/** Open a migrated test database in a world, its own by default, with the world's clock. */
async function open(
    kind?: "memory" | "file",
    environment: Environment = testSimulation().environment("test"),
) {
    const storage = await TestDatabase.create(
        changeTables,
        kind === undefined ? { environment } : { storage: kind, environment },
    );
    onTestFinished(() => storage.close());
    await storage.database.migrate(changeTables);

    return { storage, database: storage.database, clock: environment.clock };
}

test("log committed changes of tables in commit order", async () => {
    const { database, clock } = await open();
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
    await database.transaction((transaction) =>
        transaction.update(note).set({ title: "First" }).where(eq(note.id, "a")).execute(),
    );
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(note)
                .set({ attachment: new Uint8Array([4]) })
                .where(eq(note.id, "a")),
    );
    await database.transaction((transaction) =>
        transaction.update(note).set({ title: "Renamed" }).where(eq(note.id, "a")).execute(),
    );
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
    await database.transaction((transaction) =>
        transaction.update(note).set({ id: "b" }).where(eq(note.id, "a")).execute(),
    );
    await database.transaction((transaction) =>
        transaction.delete(note).where(eq(note.id, "b")).execute(),
    );
    const moved = await database.log.read({ tables: [note], after: updated.sequence });
    expect(
        moved.changes.map((change) => [change.operation, change.key, Change.image(change).title]),
    ).toEqual([
        ["delete", { id: "a" }, "Renamed"],
        ["insert", { id: "b" }, "Renamed"],
        ["delete", { id: "b" }, "Renamed"],
    ]);

    // advance a filtered cursor past other tables' changes
    await database.transaction(
        async (transaction) =>
            await transaction
                .insert(revision)
                .values({ scope: "inbox", noteId: "b", number: 2, title: "Second" }),
    );
    const filtered = await database.log.read({ tables: [note], after: moved.sequence });
    expect(filtered.changes).toEqual([]);
    expect(filtered.sequence).toBe((await database.log.position()).sequence);

    // compact windowed changes, retain history, and fail stale readers
    const now = clock.now();
    await database.log.compact(now + 1, now);
    expect(await refusal(database.log.read({ tables, after: 0 }))).toEqual([
        "GONE",
        "CHANGES_COMPACTED",
        "changes after 0 were compacted: list the tables again",
    ]);
    const history = await database.execute(
        sql`SELECT "table" FROM ${sql.relation("__destack_log")} ORDER BY sequence`,
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
    await database.transaction((transaction) =>
        transaction
            .insert(note)
            .values({ ...first, id: "c" })
            .execute(),
    );
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
    await database.transaction(
        async (transaction) =>
            await transaction
                .insert(revision)
                .values({ scope: "inbox", noteId: "c", number: 1, title: "First" }),
    );
    const other = (await database.log.position()).sequence;
    const keepalive = await advanced;

    // yield the next note to the follow that skipped the other table's commit
    await database.transaction((transaction) =>
        transaction
            .insert(note)
            .values({ ...first, id: "d" })
            .execute(),
    );
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

test("read the changes the open transaction wrote before its commit, after a log id", async () => {
    const { database } = await open();
    await database.transaction((transaction) => transaction.insert(note).values(first).execute());

    // read the transaction's changes in write order, then those after the first read
    const reads = await database.transaction(async (transaction) => {
        await transaction.update(note).set({ title: "Renamed" }).where(eq(note.id, "a"));
        const renamed = await transaction.log.readTransaction({ tables: [note] });
        await transaction
            .insert(revision)
            .values({ scope: "inbox", noteId: "a", number: 1, title: "First" });
        await transaction.delete(note).where(eq(note.id, "a"));
        const deleted = await transaction.log.readTransaction({
            tables: [note],
            after: renamed.after,
        });

        return [renamed, deleted];
    });
    expect(
        reads.map((read) =>
            read.changes.map((change) => [
                change.operation,
                change.key,
                Change.after(change)?.["title"],
            ]),
        ),
    ).toEqual([[["update", { id: "a" }, "Renamed"]], [["delete", { id: "a" }, undefined]]]);
});

test("preserve the log's identifier across a restore's new epoch, and refuse comparing positions of two logs", async () => {
    // renew the epoch as a restore does, beside another database's log
    const world = testSimulation().environment("test");
    const { database } = await open(undefined, world);
    const { database: other } = await open(undefined, world);
    const before = await database.log.position();
    const renewed = await database.log.renew();
    const after = await database.log.position();
    const foreign = await other.log.position();

    // preserve the identifier as the first epoch, and order the epochs of one log
    expect([after.log, after.epoch, LogPosition.isAfter(after, before)]).toEqual([
        before.epoch,
        renewed,
        true,
    ]);
    expect(() => LogPosition.isAfter(after, foreign)).toThrow(
        new TypeError(`cannot compare positions of logs ${after.log} and ${foreign.log}`),
    );
});

test("preserve the latest sequence once compaction removes every change", async () => {
    const { database, clock } = await open();
    await database.transaction((transaction) => transaction.insert(note).values(first).execute());
    await database.transaction((transaction) =>
        transaction.update(note).set({ title: "Renamed" }).where(eq(note.id, "a")).execute(),
    );
    const latest = (await database.log.position()).sequence;

    // compact every windowed change
    const now = clock.now();
    await database.log.compact(now + 1, now);
    expect([
        (await database.log.position()).sequence,
        (await database.log.position()).sequence,
        (await database.log.read({ tables: [note], after: latest })).sequence,
    ]).toEqual([latest, latest, latest]);
});

test("read one scope's changes with the rows before and after them", async () => {
    const { database } = await open();
    await database.transaction((transaction) => transaction.insert(note).values(first).execute());
    await database.transaction((transaction) =>
        transaction
            .insert(note)
            .values({ ...first, id: "b", scope: "archive" })
            .execute(),
    );
    const start = await database.log.read({ tables: [note], after: 0, scopes: ["inbox"] });
    expect(start.changes.map((change) => change.key)).toEqual([{ id: "a" }]);

    // restore the row before each update, nulls included
    await database.transaction(
        async (transaction) =>
            await transaction
                .update(note)
                .set({ title: "Titled", subtitle: "Short" })
                .where(eq(note.id, "a")),
    );
    await database.transaction((transaction) =>
        transaction.update(note).set({ subtitle: null }).where(eq(note.id, "a")).execute(),
    );
    const updated = await database.log.read({
        tables: [note],
        after: start.sequence,
        scopes: ["inbox"],
    });
    expect(
        updated.changes.map((change) => [
            change.operation,
            [Change.before(change)?.title, Change.before(change)?.subtitle],
            [Change.after(change)?.title, Change.after(change)?.subtitle],
        ]),
    ).toEqual([
        ["update", ["First", null], ["Titled", "Short"]],
        ["update", ["Titled", "Short"], ["Titled", null]],
    ]);

    // log a scope change as a deletion and an insertion
    await database.transaction((transaction) =>
        transaction.update(note).set({ scope: "archive" }).where(eq(note.id, "a")).execute(),
    );
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
});

test("wait for a position through this connection's and another connection's commits", async () => {
    const { storage, database } = await open("file");
    const other = await storage.connect(changeTables);
    onTestFinished(() => other.close());
    await database.transaction((transaction) => transaction.insert(note).values(first).execute());
    const position = (await database.log.position()).sequence;

    // return at once for a position the log has
    const signal = AbortSignal.timeout(5000);
    expect(await database.log.wait(position, signal)).toBe(true);

    // wake on this connection's next commit
    const mine = database.log.wait(position + 1, signal);
    await database.transaction((transaction) =>
        transaction
            .insert(note)
            .values({ ...first, id: "b" })
            .execute(),
    );
    expect(await mine).toBe(true);

    // wake on another connection's commit
    const foreign = database.log.wait(position + 2, signal);
    await other.transaction((transaction) =>
        transaction
            .insert(note)
            .values({ ...first, id: "c" })
            .execute(),
    );
    expect(await foreign).toBe(true);

    // give up once the signal aborts
    expect(await database.log.wait(position + 10, AbortSignal.timeout(20))).toBe(false);
});

test("end each page with a whole transaction", async () => {
    const { database } = await open();

    // write in a transaction, alone and in one statement, then read pages of two
    await database.transaction(async (transaction) => {
        for (const id of ["a", "b", "c"]) {
            await transaction.insert(note).values({ ...first, id });
        }
    });
    await database.transaction((transaction) =>
        transaction
            .insert(note)
            .values({ ...first, id: "d" })
            .execute(),
    );
    await database.transaction((transaction) =>
        transaction
            .insert(note)
            .values(["e", "f", "g"].map((id) => ({ ...first, id })))
            .execute(),
    );
    const page = await database.log.read({ tables: [note], after: 0, limit: 2 });
    const next = await database.log.read({ tables: [note], after: page.sequence, limit: 2 });

    // complete the statement's changes as one transaction
    expect([page, next].map(({ changes }) => changes.map((change) => change.key.id))).toEqual([
        ["a", "b", "c"],
        ["d", "e", "f", "g"],
    ]);
});

test("move a consumer's slot only from the position it read, refusing a consumer whose read another one moved past", async () => {
    const { database, clock } = await open();
    const expiresAt = clock.now() + 60_000;

    // place a slot from the start, then move it from where it stands
    await database.log.move("rating", 0, 3, expiresAt);
    await database.log.move("rating", 3, 5, expiresAt);

    // refuse a second consumer that read the start, and one that read the slot before the last move
    const refused = await Promise.all(
        [0, 3].map((from) => refusal(database.log.move("rating", from, 4, expiresAt))),
    );
    expect({ refused, slot: await database.log.slot("rating") }).toEqual({
        refused: [
            ["SERVICE_UNAVAILABLE", "CONCURRENT_UPDATE", "slot rating moved past 0"],
            ["SERVICE_UNAVAILABLE", "CONCURRENT_UPDATE", "slot rating moved past 3"],
        ],
        slot: 5,
    });
});

test("claim a consumer's page once from where its slot stands at the first claim's time, claiming nothing behind a move, placing a slot claimed from the start, and end the claim on a move, an advance or a drop", async () => {
    const { database, clock } = await open();
    const expiresAt = clock.now() + 60_000;

    // claim the page after 3 through 5 at 100, then claim again through 7 at 200
    await database.log.move("rating", 0, 3, expiresAt);
    const claimed = await database.log.claim("rating", 3, 5, 100, expiresAt);
    const kept = await database.log.claim("rating", 3, 7, 200, expiresAt);

    // move past the claimed page, then refuse a claim from the old place
    await database.log.move("rating", 3, 5, expiresAt);
    const moved = await database.log.claimed("rating");
    const behind = await database.log.claim("rating", 3, 9, 300, expiresAt);

    // claim again from the new place, advance to end it, then claim a new slot from the start
    const next = await database.log.claim("rating", 5, 9, 400, expiresAt);
    await database.log.advance("rating", 5, expiresAt);
    const advanced = await database.log.claimed("rating");
    const fromStart = await database.log.claim("usage:a", 0, 4, 500, expiresAt);
    await database.log.drop("rating");
    expect({
        claimed,
        kept,
        moved,
        behind,
        next,
        advanced,
        fromStart: [fromStart, await database.log.slot("usage:a")],
        dropped: [await database.log.slot("rating"), await database.log.claimed("rating")],
    }).toEqual({
        claimed: { through: 5, claimedAt: 100 },
        kept: { through: 5, claimedAt: 100 },
        moved: undefined,
        behind: undefined,
        next: { through: 9, claimedAt: 400 },
        advanced: undefined,
        fromStart: [{ through: 4, claimedAt: 500 }, 0],
        dropped: [undefined, undefined],
    });
});

test("stamp each logged change at its connection's clock as the clock moves between transactions", async () => {
    // write a note at one time, and rename it later
    const simulation = new Simulation({ seed: 1, start: Date.UTC(2026, 0, 1) });
    const { clock } = simulation;
    const environment = simulation.environment("store");
    const storage = await TestDatabase.create(changeTables, { environment });
    onTestFinished(() => storage.close());
    const { database } = storage;
    await database.migrate(changeTables);
    await database.transaction((transaction) => transaction.insert(note).values(first).execute());
    clock.set(Date.UTC(2026, 0, 2));
    await database.transaction(async (transaction) => {
        await transaction.update(note).set({ title: "Renamed" }).where(eq(note.id, "a"));
    });

    // read both changes at the clock's times
    const { changes } = await database.log.read({ tables: [note], after: 0 });
    expect(changes.map((change) => [change.operation, change.changedAt])).toEqual([
        ["insert", Date.UTC(2026, 0, 1)],
        ["update", Date.UTC(2026, 0, 2)],
    ]);
});

test("keep a transaction's stamp through a failed savepoint after its first write, logging one time for both writes", async () => {
    // write a note, fail a nested transaction, then rename the note, the clock moving between
    const simulation = new Simulation({ seed: 1, start: Date.UTC(2026, 0, 1) });
    const { clock } = simulation;
    const environment = simulation.environment("store");
    const storage = await TestDatabase.create(changeTables, { environment });
    onTestFinished(() => storage.close());
    const { database } = storage;
    await database.migrate(changeTables);
    await database.transaction(async (transaction) => {
        await transaction.insert(note).values(first);
        clock.set(Date.UTC(2026, 0, 2));
        await transaction
            .transaction(async (nested) => {
                await nested.update(note).set({ title: "Undone" }).where(eq(note.id, "a"));
                throw new Error("undo");
            })
            .catch(() => undefined);
        await transaction.update(note).set({ title: "Renamed" }).where(eq(note.id, "a"));
    });

    // log both kept writes at the transaction's first stamp
    const { changes } = await database.log.read({ tables: [note], after: 0 });
    expect(changes.map((change) => [change.operation, change.changedAt])).toEqual([
        ["insert", Date.UTC(2026, 0, 1)],
        ["update", Date.UTC(2026, 0, 1)],
    ]);
});

test("refuse a logged script outside a transaction, and log one inside a transaction", async () => {
    // run a script writing a note outside a transaction, then inside one
    const { database } = await open();
    const insert = `INSERT INTO ${note[TABLE].sqlName} (id, title, scope, views, labels, edited_at) VALUES ('b', 'Script', 'inbox', 1, '[]', 1)`;
    const outside = await refusal(() => database.executeScript(insert));
    await database.transaction((transaction) =>
        transaction.executeScript(insert.replace("'b'", "'c'")),
    );

    // log the transaction's write alone
    const { changes } = await database.log.read({ tables: [note], after: 0 });
    expect({ outside, logged: changes.map((change) => change.operation) }).toEqual({
        outside: [
            "INTERNAL_SERVER_ERROR",
            "TRANSACTION_REQUIRED",
            `a logged write of ${note[TABLE].sqlName} runs outside a transaction: run it in a transaction`,
        ],
        logged: ["insert"],
    });
});

test("place a consumer's slot at the log's start once, and list the slots of a prefix", async () => {
    const { database, clock } = await open();
    const expiresAt = clock.now() + 60_000;

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
});

test("retain the changes after a consumer's slot until it expires or is dropped, and read its position", async () => {
    const { database, clock } = await open();
    await database.transaction((transaction) => transaction.insert(note).values(first).execute());
    const kept = (await database.log.position()).sequence;
    await database.transaction((transaction) =>
        transaction.update(note).set({ title: "Renamed" }).where(eq(note.id, "a")).execute(),
    );

    // retain the changes after the slot while it lasts
    const now = clock.now();
    await database.log.advance("subscription", kept, now + 60_000);
    expect([await database.log.slot("subscription"), await database.log.slot("other")]).toEqual([
        kept,
        undefined,
    ]);
    await database.log.compact(now + 1, now);
    expect(
        (await database.log.read({ tables: [note], after: kept })).changes.map(
            (change) => change.operation,
        ),
    ).toEqual(["update"]);

    // compact past an expired and a dropped slot
    await database.log.compact(now + 1, now + 120_000);
    expect(await refusal(database.log.read({ tables: [note], after: kept }))).toEqual([
        "GONE",
        "CHANGES_COMPACTED",
        `changes after ${kept} were compacted: list the tables again`,
    ]);
    await database.log.drop("subscription");
    expect(await database.all(sql`SELECT name FROM ${sql.relation("__destack_log_slot")}`)).toEqual(
        [],
    );
});

test("bound a transaction's changes by the positions around it", async () => {
    const { database } = await open();

    // write two changes in one transaction
    await database.transaction((transaction) => transaction.insert(note).values(first).execute());
    const before = (await database.log.position()).sequence;
    await database.transaction(async (transaction) => {
        await transaction.update(note).set({ subtitle: "u" }).where(eq(note.id, "a"));
        await transaction.update(note).set({ title: "v" }).where(eq(note.id, "a"));
    });

    // read the transaction's positions and times
    const bounds = await database.log.bounds(before + 1);
    expect([bounds.before, bounds.after, bounds.startedAt <= bounds.committedAt]).toEqual([
        before,
        before + 2,
        true,
    ]);
});

test("log the rows of a table without a scope column under its database's scope", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // refuse writing the rows while the database has no scope
    const unscoped = await TestDatabase.create([setting], { environment: world });
    onTestFinished(() => unscoped.close());
    await unscoped.database.migrate([setting]);
    const unscopedWrite = await unscoped.database
        .transaction((transaction) =>
            transaction.insert(setting).values({ name: "theme", value: "dark" }).execute(),
        )
        .then(
            () => undefined,
            (error: unknown) => error,
        );
    expect(
        DatabaseError.is(unscopedWrite) && unscopedWrite.cause instanceof Error
            ? [unscopedWrite.reason, unscopedWrite.cause.message]
            : unscopedWrite,
    ).toEqual(["QUERY_FAILED", "the database has no scope for the rows of destack__db__setting"]);

    // file every change under the scope the database was created with
    const scoped = await TestDatabase.create([setting], { environment: world });
    onTestFinished(() => scoped.close());
    await scoped.database.log.create("space-a");
    await scoped.database.migrate([setting]);
    await scoped.database.transaction((transaction) =>
        transaction.insert(setting).values({ name: "theme", value: "dark" }).execute(),
    );
    await scoped.database.transaction(
        async (transaction) =>
            await transaction
                .update(setting)
                .set({ value: "light" })
                .where(eq(setting.name, "theme")),
    );
    await scoped.database.transaction((transaction) =>
        transaction.delete(setting).where(eq(setting.name, "theme")).execute(),
    );
    const changes = await scoped.database.log.read({ tables: [setting], after: 0 });
    expect(changes.changes.map((change) => [change.operation, change.scope])).toEqual([
        ["insert", "space-a"],
        ["update", "space-a"],
        ["delete", "space-a"],
    ]);
});

test("migrate after a transaction that created the log rolled back, as no log remains", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // create the log and the table in a transaction that rolls back
    const storage = await TestDatabase.create([setting], { environment: world });
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
    // run in a world of its own
    const world = testSimulation().environment("test");

    // stamp a transaction before the log exists, create the log, then write a logged row
    const storage = await TestDatabase.create([setting], { environment: world });
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

test("log the insertions of an append-only table alone, refusing its updates", async () => {
    const { database } = await open();

    // insert two readings, delete the first and try to change the second
    await database.transaction((transaction) =>
        transaction
            .insert(reading)
            .values([
                { id: "a", scope: "inbox", value: 1 },
                { id: "b", scope: "inbox", value: 2 },
            ])
            .execute(),
    );
    await database.transaction((transaction) =>
        transaction.delete(reading).where(eq(reading.id, "a")).execute(),
    );
    const updated = await database
        .transaction((transaction) =>
            transaction.update(reading).set({ value: 3 }).where(eq(reading.id, "b")).execute(),
        )
        .then(
            () => "updated",
            (error: unknown) => String(error instanceof Error ? (error.cause ?? error) : error),
        );

    // log both insertions and no deletion, and leave the second reading unchanged
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
});

test("read the position of a scope's latest change to some tables, past later changes to others", async () => {
    // write a note and a revision in the inbox, then a note in the archive
    const { database, clock } = await open();
    await database.transaction((transaction) =>
        transaction
            .insert(note)
            .values({ ...first, id: "n" })
            .execute(),
    );
    const noted = await database.log.position();
    await database.transaction(
        async (transaction) =>
            await transaction
                .insert(revision)
                .values({ scope: "inbox", noteId: "n", number: 1, title: "R" }),
    );
    const revised = await database.log.position();
    await database.transaction((transaction) =>
        transaction
            .insert(note)
            .values({ ...first, id: "m", scope: "archive" })
            .execute(),
    );
    const archived = await database.log.position();
    const changed = {
        notes: await database.log.changed([note], ["inbox"]),
        both: await database.log.changed([note, revision], ["inbox"]),
        archive: await database.log.changed([note], ["archive"]),
        none: await database.log.changed([], ["inbox"]),
        mine: await database.transaction(async (transaction) => {
            // leave out the transaction's uncommitted write
            await transaction.insert(note).values({ ...first, id: "o" });

            return await transaction.log.changed([note], ["inbox"]);
        }),
    };

    // place the inbox's notes at the horizon once compaction removed the notes' changes
    const before = await database.log.position();
    await database.log.compact(clock.now() + 1000, clock.now());
    expect({ ...changed, compacted: await database.log.changed([note], ["inbox"]) }).toEqual({
        notes: noted,
        both: revised,
        archive: archived,
        none: { ...archived, sequence: 0 },
        mine: noted,
        compacted: before,
    });
});

test("read the log's position from memory until a commit writes the log, past commits of unlogged tables", async () => {
    const { database } = await open();
    const { state } = database.driver;
    const counted = async () => {
        const before = state.statements;
        const { sequence } = await database.log.position();

        return { sequence, statements: state.statements - before };
    };

    // hear other writers' commits as a serving store does, catch up once, then read the position
    await database.log.listening();
    await database.log.position();
    await database.log.position();
    const warm = await counted();

    // commit an unlogged lease, then a logged note
    await database.transaction((transaction) =>
        transaction.insert(lease).values({ name: "a", expiresAt: 1 }).execute(),
    );
    const afterLease = await counted();
    await database.transaction((transaction) => transaction.insert(note).values(first).execute());
    const afterNote = await counted();

    // read the position again only once the log has a new entry
    expect({ warm, afterLease, afterNote }).toEqual({
        warm: { sequence: 0, statements: 0 },
        afterLease: { sequence: 0, statements: 0 },
        afterNote: { sequence: 1, statements: 1 },
    });
});

test("answer a read from memory until another connection's commit of its table reaches this connection", async () => {
    const { storage, database } = await open("file");
    const other = await storage.connect(changeTables);
    onTestFinished(() => other.close());
    const { state } = database.driver;
    const counted = async () => {
        const before = state.statements;
        const ids = (await database.select().from(note)).map((row) => row.id);

        return { ids, statements: state.statements - before };
    };

    // hear other writers' commits, then read the notes twice
    await database.log.listening();
    const cold = await counted();
    const warm = await counted();

    // commit a note through the other connection, and wait until this connection hears it
    await other.transaction((transaction) => transaction.insert(note).values(first).execute());
    const { sequence } = await other.log.position();
    await database.log.wait(sequence, AbortSignal.timeout(5000));
    const heard = await counted();

    // read the notes anew once the other commit arrives
    expect({ cold, warm, heard }).toEqual({
        cold: { ids: [], statements: 1 },
        warm: { ids: [], statements: 0 },
        heard: { ids: [first.id], statements: 1 },
    });
});

test("answer a read anew once another connection deletes an append-only row, which the log never records", async () => {
    const { storage, database } = await open("file");
    const other = await storage.connect(changeTables);
    onTestFinished(() => other.close());
    const name = reading[TABLE].sqlName;
    await database.transaction((transaction) =>
        transaction.insert(reading).values({ id: "r", scope: "inbox", value: 1 }).execute(),
    );

    // hear commits, then read the readings from the database
    await database.log.listening();
    const readings = async () => (await database.select().from(reading)).map((row) => row.id);
    const before = await readings();

    // delete the reading through the other connection, and wait until this connection hears it
    const { generation } = database.state.commits;
    await other.transaction((transaction) =>
        transaction.delete(reading).where(eq(reading.id, "r")).execute(),
    );
    await database.log.until(
        async (tables) => tables.has(name),
        AbortSignal.timeout(5000),
        generation,
    );

    // read the readings anew without the deleted one
    expect({ before, after: await readings() }).toEqual({ before: ["r"], after: [] });
});
