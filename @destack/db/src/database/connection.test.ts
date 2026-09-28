import { expect, onTestFinished, test } from "@destack/test";
import { TEST_DIALECTS, TestDatabase } from "../test/database.ts";
import { asc } from "../query/builder.ts";
import { CHAIN_TERMS } from "../query/predicate.ts";
import { integer, text } from "../table/column.ts";
import { primaryKey } from "../table/constraint.ts";
import { defineTable } from "../table/table.ts";

/** Counters by owner and name. */
const counter = defineTable(
    "write_counter",
    {
        /** The space the counter lives in. */
        scope: text("scope").notNull(),
        /** The owning principal. */
        owner: text("owner").notNull(),
        /** The counter's name within its owner. */
        name: text("name").notNull(),
        /** The counted value. */
        value: integer("value").notNull(),
    },
    {
        log: {},
        constraints: (entry) => [primaryKey({ columns: [entry.owner, entry.name] })],
    },
);

test.for(TEST_DIALECTS)(
    "upsert rows as they are and remove them by key across several statements on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, [counter], { isMigrated: true });
        onTestFinished(() => storage.close());
        const database = storage.database;
        const rows = (count: number, value: number) =>
            Array.from({ length: count }, (_, index) => ({
                scope: "s",
                owner: "ada",
                name: `c${String(index).padStart(4, "0")}`,
                value,
            }));
        const read = () =>
            database.select().from(counter).orderBy(asc(counter.owner), asc(counter.name));

        // insert and update more rows than one key chain holds
        const count = 2 * CHAIN_TERMS + 1;
        await database.upsert(counter, rows(count, 1));
        const inserted = await database.log.position();
        await database.upsert(counter, [...rows(count, 2), { ...rows(1, 7)[0]!, owner: "bob" }]);
        const page = await database.log.read({ tables: [counter], after: inserted.sequence });
        expect([
            (await read()).map((row) => row.value),
            page.changes.map((change) => change.operation).sort(),
        ]).toEqual([
            [...rows(count, 2).map(() => 2), 7],
            [...rows(count, 2).map(() => "update"), "insert"].sort(),
        ]);

        // remove every row but one across several chains
        await database.remove(counter, rows(count, 0));
        expect(await read()).toEqual([{ scope: "s", owner: "bob", name: "c0000", value: 7 }]);
    },
);
