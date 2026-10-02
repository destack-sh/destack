import { asc } from "../sql/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { TEST_DIALECTS, TestDatabase } from "../test/database.ts";
import { defineTable } from "../table/table.ts";
import type { JsonValue } from "@destack/schema";
import { integer, json, real, text } from "../table/column.ts";
import { schema } from "@destack/schema";
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

/** Rows with a JSON document and a text mode. */
const documents = defineTable("expression_document", {
    /** The row's key. */
    id: integer("id").primaryKey(),
    /** A JSON document. */
    data: json("data", schema.json()),
    /** Text. */
    mode: text("mode"),
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
    const picked = values[Math.floor(random(seed) * values.length)];
    if (picked === undefined) {
        throw new RangeError("pick from at least one value");
    }

    return picked;
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
        const storage = await TestDatabase.create(dialect, [sample], { isMigrated: true });
        onTestFinished(() => storage.close());

        // insert random rows
        const seed = { value: 11 };
        const rows = Array.from({ length: 30 }, (_, id) => ({
            id,
            count: pick(VALUES.count, seed),
            ratio: pick(VALUES.ratio, seed),
            name: pick(VALUES.name, seed),
        }));
        await storage.database.insert(sample).values(rows);

        // compute each expression in SQL and memory alike
        for (let index = 0; index < EXPRESSIONS; index += 1) {
            const drawn = draw(seed, 3);
            Expression.require(drawn, sample);
            const selected = await storage.database
                .select({ value: Expression.render(drawn, sample) })
                .from(sample)
                .orderBy(asc(sample.id));
            const computed = rows.map((row) => Expression.evaluate(drawn, row));
            expect([drawn, selected.map((row) => row.value)]).toEqual([drawn, computed]);
        }
    },
);

test.for(TEST_DIALECTS)("take the first present text on %s", async (dialect) => {
    const storage = await TestDatabase.create(dialect, [sample], { isMigrated: true });
    onTestFinished(() => storage.close());
    await storage.database.insert(sample).values([
        { id: 1, name: "a" },
        { id: 2, name: null },
    ]);

    // fall back to a literal
    const expression = Expression.coalesce(Expression.column("name"), Expression.literal("none"));
    Expression.require(expression, sample);
    const selected = await storage.database
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

/** JSON documents covering objects, arrays, scalars, nesting and absence. */
const DOCUMENTS: readonly { readonly data: JsonValue; readonly mode: string | null }[] = [
    { data: { editor: { mode: "emacs", size: 12 }, tags: ["a"] }, mode: "emacs" },
    { data: { editor: { mode: "vim", size: 1.5 } }, mode: "vim" },
    { data: { editor: { mode: 3, size: "large" } }, mode: "standard" },
    { data: { editor: null }, mode: null },
    { data: ["editor"], mode: "é" },
    { data: "editor", mode: "" },
    { data: null, mode: "emacs" },
];

/** JSON expressions over the documents, each with its result per document. */
const JSON_EXPRESSIONS: readonly Expression[] = [
    Expression.path(Expression.column("data"), "editor", "mode"),
    Expression.scalar(Expression.path(Expression.column("data"), "editor", "mode"), "text"),
    Expression.scalar(Expression.path(Expression.column("data"), "editor", "size"), "real"),
    Expression.object({
        mode: Expression.case(
            Expression.scalar(Expression.path(Expression.column("data"), "editor", "mode"), "text"),
            [{ when: "emacs", then: Expression.literal("standard") }],
            Expression.scalar(Expression.path(Expression.column("data"), "editor", "mode"), "text"),
        ),
        size: Expression.path(Expression.column("data"), "editor", "size"),
        pinned: Expression.json(false),
        tags: Expression.json(["b", { nested: true }]),
    }),
    Expression.case(
        Expression.column("mode"),
        [
            { when: "emacs", then: Expression.literal("standard") },
            { when: "vim", then: Expression.column("mode") },
        ],
        Expression.literal("other"),
    ),
    Expression.coalesce(
        Expression.scalar(Expression.path(Expression.column("data"), "editor", "mode"), "text"),
        Expression.column("mode"),
    ),
    Expression.object({ document: Expression.column("data"), mode: Expression.column("mode") }),
];

test.for(TEST_DIALECTS)(
    "compute JSON expressions alike in SQL and in memory on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, [documents], { isMigrated: true });
        onTestFinished(() => storage.close());
        const rows = DOCUMENTS.map((document, id) => ({ id, ...document }));
        await storage.database.insert(documents).values(rows);

        // compare each expression's SQL results, JSON read back as values, with its memory results
        const read = (value: unknown): unknown => {
            if (dialect === "sqlite" && typeof value === "string") {
                const parsed: unknown = JSON.parse(value);

                return parsed;
            }

            return value;
        };
        for (const expression of JSON_EXPRESSIONS) {
            const isJson = Expression.kind(expression, documents) === "json";
            const selected = await storage.database
                .select({ value: Expression.render(expression, documents) })
                .from(documents)
                .orderBy(asc(documents.id));
            const values = selected.map((row) => (isJson ? read(row.value) : row.value));
            const computed = rows.map((row) => Expression.evaluate(expression, row));
            expect([expression, values]).toEqual([expression, computed]);
        }
    },
);

test("refuse JSON where numbers or text are required, and reads inside values that are not JSON", () => {
    const data = Expression.column("data");
    const mode = Expression.column("mode");
    const refusal = (expression: Expression) => {
        try {
            Expression.require(expression, documents);

            return undefined;
        } catch (error) {
            if (!(error instanceof Error)) {
                throw error;
            }

            return error.message;
        }
    };

    expect([
        refusal(Expression.path(data, "editor")),
        refusal(Expression.case(data, [{ when: "a", then: mode }], mode)),
        refusal(Expression.path(mode, "editor")),
        refusal(Expression.scalar(mode, "text")),
    ]).toEqual([
        "expression yields JSON; read a scalar of it",
        "expression compares JSON in a case; read a scalar of it",
        "expression reads a path of a value that is not JSON",
        "expression reads a scalar of a value that is not JSON",
    ]);
});

test("upgrade partial records through each later release, leaving fields computed from absent fields absent", () => {
    // rename title to name in 2026.9.0, and default a new limit in 2026.10.0
    const conversions = {
        "2026.9.0": { name: Expression.column("title") },
        "2026.10.0": {
            limit: Expression.coalesce(Expression.column("limit"), Expression.literal(50)),
        },
    };
    const convert = (record: Record<string, JsonValue>, from: string) =>
        Expression.upgrade(conversions, record, from, "2026.10.0");

    // convert a full record, a partial one, an explicit null, and a record of the latest release
    expect([
        convert({ title: "Plan" }, "2026.8.0"),
        convert({ id: "a" }, "2026.8.0"),
        convert({ title: null }, "2026.8.0"),
        convert({ name: "Plan", limit: 5 }, "2026.10.0"),
    ]).toEqual([
        { title: "Plan", name: "Plan", limit: 50 },
        { id: "a", limit: 50 },
        { title: null, name: null, limit: 50 },
        { name: "Plan", limit: 5 },
    ]);
});
