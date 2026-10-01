import { expect, onTestFinished, test } from "@destack/test";
import { TEST_DIALECTS, TestDatabase } from "../test/database.ts";
import { defineTable } from "../table/table.ts";
import { bigint, boolean, integer, real, text, type Column } from "../table/column.ts";
import { TABLE } from "../table/table.ts";
import { asc } from "drizzle-orm";
import { Condition, type Scalar } from "./condition.ts";
import { Order } from "./order.ts";

/** The random conditions per dialect. */
const CONDITIONS = 400;

/** Rows with a nullable column of each comparable kind. */
const sample = defineTable("condition_sample", {
    /** The row's key. */
    id: integer("id").primaryKey(),
    /** Text, with case, accents and astral characters. */
    name: text("name"),
    /** An integer. */
    count: integer("count"),
    /** A real number. */
    ratio: real("ratio"),
    /** A boolean. */
    isOpen: boolean("is_open"),
    /** An exact integer beyond the safe range. */
    views: bigint("views"),
    /** An instant. */
    editedAt: integer("edited_at"),
});

/** The literal values of each column. */
const VALUES: Readonly<Record<string, readonly Scalar[]>> = {
    name: ["a", "B", "b", "é", "z", "😀", "￿", "", null],
    count: [-2, 0, 1, 7, null],
    ratio: [-0.5, 0, 0.25, 3.5, null],
    isOpen: [true, false, null],
    views: ["-9007199254740993", "0", "9007199254740993", null],
    editedAt: [0, 1790244000123, 1790244000124, null],
};

/** Draw a seeded pseudo-random number. */
function random(seed: { value: number }): number {
    seed.value = (seed.value * 1_103_515_245 + 12_345) % 2_147_483_648;

    return seed.value / 2_147_483_648;
}

/** Pick one element at random. */
function pick<Value>(values: readonly Value[], seed: { value: number }): Value {
    return values[Math.floor(random(seed) * values.length)]!;
}

/** Build a random nested condition. */
function draw(seed: { value: number }, depth: number): Condition {
    // compare, test or combine
    const name = pick(Object.keys(VALUES), seed);
    const choice = depth === 0 ? Math.floor(random(seed) * 3) : Math.floor(random(seed) * 6);
    if (choice === 0) {
        const operator = pick(["eq", "ne", "lt", "lte", "gt", "gte"] as const, seed);

        return Condition.compare(operator, name, pick(VALUES[name]!, seed));
    } else if (choice === 1) {
        const values = VALUES[name]!.filter(
            (value): value is Exclude<Scalar, null> => value !== null && random(seed) < 0.5,
        );

        return Condition.oneOf(name, values);
    } else if (choice === 2) {
        return Condition.missing(name);
    } else if (choice === 3) {
        return Condition.all(draw(seed, depth - 1), draw(seed, depth - 1));
    } else if (choice === 4) {
        return Condition.any(draw(seed, depth - 1), draw(seed, depth - 1));
    }

    return Condition.not(draw(seed, depth - 1));
}

test.for(TEST_DIALECTS)(
    "decide random conditions alike in SQL and in memory on %s",
    async (dialect) => {
        const test = await TestDatabase.create(dialect, [sample], { isMigrated: true });
        onTestFinished(() => test.close());
        const database = test.database;
        const columns = sample[TABLE].columns as Readonly<Record<string, Column>>;

        // hold every combination of a few values
        const seed = { value: 7 };
        const rows = Array.from({ length: 60 }, (_, id) =>
            Object.fromEntries([
                ["id", id],
                ...Object.entries(VALUES).map(([name, values]) => {
                    const value = pick(values, seed);

                    return [
                        name,
                        value === null ? null : columns[name]!.definition.fromJson(value),
                    ];
                }),
            ]),
        );
        await database.insert(sample).values(rows as never);

        // match each condition in SQL and memory alike
        for (let index = 0; index < CONDITIONS; index += 1) {
            const drawn = draw(seed, 3);
            const selected = await database
                .select({ id: sample.id })
                .from(sample)
                .where(Condition.render(drawn, Condition.bind(sample)))
                .orderBy(asc(sample.id));
            const match = Condition.compile(drawn, sample);
            const matched = rows.filter(
                (row) =>
                    match({
                        column: (name: string) => row[name],
                        parameter: () => null,
                        exists: () => undefined,
                    }) === true,
            );
            expect([drawn, selected.map((row) => row.id)]).toEqual([
                drawn,
                matched.map((row) => row.id),
            ]);
        }
    },
);

test("order text by code point, as UTF-8 bytes order it", () => {
    const texts = ["😀", "￿", "é", "z", "b", "B", "a", ""];
    const bytes = (value: string) => [...new TextEncoder().encode(value)];
    const byByte = [...texts].sort((left, right) => {
        const first = bytes(left);
        const second = bytes(right);
        for (let index = 0; index < Math.min(first.length, second.length); index += 1) {
            if (first[index] !== second[index]) {
                return first[index]! - second[index]!;
            }
        }

        return first.length - second.length;
    });
    expect([...texts].sort(Order.codePoints)).toEqual(byByte);
});

test("refuse conditions beyond a thousand terms", () => {
    // accept a thousand values and refuse one more
    const values = Array.from({ length: 1000 }, (_, index) => index);
    Condition.require(Condition.oneOf("count", values), sample);
    expect(() =>
        Condition.require(
            Condition.all(Condition.oneOf("count", values.slice(1)), Condition.eq("name", "a")),
            sample,
        ),
    ).toThrow("condition holds 1001 terms, more than 1000");
});

test.for(TEST_DIALECTS)("match a thousand listed values in SQL on %s", async (dialect) => {
    const test = await TestDatabase.create(dialect, [sample], { isMigrated: true });
    onTestFinished(() => test.close());
    await test.database.insert(sample).values([
        { id: 1, count: 999 },
        { id: 2, count: 1000 },
    ]);

    // render a thousand values as a balanced tree
    const values = Array.from({ length: 1000 }, (_, index) => index);
    const rows = await test.database
        .select()
        .from(sample)
        .where(Condition.render(Condition.oneOf("count", values), Condition.bind(sample)));
    expect(rows.map((row) => row.id)).toEqual([1]);
});

test("decide relations through the binding, unknown until the relation is read", () => {
    // decide each relation answer
    const condition = Condition.any(
        Condition.not(Condition.exists("marks", Condition.eq("name", "a"))),
        Condition.eq("count", 1),
    );
    const match = Condition.compile(condition, sample);
    const decide = (count: number, exists: boolean | undefined) =>
        match({ column: () => count, parameter: () => null, exists: () => exists });
    expect([decide(1, undefined), decide(0, undefined), decide(0, true), decide(0, false)]).toEqual(
        [true, undefined, false, true],
    );

    // refuse undeclared relations
    expect(() => Condition.render(condition, Condition.bind(sample))).toThrow(
        "condition follows relation marks, which condition_sample does not declare",
    );
});

test("merge ordered rows of two tables into the first of their order, tying by list and key", () => {
    // order two lists by a shared column
    const marks = defineTable("condition_mark", {
        /** The mark's key. */
        id: text("id").primaryKey(),
        /** An integer. */
        count: integer("count"),
    });
    const merged = Order.merge(
        [{ column: "count", direction: "desc" }],
        [
            {
                name: "samples",
                table: sample,
                rows: [
                    { id: 1, count: 5 },
                    { id: 2, count: 3 },
                ],
            },
            {
                name: "marks",
                table: marks,
                rows: [
                    { id: "a", count: 5 },
                    { id: "b", count: 4 },
                ],
            },
        ],
        3,
    );
    expect(merged).toEqual([
        { name: "marks", row: { id: "a", count: 5 } },
        { name: "samples", row: { id: 1, count: 5 } },
        { name: "marks", row: { id: "b", count: 4 } },
    ]);
});
