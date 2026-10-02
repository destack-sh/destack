import { CHAIN_TERMS, asc } from "../sql/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { TEST_DIALECTS, TestDatabase } from "../test/database.ts";
import { integer, text } from "../table/column.ts";
import { defineTable } from "../table/table.ts";

/** Counters by owner and name. */
const counter = defineTable(
    "write_counter",
    {
        /** The space the counter lives in. */
        scope: text("scope").notNull(),
        /** The owning principal. */
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

test.for(TEST_DIALECTS)(
    "upsert rows as they are and remove them by key across several statements on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, [counter], { isMigrated: true });
        onTestFinished(() => storage.close());
        const database = storage.database;
        const read = () =>
            database.select().from(counter).orderBy(asc(counter.owner), asc(counter.name));

        // insert and update more rows than one key chain matches
        const count = 2 * CHAIN_TERMS + 1;
        await database.upsert(counter, counters(count, 1));
        const inserted = await database.log.position();
        const bob = { scope: "s", owner: "bob", name: "c0000", value: 7 };
        await database.upsert(counter, [...counters(count, 2), bob]);
        const page = await database.log.read({ tables: [counter], after: inserted.sequence });
        expect([
            (await read()).map((row) => row.value),
            page.changes.map((change) => change.operation).toSorted(),
        ]).toEqual([
            [...counters(count, 2).map(() => 2), 7],
            [...counters(count, 2).map(() => "update"), "insert"].toSorted(),
        ]);

        // remove every row but one across several chains
        await database.remove(counter, counters(count, 0));
        expect(await read()).toEqual([bob]);
    },
);

test.for(TEST_DIALECTS)(
    "roll back a transaction its callback rolls back, keeping what the callback read on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, [counter], { isMigrated: true });
        onTestFinished(() => storage.close());
        const database = storage.database;

        // write and read in a rehearsal, which rolls the writes back
        const read = await database.rehearse(async (transaction) => {
            await transaction.upsert(counter, [
                { scope: "s", owner: "ada", name: "rolled", value: 1 },
            ]);

            return transaction.select().from(counter);
        });

        // keep the read and find nothing written
        expect([read, await database.select().from(counter)]).toEqual([
            [{ scope: "s", owner: "ada", name: "rolled", value: 1 }],
            [],
        ]);
    },
);
