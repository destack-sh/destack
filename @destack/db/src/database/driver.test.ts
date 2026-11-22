import { expect, onTestFinished, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";
import { SimulatedTelemetry } from "@destack/environment/simulation";
import { asc, defineTable, sql, TABLE, text } from "../index.ts";
import { TestDatabase } from "../test/index.ts";

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
        "destack.cache.requests": database.dialect === "sqlite" ? 5 : 2,
        "destack.db.rows": 3,
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

    // expect the commit's one wake, the read's one cache miss, SQLite's three statements prepared, and its queue timing each turn
    expect(grown).toEqual(statements);
});

test("announce a write through all whenever it runs, whatever rows it answers", async () => {
    // open a database with two notes, its commit watch listening
    await using simulation = testSimulation();
    const world = simulation.environment("test");
    const storage = await TestDatabase.create([note], { environment: world, isMigrated: true });
    onTestFinished(() => storage.close());
    const { database } = storage;
    await database.insert(note).values([
        { id: "a", body: "first" },
        { id: "b", body: "second" },
    ]);
    const { commits } = database.state;
    await commits.listen();

    // update a note through all without RETURNING, answering no rows, while waiting for its table
    const stop = new AbortController();
    const announced = commits.waitUntil(
        async (tables) => tables.has(note[TABLE].sqlName),
        stop.signal,
        commits.generation,
    );
    const answered = await database.all(
        sql`UPDATE ${note} SET body = 'updated' WHERE ${note.id} = 'a'`,
    );
    const isAnnounced = await announced;

    // announce the note's table and keep the update
    expect({
        answered,
        isAnnounced,
        notes: await database.select().from(note).orderBy(asc(note.id)),
    }).toEqual({
        answered: [],
        isAnnounced: true,
        notes: [
            { id: "a", body: "updated" },
            { id: "b", body: "second" },
        ],
    });
});

test("tell commit waiters of a row's write, and nothing of a moved checkpoint", async () => {
    // open a database with a note, its commit watch listening
    await using simulation = testSimulation();
    const world = simulation.environment("test");
    const storage = await TestDatabase.create([note], { environment: world, isMigrated: true });
    onTestFinished(() => storage.close());
    const { database } = storage;
    await database.insert(note).values([{ id: "a", body: "first" }]);
    await database.state.commits.listen();

    // wait on the log while a checkpoint moves, then while the note changes
    const stop = new AbortController();
    const told: string[][] = [];
    const waiting = database.log.waitUntil(async (tables) => {
        told.push([...tables].toSorted());

        return tables.has(note[TABLE].sqlName);
    }, stop.signal);
    const { sequence } = await database.log.position();
    await database.log.move({ name: "reader", tables: [note] }, 0, sequence, world.clock.now());
    await database.all(sql`UPDATE ${note} SET body = 'updated' WHERE ${note.id} = 'a'`);
    const isTold = await waiting;

    // leave out the first check, which runs before any commit
    expect({ isTold, told: told.filter((tables) => tables.length > 0) }).toEqual({
        isTold: true,
        told: [[note[TABLE].sqlName]],
    });
});
