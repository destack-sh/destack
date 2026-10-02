import { eq, gt } from "../sql/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { TEST_DIALECTS, TestDatabase } from "../test/database.ts";
import { defineTable } from "../table/table.ts";
import { integer, json, text } from "../table/column.ts";
import { schema } from "@destack/schema";

/** Counters by owner and name. */
const counter = defineTable("mutation_counter", {
    /** The owning principal. */
    owner: text("owner").primaryKey(),
    /** The counter's name within its owner. */
    name: text("name").primaryKey(),
    /** The counted value. */
    value: integer("value").notNull(),
});

test.for(TEST_DIALECTS)("return exactly the rows each mutation changed on %s", async (dialect) => {
    const storage = await TestDatabase.create(dialect, [counter], { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;

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

    // return inserted and updated rows of an upsert
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

    // return updated rows even when they leave the predicate
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

    // return deleted rows as they were
    expect([
        await database
            .delete(counter)
            .where(eq(counter.owner, "bob"))
            .returning({ owner: counter.owner, value: counter.value }),
        await database.delete(counter).where(eq(counter.owner, "eve")).returning(),
    ]).toEqual([[{ owner: "bob", value: 3 }], []]);
});

/** Labels with a default color. */
const label = defineTable("mutation_label", {
    /** The label's key. */
    id: text("id").primaryKey(),
    /** The label's color. */
    color: text("color").notNull().default("grey"),
});

test.for(TEST_DIALECTS)(
    "insert the same shape twice in one transaction, taking defaults for values left undefined, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, [label], { isMigrated: true });
        onTestFinished(() => storage.close());

        // insert two labels, the second without a color
        const inserted = await storage.database.transaction(async (transaction) => [
            await transaction.insert(label).values({ id: "a", color: "red" }).returning(),
            await transaction.insert(label).values({ id: "b", color: "blue" }).returning(),
            await transaction.insert(label).values({ id: "c", color: undefined }).returning(),
        ]);
        expect(inserted).toEqual([
            [{ id: "a", color: "red" }],
            [{ id: "b", color: "blue" }],
            [{ id: "c", color: "grey" }],
        ]);
    },
);

/** Retentions with a JSON default that is a plain string. */
const retention = defineTable("mutation_retention", {
    /** The retention's key. */
    id: text("id").primaryKey(),
    /** What the retention keeps. */
    keep: json(
        "keep",
        schema.union([schema.literal("forever"), schema.object({ days: schema.number() })]),
    )
        .notNull()
        .default("forever"),
});

test.for(TEST_DIALECTS)(
    "take a JSON default that is a plain string, encoded as JSON, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, [retention], { isMigrated: true });
        onTestFinished(() => storage.close());

        // insert one retention with a window and one taking the default
        await storage.database
            .insert(retention)
            .values([{ id: "a", keep: { days: 1 } }, { id: "b" }]);
        expect(await storage.database.select().from(retention).orderBy(retention.id)).toEqual([
            { id: "a", keep: { days: 1 } },
            { id: "b", keep: "forever" },
        ]);
    },
);
