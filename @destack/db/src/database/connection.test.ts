import { asc } from "../sql/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { TestDatabase } from "../test/database.ts";
import { integer, text } from "../table/column.ts";
import { defineTable } from "../table/table.ts";
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
