import { expect, onTestFinished, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";
import { TestDatabase } from "../test/database.ts";
import { changeTables, first, note } from "../log/test/fixture.ts";
import { CompactionController } from "./compaction.ts";

/** How long a database retains windowed changes: a week. */
const WEEK_MILLISECONDS = 7 * 24 * 60 * 60_000;

test("compact once the oldest windowed change leaves the window, and a week on without any", async () => {
    // open a log with no windowed change yet
    await using simulation = testSimulation();
    const world = simulation.environment("test");
    const { clock } = simulation;
    const storage = await TestDatabase.create(changeTables, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const { database } = storage;
    const compaction = new CompactionController(database, clock);
    const empty = await compaction.reconcile();

    // log a change, then look again a day later and once it left the window
    await database.transaction((transaction) => transaction.insert(note).values(first).execute());
    await simulation.jump(clock.now() + 24 * 60 * 60_000);
    const dayLater = await compaction.reconcile();
    await simulation.jump(clock.now() + WEEK_MILLISECONDS);
    const compacted = await compaction.reconcile();
    const oldest = await database.log.compact(clock.now(), clock.now());
    expect({ empty, dayLater, compacted, oldest }).toEqual({
        empty: WEEK_MILLISECONDS,
        dayLater: WEEK_MILLISECONDS - 24 * 60 * 60_000,
        compacted: WEEK_MILLISECONDS,
        oldest: undefined,
    });
});
