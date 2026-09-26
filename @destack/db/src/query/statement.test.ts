import { expect, onTestFinished, test } from "@destack/test";
import { sql } from "drizzle-orm";
import { TEST_DIALECTS, TestDatabase } from "../test/database.ts";
import { defineTable } from "../table/table.ts";
import { integer, text } from "../table/column.ts";
import { jsonElements, Statement } from "./statement.ts";

/** Items a statement reads by a list of identifiers. */
const item = defineTable("statement_item", {
    /** The item's identifier. */
    id: text("id").primaryKey(),
    /** The item's rank. */
    rank: integer("rank").notNull(),
});

/** Items whose identifiers a JSON list names, ranked at least a minimum. */
const listed = new Statement<{ id: string; rank: number | string }>(
    (value) => sql`SELECT ${item.id} AS id, ${item.rank} AS rank
        FROM ${item} JOIN ${jsonElements(value("ids"), "listed")} ON ${item.id} = listed.value ->> 0
        WHERE ${item.rank} >= ${value("minimum")}
        ORDER BY ${item.id}`,
);

test.for(TEST_DIALECTS)(
    "run a statement rendered once with each run's values on %s",
    async (dialect) => {
        const test = await TestDatabase.create(dialect, [item], { isMigrated: true });
        onTestFinished(() => test.close());
        await test.database.insert(item).values([
            { id: "a", rank: 1 },
            { id: "b", rank: 2 },
            { id: "c", rank: 3 },
        ]);

        // read with different values through the same statement, inside and outside a transaction, as driver rows
        const ids = (values: readonly string[]) => JSON.stringify(values.map((id) => [id]));
        const ranked = (rows: readonly { id: string; rank: number | string }[]) =>
            rows.map((row) => ({ id: row.id, rank: Number(row.rank) }));
        const outside = await listed.all(test.database, { ids: ids(["a", "c"]), minimum: 1 });
        const inside = await test.database.transaction((transaction) =>
            listed.all(transaction, { ids: ids(["a", "b", "c"]), minimum: 2 }),
        );
        expect([ranked(outside), ranked(inside)]).toEqual([
            [
                { id: "a", rank: 1 },
                { id: "c", rank: 3 },
            ],
            [
                { id: "b", rank: 2 },
                { id: "c", rank: 3 },
            ],
        ]);
    },
);
