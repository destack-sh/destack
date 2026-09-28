import { expect, onTestFinished, test } from "@destack/test";
import { asc } from "drizzle-orm";
import { TEST_DIALECTS, TestDatabase } from "../test/database.ts";
import { defineTable } from "../table/table.ts";
import { integer, real, text } from "../table/column.ts";
import { Expression } from "./expression.ts";

/** The random expressions per dialect. */
const EXPRESSIONS = 300;

/** Rows with a nullable column of each expression kind. */
const sample = defineTable("expression_sample", {
    /** The row's key. */
    id: integer("id").primaryKey(),
    /** An integer. */
    count: integer("count"),
    /** A real number. */
    ratio: real("ratio"),
    /** Text. */
    name: text("name"),
});

/** The values of each column. */
const VALUES = {
    count: [-3, 0, 1, 7, 1_000_003, null],
    ratio: [-0.5, 0, 0.1, 3.25, null],
    name: ["a", "é", "", null],
} as const;

/** Draw a seeded pseudo-random number. */
function random(seed: { value: number }): number {
    seed.value = (seed.value * 1_103_515_245 + 12_345) % 2_147_483_648;

    return seed.value / 2_147_483_648;
}

/** Pick one element at random. */
function pick<Value>(values: readonly Value[], seed: { value: number }): Value {
    return values[Math.floor(random(seed) * values.length)]!;
}

/** Build a random nested numeric expression. */
function draw(seed: { value: number }, depth: number): Expression {
    // read or combine
    const choice = depth === 0 ? Math.floor(random(seed) * 2) : Math.floor(random(seed) * 7);
    if (choice === 0) {
        return Expression.column(pick(["count", "ratio"], seed));
    } else if (choice === 1) {
        return Expression.literal(pick([0, 2, -1.5, 0.1, null], seed));
    } else if (choice === 2) {
        return Expression.add(draw(seed, depth - 1), draw(seed, depth - 1));
    } else if (choice === 3) {
        return Expression.subtract(draw(seed, depth - 1), draw(seed, depth - 1));
    } else if (choice === 4) {
        return Expression.multiply(draw(seed, depth - 1), draw(seed, depth - 1));
    } else if (choice === 5) {
        return Expression.divide(draw(seed, depth - 1), draw(seed, depth - 1));
    }

    return Expression.coalesce(draw(seed, depth - 1), draw(seed, depth - 1));
}

test.for(TEST_DIALECTS)(
    "compute random expressions alike in SQL and in memory on %s",
    async (dialect) => {
        const test = await TestDatabase.create(dialect, [sample], { isMigrated: true });
        onTestFinished(() => test.close());

        // hold random rows
        const seed = { value: 11 };
        const rows = Array.from({ length: 30 }, (_, id) => ({
            id,
            count: pick(VALUES.count, seed),
            ratio: pick(VALUES.ratio, seed),
            name: pick(VALUES.name, seed),
        }));
        await test.database.insert(sample).values(rows);

        // compute each expression in SQL and memory alike
        for (let index = 0; index < EXPRESSIONS; index += 1) {
            const drawn = draw(seed, 3);
            Expression.require(drawn, sample);
            const selected = await test.database
                .select({ value: Expression.render(drawn, sample) })
                .from(sample)
                .orderBy(asc(sample.id));
            const computed = rows.map((row) => Expression.evaluate(drawn, row));
            expect([drawn, selected.map((row) => row.value)]).toEqual([drawn, computed]);
        }
    },
);

test.for(TEST_DIALECTS)("take the first present text on %s", async (dialect) => {
    const test = await TestDatabase.create(dialect, [sample], { isMigrated: true });
    onTestFinished(() => test.close());
    await test.database.insert(sample).values([
        { id: 1, name: "a" },
        { id: 2, name: null },
    ]);

    // fall back to a literal
    const expression = Expression.coalesce(Expression.column("name"), Expression.literal("none"));
    Expression.require(expression, sample);
    const selected = await test.database
        .select({ value: Expression.render(expression, sample) })
        .from(sample)
        .orderBy(asc(sample.id));
    expect(selected).toEqual([{ value: "a" }, { value: "none" }]);
});

test("refuse expressions mixing numbers with text", () => {
    expect(() =>
        Expression.require(
            Expression.add(Expression.column("count"), Expression.column("name")),
            sample,
        ),
    ).toThrow("expression mixes numbers with other values");
});
