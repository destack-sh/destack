import { expect, onTestFinished, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";
import { SimulatedTelemetry } from "@destack/environment/simulation";
import { defineTable, text } from "../index.ts";
import { TestDatabase } from "../test/index.ts";
import { TelemetryScope } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";

/** The notes a database keeps. */
const note = defineTable("telemetry_note", {
    /** The note's name. */
    id: text("id").primaryKey(),
    /** The note's text. */
    body: text("body").notNull(),
});

test("count a database's statements, transactions, commits and connection waits by their target", async () => {
    // open a database on a recorded telemetry listening for commits, then insert a note and read it back
    await using simulation = testSimulation();
    const world = simulation.environment("test");
    const telemetry = new SimulatedTelemetry(world);
    const storage = await TestDatabase.create([note], {
        environment: { ...world, telemetry },
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const { database } = storage;
    await database.state.commits.listen();
    const before = await telemetry.counts();
    await database.transaction((transaction) =>
        transaction.insert(note).values({ id: "a", body: "first" }).execute(),
    );
    await database.select().from(note);

    // count each signal's records over the work, leaving out the seconds, which vary
    const grown: Record<string, number> = {};
    for (const [key, value] of Object.entries(await telemetry.counts())) {
        const growth = value - (before[key] ?? 0);
        if (!key.includes(".seconds") && growth !== 0) {
            grown[key] = growth;
        }
    }

    // expect the insert's transaction and the log compaction its first write makes due, each attempted once
    const name = database.state.name;
    const queued =
        database.dialect === "sqlite"
            ? {
                  "db.client.connection.wait_time.count": 3,
                  [`db.client.connection.wait_time.count.${name}`]: 3,
              }
            : {};
    const statements = {
        ...queued,
        "destack.cache.requests": 1,
        "destack.watch.wakes": 1,
        "db.client.operation.duration.count": 5,
        [`db.client.operation.duration.count.${name}:INSERT:destack__db__telemetry_note`]: 1,
        [`db.client.operation.duration.count.${name}:SELECT:__destack_log`]: 1,
        [`db.client.operation.duration.count.${name}:SELECT:destack__db__telemetry_note`]: 1,
        [`db.client.operation.duration.count.${name}:SELECT:__destack_log`]: 1,
        [`db.client.operation.duration.count.${name}:TRANSACTION`]: 2,
        "destack.transaction.run.duration.count": 2,
        [`destack.transaction.run.duration.count.${name}:database`]: 2,
    };

    // expect the commit's one wake, the read's one cache miss, and SQLite's queue timing each statement's turn
    expect(grown).toEqual(statements);
});

test("record each statement text under the span path that ran it, with its runs and rows", async () => {
    // insert two notes in a named work, then read them outside any
    await using simulation = testSimulation();
    const world = simulation.environment("test");
    const telemetry = new SimulatedTelemetry(world);
    const storage = await TestDatabase.create([note], {
        environment: { ...world, telemetry },
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const { database } = storage;
    const scope = TelemetryScope.of(import.meta.destack.package, telemetry);
    await scope.span("destack.note.create", { "destack.note.name": "a" }, () =>
        database.transaction(async (transaction) => {
            await transaction.insert(note).values({ id: "a", body: "first" }).execute();
            await transaction.insert(note).values({ id: "b", body: "second" }).execute();
        }),
    );
    await database.select().from(note);

    // expect one record per text and span path, the inserts' under the span's transaction, leaving out times and hashes
    const name = database.state.name;
    const records = database.state.statementRecords
        .list()
        .filter(({ summary }) => /^(INSERT|SELECT) destack__db__telemetry_note$/u.test(summary))
        .map(({ path, summary, count, rows }) => ({ path, summary, count, rows }));
    expect(records.toSorted((left, right) => left.path.localeCompare(right.path))).toEqual([
        { path: "(none)", summary: "SELECT destack__db__telemetry_note", count: 1, rows: 2 },
        {
            path: `destack.note.create[a]>destack.transaction.run[${name}:database]`,
            summary: "INSERT destack__db__telemetry_note",
            count: 2,
            rows: 2,
        },
    ]);
});
