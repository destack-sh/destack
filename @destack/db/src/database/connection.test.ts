import { asc } from "../sql/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { TestDatabase } from "../test/database.ts";
import { integer, text } from "../table/column.ts";
import { defineTable, TABLE, type Table } from "../table/table.ts";
import type { WriteOperation } from "./connection.ts";
import { post, remark, section } from "../dependent/test/fixture.ts";
import { testSimulation } from "@destack/test/world";

/** The counters one upsert writes and one remove deletes. */
const COUNTERS = 201;

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

/** Build a count of Ada's counters at one value. */
function counters(count: number, value: number) {
    return Array.from({ length: count }, (_, index) => ({
        scope: "s",
        owner: "ada",
        name: `c${String(index).padStart(4, "0")}`,
        value,
    }));
}

test("upsert rows as they are, get them and remove them by key in one statement each", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create([counter], {
        environment: world,
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
    const statements = database.driver.state.statements;
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
        statements: database.driver.state.statements - statements,
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

test("answer reads of a table declared after opening from memory until it changes", async () => {
    // open a database knowing no tables, migrate counters, which declares them, and listen to other writers
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

    // read twice in one statement, then again after a write
    const counted = async () => {
        const before = database.state.statements;
        await database.select().from(counter);

        return database.state.statements - before;
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
    const storage = await TestDatabase.create([], {
        environment: testSimulation().environment("test"),
    });
    onTestFinished(() => storage.close());
    const { database } = storage;
    const applied = await database.migrate([counter]);
    const before = database.state.statements;
    const again = await database.migrate([counter]);

    // plan the first, then read only the table list and the stamp the second time
    expect({
        applied: applied.steps.length > 0,
        again: again.steps,
        statements: database.state.statements - before,
    }).toEqual({ applied: true, again: [], statements: 2 });
});
