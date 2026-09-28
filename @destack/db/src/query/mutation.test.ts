import { expect, onTestFinished, test } from "@destack/test";
import { TEST_DIALECTS, TestDatabase } from "../test/database.ts";
import { defineTable } from "../table/table.ts";
import { integer, text } from "../table/column.ts";
import { primaryKey } from "../table/constraint.ts";
import { eq, gt } from "./predicate.ts";

/** Counters keyed by owner and name. */
const counter = defineTable(
    "mutation_counter",
    {
        /** The owning principal. */
        owner: text("owner").notNull(),
        /** The counter's name within its owner. */
        name: text("name").notNull(),
        /** The counted value. */
        value: integer("value").notNull(),
    },
    { constraints: (entry) => [primaryKey({ columns: [entry.owner, entry.name] })] },
);

test.for(TEST_DIALECTS)("return exactly the rows each mutation changed on %s", async (dialect) => {
    const test = await TestDatabase.create(dialect, [counter], { isMigrated: true });
    onTestFinished(() => test.close());
    const database = test.database;

    // return inserted rows in insertion order
    expect(
        await database
            .insert(counter)
            .values([
                { owner: "ada", name: "b", value: 1 },
                { owner: "ada", name: "a", value: 2 },
            ])
            .returning(),
    ).toEqual([
        { owner: "ada", name: "b", value: 1 },
        { owner: "ada", name: "a", value: 2 },
    ]);

    // return only the rows a conflict let through
    expect(
        await database
            .insert(counter)
            .values([
                { owner: "ada", name: "a", value: 9 },
                { owner: "bob", name: "a", value: 3 },
            ])
            .onConflictDoNothing()
            .returning({ owner: counter.owner, value: counter.value }),
    ).toEqual([{ owner: "bob", value: 3 }]);

    // return inserted and updated rows of an upsert on the key
    expect(
        await database
            .insert(counter)
            .values([
                { owner: "ada", name: "a", value: 5 },
                { owner: "cyd", name: "a", value: 7 },
            ])
            .onConflictDoUpdate({ target: [counter.owner, counter.name], set: { value: 5 } })
            .returning({ owner: counter.owner, value: counter.value }),
    ).toEqual([
        { owner: "ada", value: 5 },
        { owner: "cyd", value: 7 },
    ]);

    // return updated rows as changed, even when the change leaves the predicate
    expect(
        await database
            .update(counter)
            .set({ value: 0 })
            .where(gt(counter.value, 4))
            .returning({ owner: counter.owner, value: counter.value }),
    ).toEqual([
        { owner: "ada", value: 0 },
        { owner: "cyd", value: 0 },
    ]);

    // return deleted rows as they were, and nothing for no match
    expect([
        await database
            .delete(counter)
            .where(eq(counter.owner, "bob"))
            .returning({ owner: counter.owner, value: counter.value }),
        await database.delete(counter).where(eq(counter.owner, "eve")).returning(),
    ]).toEqual([[{ owner: "bob", value: 3 }], []]);
});
