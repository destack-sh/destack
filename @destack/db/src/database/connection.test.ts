import { asc } from "../sql/index.ts";
import { expect, onTestFinished, Recording, test, testOn } from "@destack/test";
import init from "@sqlite.org/sqlite-wasm";
import { WasmClient } from "../browser/client.ts";
import { connectWasmSqlite } from "../browser/connection.ts";
import { note } from "../log/test/fixture.ts";
import { TestDatabase } from "../test/database.ts";
import { integer, text } from "../table/column.ts";
import { defineTable, TABLE, type Table } from "../table/table.ts";
import type { WriteOperation } from "./connection.ts";
import { post, remark, section } from "../dependent/test/fixture.ts";
import { testSimulation } from "@destack/test/world";
import { localState, readManagers, readState, STAMP, STATE } from "../migration/state.ts";
import { sql } from "../sql/index.ts";
import { quote } from "../dialect/quote.ts";
import { relation } from "../table/namespace.ts";

/** The counters one upsert writes and one remove deletes. */
const COUNTERS = 201;

/** Order two pairs by their first text. */
function byName(left: readonly [string, string], right: readonly [string, string]): number {
    return left[0].localeCompare(right[0]);
}

/** Counters by owner and name. */
const counter = defineTable(
    "write_counter",
    {
        /** The space the counter lives in. */
        scope: text("scope").notNull(),
        /** The owner principal. */
        owner: text("owner").primaryKey(),
        /** The counter's name within its owner. */
        name: text("name").primaryKey(),
        /** The counted value. */
        value: integer("value").notNull(),
    },
    {
        log: {},
    },
);

/** One counter's row. */
const VISITS = { scope: "a", owner: "ada", name: "visits", value: 3 };

/** List tables by their SQL names. */
function sqlNames(tables: readonly Table[]): string[] {
    return tables.map((table) => table[TABLE].sqlName);
}

/** Build a count of Ada's counters at one value. */
function counters(count: number, value: number) {
    return Array.from({ length: count }, (_, index) => ({
        scope: "s",
        owner: "ada",
        name: `c${String(index).padStart(4, "0")}`,
        value,
    }));
}

/** Test against SQLite alone, as the file tests a connector of it. */
const onSqlite = testOn("sqlite");

test("upsert rows as they are, get them and remove them by key in one statement each", async () => {
    // run in a world of its own
    const simulation = testSimulation();
    const storage = await TestDatabase.create([counter], {
        environment: simulation.environment("test"),
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    const read = () =>
        database.select().from(counter).orderBy(asc(counter.owner), asc(counter.name));

    // insert and update the counters
    await database.transaction(
        async (transaction) => await transaction.upsert(counter, counters(COUNTERS, 1)),
    );
    const inserted = await database.log.position();
    const bob = { scope: "s", owner: "bob", name: "c0000", value: 7 };
    await database.transaction(
        async (transaction) => await transaction.upsert(counter, [...counters(COUNTERS, 2), bob]),
    );
    const page = await database.log.read({ tables: [counter], after: inserted.sequence });
    expect([
        (await read()).map((row) => row.value),
        page.changes.map((change) => change.operation).toSorted(),
    ]).toEqual([
        [...counters(COUNTERS, 2).map(() => 2), 7],
        [...counters(COUNTERS, 2).map(() => "update"), "insert"].toSorted(),
    ]);

    // get Bob's counter, two of Ada's and a missing one in one statement
    const before = await Recording.read(simulation.telemetry);
    const found = await database.get(counter, [
        bob,
        ...counters(2, 0),
        { owner: "carol", name: "c0000" },
    ]);
    expect({
        rows: found.toSorted((left, right) =>
            JSON.stringify([left["owner"], left["name"]]) <
            JSON.stringify([right["owner"], right["name"]])
                ? -1
                : 1,
        ),
        statements: (await Recording.read(simulation.telemetry))
            .since(before)
            .count(TestDatabase.statements(database)),
    }).toEqual({ rows: [...counters(2, 2), bob], statements: 1 });

    // remove every row but one
    await database.transaction((transaction) => transaction.remove(counter, counters(COUNTERS, 0)));
    expect(await read()).toEqual([bob]);
});

test("list the tables each write operation changes, with tables declared after opening", async () => {
    // open posts and their remarks, sections declared later
    const storage = await TestDatabase.create([remark, post], {
        environment: testSimulation().environment("test"),
    });
    onTestFinished(() => storage.close());
    const { state } = storage.database;
    const cascade = (table: Table, operation: WriteOperation) =>
        [...state.cascade([table[TABLE].sqlName], operation)]
            .map((name) => name.replace("destack__db__", ""))
            .toSorted();
    const before = {
        insert: cascade(post, "INSERT"),
        delete: cascade(post, "DELETE"),
        section: cascade(section, "DELETE"),
    };
    state.declare([section]);

    // delete dependents and referencing rows only on a delete, reaching the later sections too
    expect({
        before,
        after: {
            insert: cascade(post, "INSERT"),
            update: cascade(post, "UPDATE"),
            delete: cascade(post, "DELETE"),
            section: cascade(section, "DELETE"),
        },
    }).toEqual({
        before: {
            insert: ["dependent_post"],
            delete: ["dependent_post", "dependent_remark"],
            section: ["dependent_section"],
        },
        after: {
            insert: ["dependent_post"],
            update: ["dependent_post"],
            delete: ["dependent_post", "dependent_remark", "dependent_section"],
            section: ["dependent_remark", "dependent_section"],
        },
    });
});

test("declare a table migrated after opening on the connection and every connection sharing its state", async () => {
    // open posts and their remarks, then migrate counters as another manager
    const storage = await TestDatabase.create([remark, post], {
        environment: testSimulation().environment("test"),
    });
    onTestFinished(() => storage.close());
    const { database } = storage;
    const before = sqlNames(database.tables);
    await database.migrate([counter], { manager: "counters" });

    // list the counters after the opened tables, in a transaction's connection over the same state too
    const opened = sqlNames([remark, post]);
    expect({
        before,
        after: sqlNames(database.tables),
        transaction: await database.transaction(async (transaction) =>
            sqlNames(transaction.tables),
        ),
    }).toEqual({
        before: opened,
        after: [...opened, ...sqlNames([counter])],
        transaction: [...opened, ...sqlNames([counter])],
    });
});

test("answer reads of a table declared after opening from memory until it changes", async () => {
    // open a database knowing no tables, migrate counters, which declares them, and listen to other writers
    const simulation = testSimulation();
    const storage = await TestDatabase.create([], { environment: simulation.environment("test") });
    onTestFinished(() => storage.close());
    const { database } = storage;
    await database.migrate([counter]);
    await database.log.listening();
    await database.transaction(async (transaction) => {
        await transaction.insert(counter).values(counters(1, 1));
    });

    // read twice in one statement, then again after a write
    const counted = async () => {
        const before = await Recording.read(simulation.telemetry);
        await database.select().from(counter);

        return (await Recording.read(simulation.telemetry))
            .since(before)
            .count(TestDatabase.statements(database));
    };
    const reads = [await counted(), await counted()];
    await database.transaction(async (transaction) => {
        await transaction
            .insert(counter)
            .values({ scope: "s", owner: "ada", name: "other", value: 2 });
    });
    reads.push(await counted());
    expect(reads).toEqual([1, 0, 1]);
});

test("share one frozen copy of a cached read with every reader", async () => {
    // migrate counters, listen to other writers, and write one
    const storage = await TestDatabase.create([], {
        environment: testSimulation().environment("test"),
    });
    onTestFinished(() => storage.close());
    const { database } = storage;
    await database.migrate([counter]);
    await database.log.listening();
    await database.transaction(async (transaction) => {
        await transaction.insert(counter).values(counters(1, 1));
    });

    // read the same raw rows twice, the second from memory
    const statement = database.select().from(counter);
    const [first, second] = [await database.all(statement), await database.all(statement)];
    expect({
        isShared: first === second,
        isFrozen: Object.isFrozen(first) && first.every((row) => Object.isFrozen(row)),
    }).toEqual({ isShared: true, isFrozen: true });
});

test("skip planning a declaration the database applied last, by its stamp", async () => {
    // migrate counters, then migrate the same declaration again
    const simulation = testSimulation();
    const storage = await TestDatabase.create([], { environment: simulation.environment("test") });
    onTestFinished(() => storage.close());
    const { database } = storage;
    const applied = await database.migrate([counter]);
    const before = await Recording.read(simulation.telemetry);
    const again = await database.migrate([counter]);

    // plan the first, then read only the table list and the stamp the second time
    expect({
        applied: applied.steps.length > 0,
        again: again.steps,
        statements: (await Recording.read(simulation.telemetry))
            .since(before)
            .count(TestDatabase.statements(database)),
    }).toEqual({ applied: true, again: [], statements: 2 });
});

test("read each applied table's manager as its applied state names it", async () => {
    // migrate counters under one manager and tallies under another
    const simulation = testSimulation();
    const storage = await TestDatabase.create([], { environment: simulation.environment("test") });
    onTestFinished(() => storage.close());
    const { database } = storage;
    const tally = defineTable("connection_tally", { id: text("id").primaryKey() });
    await database.migrate([counter], { manager: "counters" });
    await database.migrate([tally], { manager: "tallies" });

    // read the managers alone, and from the whole applied states
    const { namespace } = database.state;
    const applied = await readState(database);
    const named = applied.map(
        (entry) => [localState(entry.state, namespace).table.name, entry.manager] as const,
    );
    expect({
        managers: [...(await readManagers(database))].toSorted(byName),
        named: named.toSorted(byName),
    }).toEqual({
        managers: [
            [tally[TABLE].sqlName, "tallies"],
            [counter[TABLE].sqlName, "counters"],
        ],
        named: [
            [tally[TABLE].sqlName, "tallies"],
            [counter[TABLE].sqlName, "counters"],
        ],
    });
});

test("open a database migrated before managers, keeping its tables under the database's manager", async () => {
    // migrate counters, then give the state and stamp tables the shape from before managers
    const simulation = testSimulation();
    const storage = await TestDatabase.create([], { environment: simulation.environment("test") });
    onTestFinished(() => storage.close());
    const { database } = storage;
    await database.migrate([counter]);
    await database.transaction((transaction) =>
        transaction.insert(counter).values(VISITS).execute(),
    );
    const { namespace } = database.state;
    const [state, stamp] = [STATE, STAMP].map((name) => quote(relation(name, namespace)));
    await database.executeScript(`ALTER TABLE ${state} DROP COLUMN manager`);
    await database.executeScript(`DROP TABLE ${stamp}`);
    await database.executeScript(
        `CREATE TABLE ${stamp} (slot INTEGER PRIMARY KEY, digest TEXT NOT NULL)`,
    );
    await database.executeScript(`INSERT INTO ${stamp} (slot, digest) VALUES (1, 'before')`);

    // read its unapplied tables, migrate the same declaration, then give it the old stamp alone once more
    const unapplied = await database.unapplied([counter]);
    const upgraded = await database.migrate([counter]);
    await database.executeScript(`DROP TABLE ${stamp}`);
    await database.executeScript(
        `CREATE TABLE ${stamp} (slot INTEGER PRIMARY KEY, digest TEXT NOT NULL)`,
    );
    const again = await database.migrate([counter]);

    // change no table, keep its rows, and stamp the declaration for the database's manager
    expect({
        unapplied,
        steps: [upgraded.steps, again.steps],
        managers: [...(await readManagers(database))],
        rows: await database.select().from(counter),
        stamps: await database.all(sql`SELECT manager FROM ${sql.relation(STAMP)}`),
    }).toEqual({
        unapplied: [],
        steps: [[], []],
        managers: [[counter[TABLE].sqlName, "database"]],
        rows: [VISITS],
        stamps: [{ manager: "database" }],
    });
});

onSqlite(
    "replicate the replicated tables again on a connection that finds them migrated",
    async () => {
        // migrate a table as a replica on one connection to a database
        const world = testSimulation().environment("test");
        const sqlite = await init();
        const client = new WasmClient(new sqlite.oo1.DB(":memory:"), world);
        const connect = () =>
            connectWasmSqlite(client, [note], { name: "memory", environment: world });
        const options = { isReplica: true, replicated: new Set([note[TABLE].sqlName]) };
        const first = connect();
        await first.migrate([note], options);

        // open the database again, as a reloaded page does, finding the declaration stamped
        const second = connect();
        onTestFinished(() => second.close());
        const plan = await second.migrate([note], options);

        // expect no steps, and the table read as a replica on both connections
        expect({
            steps: plan.steps.length,
            first: first.replicates(note),
            second: second.replicates(note),
        }).toEqual({ steps: 0, first: true, second: true });
    },
);
