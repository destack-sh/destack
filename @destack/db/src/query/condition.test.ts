import { asc } from "../sql/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { TEST_DIALECTS, TestDatabase } from "../test/database.ts";
import { defineTable, type Table } from "../table/table.ts";
import { bigint, boolean, integer, real, text } from "../table/column.ts";
import { TABLE } from "../table/table.ts";
import { Condition, type Scalar } from "./condition.ts";
import { Order } from "./order.ts";

/** The encoder of texts' UTF-8 bytes. */
const UTF8 = new TextEncoder();

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
    const picked = values[Math.floor(random(seed) * values.length)];
    if (picked === undefined) {
        throw new RangeError("pick from at least one value");
    }

    return picked;
}

/** Build a random nested condition. */
function draw(seed: { value: number }, depth: number): Condition {
    // compare, test or combine
    const [name, listed] = pick(Object.entries(VALUES), seed);
    const choice = depth === 0 ? Math.floor(random(seed) * 3) : Math.floor(random(seed) * 6);
    if (choice === 0) {
        const operator = pick(["eq", "ne", "lt", "lte", "gt", "gte"] as const, seed);

        return Condition.compare(operator, name, pick(listed, seed));
    } else if (choice === 1) {
        const values = listed.filter(
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
        const storage = await TestDatabase.create(dialect, [sample], { isMigrated: true });
        onTestFinished(() => storage.close());
        const database = storage.database;

        // insert every combination of a few values, read from their JSON form
        const seed = { value: 7 };
        const rows = Array.from({ length: 60 }, (_, id) =>
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
                        column: (name: string): unknown => row[name],
                        parameter: () => null,
                        exists: () => undefined,
                    }) === true,
            );
            expect([drawn, selected.map((row) => row.id)]).toEqual([
                drawn,
                matched.map((row) => row["id"]),
            ]);
        }
    },
);

test("order text by code point, as UTF-8 bytes order it", () => {
    const texts = ["😀", "￿", "é", "z", "b", "B", "a", ""];
    const byByte = texts.toSorted((left, right) =>
        Buffer.compare(UTF8.encode(left), UTF8.encode(right)),
    );
    expect(texts.toSorted((left, right) => Order.codePoints(left, right))).toEqual(byByte);
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
    ).toThrow("condition has 1001 terms, more than 1000");
});

test.for(TEST_DIALECTS)("match a thousand listed values in SQL on %s", async (dialect) => {
    const storage = await TestDatabase.create(dialect, [sample], { isMigrated: true });
    onTestFinished(() => storage.close());
    await storage.database.insert(sample).values([
        { id: 1, count: 999 },
        { id: 2, count: 1000 },
    ]);

    // render a thousand values as a balanced tree
    const values = Array.from({ length: 1000 }, (_, index) => index);
    const rows = await storage.database
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

test("build conditions, and read their columns, relations and terms", () => {
    const condition = Condition.all(
        Condition.eq("status", "open"),
        Condition.not(Condition.missing("due")),
        Condition.oneOf("priority", ["high", "low"]),
        Condition.exists("assignee", Condition.gt("level", Condition.parameter("level"))),
    );

    // list what the condition reads
    expect([
        [...Condition.columns(condition)],
        Condition.relations(condition).map((relation) => relation.via),
        Condition.terms(condition),
    ]).toEqual([["status", "due", "priority"], ["assignee"], 8]);

    // rename columns, leaving relations to their own rows
    expect(Condition.rename(condition, (column) => `t_${column}`)).toEqual(
        Condition.all(
            Condition.eq("t_status", "open"),
            Condition.not(Condition.missing("t_due")),
            Condition.oneOf("t_priority", ["high", "low"]),
            Condition.exists("assignee", Condition.gt("level", Condition.parameter("level"))),
        ),
    );

    // round-trip through the schema
    expect(Condition.schema.parse(JSON.parse(JSON.stringify(condition)))).toEqual(condition);
});
