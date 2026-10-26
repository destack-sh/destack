import { asc, Placeholder, sql } from "../sql/index.ts";
import { expect, onTestFinished, test, refusal } from "@destack/test";
import { TestDatabase } from "../test/database.ts";
import { defineTable, Table } from "../table/table.ts";
import { bigint, boolean, integer, real, text } from "../table/column.ts";
import { Condition, type Comparable, type Scalar } from "./condition.ts";
import { Predicate } from "./predicate.ts";
import { Namespace } from "./namespace.ts";
import { Order } from "./order.ts";
import { testSimulation } from "@destack/test/world";

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
    name: ["a", "B", "b", "é", "z", "😀", "￿", "", "a%", "a_", "a\\b", null],
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

/** The text patterns matched against names: wildcards, case, accents and astral characters. */
const PATTERNS = [
    "a%",
    "%b",
    "_",
    "B",
    "%é%",
    "%😀",
    "%",
    "",
    "[a]%",
    "*",
    "a\\%",
    "a\\_",
    "a\\\\b",
];

/** The literal ends of names drawn against them, wildcards and escapes included. */
const ENDS = ["a", "b", "%", "_", "\\", "é", "😀", "a%"];

/** The character sets drawn against names. */
const CHARACTER_SETS = ["a-z", "a-zA-Z", "0-9a-f", "a-z._-", "-a", "Z-a"];

/** The numeric fields compared with each other. */
const NUMBERS = ["count", "ratio", "editedAt"] as const;

/** The pattern operators drawn against names. */
const PATTERN_OPERATORS = ["like", "ilike", "notLike", "notIlike"] as const;

/** Build a random nested condition. */
function draw(seed: { value: number }, depth: number): Condition {
    // pick a field and its values, and a leaf at the bottom
    const [name, listed] = pick(Object.entries(VALUES), seed);
    const values = listed.filter((value): value is Comparable => value !== null);
    const choice = Math.floor(random(seed) * (depth === 0 ? 10 : 15));

    // compare with a value
    if (choice === 0) {
        const operator = pick(["eq", "ne", "lt", "lte", "gt", "gte"] as const, seed);

        return { [name]: { [operator]: pick(values, seed) } };
    }
    // list some values
    else if (choice === 1 || choice === 2) {
        const chosen = values.filter(() => random(seed) < 0.5);

        return { [name]: choice === 1 ? { in: chosen } : { notIn: chosen } };
    }
    // test for null
    else if (choice === 3) {
        return { [name]: { isNull: true } };
    }
    // test for a value
    else if (choice === 4) {
        return { [name]: { isNotNull: true } };
    }
    // match a pattern
    else if (choice === 5) {
        return { name: { [pick(PATTERN_OPERATORS, seed)]: pick(PATTERNS, seed) } };
    }
    // compare with another field
    else if (choice === 6) {
        const operator = pick(["eq", "ne", "lt", "lte", "gt", "gte"] as const, seed);

        return { [pick(NUMBERS, seed)]: { [operator]: { field: pick(NUMBERS, seed) } } };
    }
    // compare a length
    else if (choice === 7) {
        const operator = pick(["eq", "gt", "lte"] as const, seed);

        return { name: { length: { [operator]: Math.floor(random(seed) * 3) } } };
    }
    // match a start or an end
    else if (choice === 8) {
        return { name: { [pick(["startsWith", "endsWith"] as const, seed)]: pick(ENDS, seed) } };
    }
    // match a character set
    else if (choice === 9) {
        return { name: { characters: pick(CHARACTER_SETS, seed) } };
    }
    // combine operators on one field
    else if (choice === 10) {
        const operator = pick(["eq", "gte"] as const, seed);

        return { [name]: { OR: [{ isNull: true }, { NOT: { [operator]: pick(values, seed) } }] } };
    } else if (choice === 11) {
        return { [name]: { AND: [{ isNotNull: true }, { ne: pick(values, seed) }] } };
    }
    // combine conditions
    else if (choice === 12) {
        return { AND: [draw(seed, depth - 1), draw(seed, depth - 1)] };
    } else if (choice === 13) {
        return { OR: [draw(seed, depth - 1), draw(seed, depth - 1)] };
    }
    // negate a condition
    else {
        return { NOT: draw(seed, depth - 1) };
    }
}

test("decide random conditions alike in SQL and in memory", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create([sample], {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;

    // insert every combination of a few values, read from their JSON form
    const seed = { value: 7 };
    const rows = Array.from({ length: 60 }, (_, id) =>
        Table.decode(
            sample,
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
        const predicate = Condition.resolve(drawn, Namespace.fields(sample));
        const selected = await database
            .select({ id: sample.id })
            .from(sample)
            .where(Predicate.render(predicate, Predicate.bind(sample)))
            .orderBy(asc(sample.id));
        const match = Predicate.compile(predicate, sample);
        const matched = rows.filter((row) => Predicate.matches(match, row));
        expect([drawn, selected.map((row) => row.id)]).toEqual([
            drawn,
            matched.map((row) => row["id"]),
        ]);
    }
});

test("escape a pattern's wildcards with a backslash, refusing an escape ending it", async () => {
    // match literal wildcards and backslashes only where escaped
    const matching = (pattern: string) =>
        ["a%", "ab", "a_", "a\\b"].filter((name) =>
            Predicate.matches(
                Predicate.compile(
                    Condition.resolve({ name: { like: pattern } }, Namespace.fields(sample)),
                    sample,
                ),
                { name },
            ),
        );
    expect([matching("a\\%"), matching("a%"), matching("a\\_"), matching("a\\\\b")]).toEqual([
        ["a%"],
        ["a%", "ab", "a_", "a\\b"],
        ["a_"],
        ["a\\b"],
    ]);
    expect(await refusal(() => matching("a\\"))).toEqual([
        "BAD_REQUEST",
        "QUERY_INVALID",
        "a like pattern cannot end with its escape character",
    ]);
});

test("order text by code point, as UTF-8 bytes order it", async () => {
    const texts = ["😀", "￿", "é", "z", "b", "B", "a", ""];
    const byByte = texts.toSorted((left, right) =>
        Buffer.compare(UTF8.encode(left), UTF8.encode(right)),
    );
    expect(texts.toSorted((left, right) => Order.codePoints(left, right))).toEqual(byByte);
});

test("refuse a descending character range, reading a mixed one as its characters", async () => {
    // refuse z-a, and read Z-a as Z and a with a hyphen
    const fields = Namespace.fields(sample);
    expect(await refusal(() => Condition.resolve({ name: { characters: "z-a" } }, fields))).toEqual(
        ["BAD_REQUEST", "QUERY_INVALID", "character range z-a is descending"],
    );
    const set = Predicate.compile(
        Condition.resolve({ name: { characters: "Z-a" } }, fields),
        sample,
    );
    expect([
        Predicate.matches(set, { name: "Za-" }),
        Predicate.matches(set, { name: "_" }),
    ]).toEqual([true, false]);
});

test("refuse comparing fields that order as different values, and a field the table lacks", async () => {
    // compare text with a number, and a number with a missing field
    const require = (condition: Condition) => () =>
        Predicate.require(Condition.resolve(condition, Namespace.fields(sample)), sample);
    expect(await refusal(require({ name: { gt: { field: "count" } } }))).toEqual([
        "BAD_REQUEST",
        "QUERY_INVALID",
        "cannot compare condition_sample.name (text ordering) with count (number ordering)",
    ]);
    expect(await refusal(require({ count: { gt: { field: "missing" } } }))).toEqual([
        "BAD_REQUEST",
        "QUERY_INVALID",
        "condition_sample has no column missing",
    ]);
});

test("read a placeholder's or a field's JSON form as its operand, and refuse unknown or mistyped comparisons", async () => {
    // read the JSON forms the wire sends
    const fields = Namespace.fields(sample);
    expect([
        Condition.resolve({ name: { placeholder: "query" } }, fields),
        Condition.resolve({ name: { ne: { placeholder: "query" } } }, fields),
        Condition.resolve({ count: { field: "ratio" } }, fields),
        Condition.resolve({ count: { gt: { field: "ratio" } } }, fields),
    ]).toEqual([
        Condition.resolve({ name: new Placeholder("query") }, fields),
        Condition.resolve({ name: { ne: new Placeholder("query") } }, fields),
        Predicate.all({
            kind: "compare",
            operator: "eq",
            name: "count",
            value: { field: "ratio" },
        }),
        Predicate.all({
            kind: "compare",
            operator: "gt",
            name: "count",
            value: { field: "ratio" },
        }),
    ]);

    // refuse an unknown operator, a mistyped operand, a negative length, an empty placeholder name and a mistyped negation
    const refused = (condition: Condition) => refusal(() => Condition.resolve(condition, fields));
    expect([
        await refused({ name: { contains: "a" } }),
        await refused({ name: { like: 1 } }),
        await refused({ name: { length: -1 } }),
        await refused({ name: { eq: { placeholder: "" } } }),
        await refused({ name: { NOT: { isNull: false } } }),
    ]).toEqual([
        invalidField("name"),
        invalidField("name"),
        invalidField("name"),
        invalidField("name"),
        invalidField("name"),
    ]);
});

test("refuse conditions beyond a thousand terms", () => {
    // accept a thousand values and refuse one more
    const values = Array.from({ length: 1000 }, (_, index) => index);
    const fields = Namespace.fields(sample);
    Condition.resolve({ count: { in: values } }, fields);
    expect(() => Condition.resolve({ count: { in: values.slice(1) }, name: "a" }, fields)).toThrow(
        "condition has 1001 terms, more than 1000",
    );
});

test("match a thousand listed values in SQL", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create([sample], {
        environment: world,
        isMigrated: true,
    });
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
        .where(
            Predicate.render(
                Condition.resolve({ count: { in: values } }, Namespace.fields(sample)),
                Predicate.bind(sample),
            ),
        );
    expect(rows.map((row) => row.id)).toEqual([1]);
});

test("decide relations through the binding, unknown until the relation is read", () => {
    // decide each relation answer
    const predicate = Condition.resolve(
        { OR: [{ NOT: { marks: { name: "a" } } }, { count: 1 }] },
        Namespace.fields(sample),
    );
    const match = Predicate.compile(predicate, sample);
    const decide = (count: number, exists: boolean | undefined) =>
        match({
            field: () => count,
            placeholder: () => null,
            exists: () => exists,
            search: () => undefined,
        });
    expect([decide(1, undefined), decide(0, undefined), decide(0, true), decide(0, false)]).toEqual(
        [true, undefined, false, true],
    );

    // refuse undeclared relations
    expect(() => Predicate.render(predicate, Predicate.bind(sample))).toThrow(
        "condition follows relation marks, but condition_sample does not declare it",
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

test("resolve conditions by their row's fields, and read their fields, relations and size", () => {
    const condition: Condition = {
        status: "open",
        due: { isNotNull: true },
        priority: { in: ["high", "low"] },
        assignee: { level: { gt: sql.placeholder("level") } },
        labels: true,
    };
    const predicate = Condition.resolve(condition, new Set(["status", "due", "priority"]));

    // resolve fields to comparisons and any other name to a relation
    expect(predicate).toEqual({
        kind: "all",
        predicates: [
            { kind: "compare", operator: "eq", name: "status", value: "open" },
            { kind: "not", predicate: { kind: "missing", name: "due" } },
            { kind: "oneOf", name: "priority", values: ["high", "low"] },
            {
                kind: "exists",
                via: "assignee",
                where: { level: { gt: sql.placeholder("level") } },
            },
            { kind: "exists", via: "labels", where: {} },
        ],
    });

    // list what the predicate reads
    expect([
        [...Predicate.fields(predicate)],
        Predicate.relations(predicate).map((relation) => relation.via),
        Predicate.size(predicate),
    ]).toEqual([["status", "due", "priority"], ["assignee", "labels"], 8]);

    // round-trip through JSON and the schema to the same predicate, placeholders in their JSON form
    const parsed = Condition.schema.parse(JSON.parse(JSON.stringify(condition)));
    const resolved = Condition.resolve(parsed, new Set(["status", "due", "priority"]));
    expect(JSON.parse(JSON.stringify(resolved))).toEqual(JSON.parse(JSON.stringify(predicate)));
});

/** Read the refusal of a field's comparisons that are neither a value nor known comparisons. */
function invalidField(name: string): readonly string[] {
    return ["BAD_REQUEST", "QUERY_INVALID", `condition field ${name} needs a value or comparisons`];
}
