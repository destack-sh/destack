import { expect, onTestFinished, test } from "@destack/test";
import { TEST_DIALECTS, TestDatabase } from "../test/database.ts";
import { defineTable, TABLE, type Table } from "../table/table.ts";
import { boolean, integer, text } from "../table/column.ts";
import { Order, type OrderKey } from "./order.ts";

/** The random orders per dialect. */
const ORDERS = 100;

/** Rows with nullable orderable columns and many ties. */
const sample = defineTable("order_sample", {
    /** The row's key. */
    id: integer("id").primaryKey(),
    /** Text, with case, accents and astral characters. */
    name: text("name"),
    /** An integer. */
    rank: integer("rank"),
    /** A boolean. */
    isOpen: boolean("is_open"),
    /** An instant. */
    dueAt: integer("due_at"),
});

/** The values of each column. */
const VALUES: Readonly<Record<string, readonly unknown[]>> = {
    name: ["a", "B", "é", "😀", "￿", "", null],
    rank: [-1, 0, 3, null],
    isOpen: [true, false, null],
    dueAt: [0, 1790244000123, null],
};

/** Draw a seeded pseudo-random number. */
function random(seed: { value: number }): number {
    seed.value = (seed.value * 1_103_515_245 + 12_345) % 2_147_483_648;

    return seed.value / 2_147_483_648;
}

/** Pick one element at random. */
function pick<Value>(values: readonly Value[], seed: { value: number }): Value {
    const picked = values[Math.floor(random(seed) * values.length)];
    if (picked === undefined) {
        throw new RangeError("pick from at least one value");
    }

    return picked;
}

test.for(TEST_DIALECTS)("sort and page rows alike in SQL and in memory on %s", async (dialect) => {
    const storage = await TestDatabase.create(dialect, [sample], { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;

    // insert rows that tie often
    const seed = { value: 11 };
    const rows = Array.from({ length: 50 }, (_, id) =>
        sample[TABLE].decode(
            Object.fromEntries([
                ["id", id],
                ...Object.entries(VALUES).map(
                    ([name, values]) => [name, pick(values, seed)] as const,
                ),
            ]),
        ),
    );
    const untyped: Table = sample;
    await database.upsert(untyped, rows);

    // sort by random keys and page after a random row
    for (let index = 0; index < ORDERS; index += 1) {
        const keys: OrderKey[] = Array.from({ length: 1 + Math.floor(random(seed) * 3) }, () => ({
            column: pick(Object.keys(VALUES), seed),
            direction: pick(["asc", "desc"] as const, seed),
        }));
        const order = Order.complete(keys, sample);
        const sorted = rows.toSorted((left, right) => Order.rows(order, left, right));
        const boundary = pick(sorted, seed);

        // match the SQL order and page
        const selected = await database
            .select({ id: sample.id })
            .from(sample)
            .orderBy(...Order.render(order, sample));
        const following = await database
            .select({ id: sample.id })
            .from(sample)
            .where(Order.after(order, sample, boundary))
            .orderBy(...Order.render(order, sample));
        const after = sorted.slice(sorted.indexOf(boundary) + 1);
        expect([order, selected.map((row) => row.id), following.map((row) => row.id)]).toEqual([
            order,
            sorted.map((row) => row["id"]),
            after.map((row) => row["id"]),
        ]);
    }
});

test("refuse orders naming a column twice or more than sixteen keys", () => {
    // refuse a repeated column and too many keys
    const key = { column: "id", direction: "asc" as const };
    expect(() => Order.require([key, key], sample)).toThrow(
        "order has more than 16 keys, or one column twice",
    );
    expect(() =>
        Order.require(
            Array.from({ length: 17 }, () => key),
            sample,
        ),
    ).toThrow("order has more than 16 keys, or one column twice");
});
