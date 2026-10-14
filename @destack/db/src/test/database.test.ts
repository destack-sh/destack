import { expect, onTestFinished, test } from "@destack/test";
import { asc } from "../sql/index.ts";
import { integer, text } from "../table/column.ts";
import { defineTable } from "../table/table.ts";
import { TEST_DIALECTS, TestDatabase, TestSeed } from "./database.ts";

/** Counters by name. */
const tally = defineTable("seed_tally", {
    /** The counter's name. */
    name: text("name").primaryKey(),
    /** The counted value. */
    value: integer("value").notNull(),
});

test.for(TEST_DIALECTS)(
    "start each seeded copy from the rows the seed writes once, keeping every copy's writes its own, on %s",
    async (dialect) => {
        // seed one counter, counting the writes
        let writes = 0;
        const seed = new TestSeed(async (storage) => {
            writes += 1;
            await storage.database.insert(tally).values({ name: "seeded", value: 1 });

            return "written";
        });

        // copy the seed twice and write into the first copy only
        const copy = async () => {
            const { storage, value } = await TestDatabase.seeded(dialect, [tally], seed);
            onTestFinished(() => storage.close());

            return { database: storage.database, value };
        };
        const first = await copy();
        await first.database.insert(tally).values({ name: "first", value: 2 });
        const second = await copy();
        const rows = async (database: typeof first.database) =>
            database.select().from(tally).orderBy(asc(tally.name));

        // read the seed's row in both copies, and only the first copy's own row in it
        expect({
            writes,
            values: [first.value, second.value],
            first: await rows(first.database),
            second: await rows(second.database),
        }).toEqual({
            writes: 1,
            values: ["written", "written"],
            first: [
                { name: "first", value: 2 },
                { name: "seeded", value: 1 },
            ],
            second: [{ name: "seeded", value: 1 }],
        });
    },
);
