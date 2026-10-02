import { schema } from "@destack/schema";
import { sql } from "../sql/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { TEST_DIALECTS, TestDatabase } from "../test/database.ts";
import { defineTable } from "../table/table.ts";
import { integer, text } from "../table/column.ts";
import { jsonElements, Statement } from "./statement.ts";

/** Write identifiers as the JSON rows a listed statement reads. */
function ids(values: readonly string[]): string {
    return JSON.stringify(values.map((id) => [id]));
}

/** Items read by a list of identifiers. */
const item = defineTable("statement_item", {
    /** The item's identifier. */
    id: text("id").primaryKey(),
    /** The item's rank. */
    rank: integer("rank").notNull(),
});

/** A listed item, its rank read as a number. */
const RANKED = schema.object({
    id: schema.string(),
    rank: schema.union([schema.number(), schema.string()]).transform((rank) => Number(rank)),
});

/** Listed items at a minimum rank. */
const listed = new Statement(
    (value) => sql`SELECT ${item.id} AS id, ${item.rank} AS rank
        FROM ${item} JOIN ${jsonElements(value("ids"), "listed")} ON ${item.id} = listed.value ->> 0
        WHERE ${item.rank} >= ${value("minimum")}
        ORDER BY ${item.id}`,
);

test.for(TEST_DIALECTS)(
    "run a statement rendered once with each run's values on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, [item], { isMigrated: true });
        onTestFinished(() => storage.close());
        await storage.database.insert(item).values([
            { id: "a", rank: 1 },
            { id: "b", rank: 2 },
            { id: "c", rank: 3 },
        ]);

        // read through one statement with different values
        const ranked = (rows: readonly Record<string, unknown>[]) =>
            rows.map((row) => RANKED.parse(row));
        const outside = await listed.all(storage.database, { ids: ids(["a", "c"]), minimum: 1 });
        const inside = await storage.database.transaction((transaction) =>
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
