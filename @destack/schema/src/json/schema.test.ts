import { expect, test } from "@destack/test";
import { fromJsonSchema, toJsonSchema } from "./schema.ts";
import * as schema from "../validate/index.ts";

/** The seed of the generated schemas, fixed so a failing one repeats. */
const GENERATOR_SEED = 20_261_012;

/** The schemas generated per run: enough to compose every builder at every depth many times. */
const GENERATED_SCHEMAS = 500;

/** The deepest nesting of generated composites. */
const GENERATED_DEPTH = 3;

/** A tree node, recursive through its children. */
type Node = { readonly name: string; readonly children: readonly Node[] };

/** A schema with every declarable builder, so its description has each keyword the import reads. */
function declared() {
    // a recursive tree and an expression union told apart by kind
    const node: schema.Schema<Node> = schema.object({
        name: schema.string().min(1),
        children: schema.array(schema.lazy(() => node)),
    });
    const operand = schema.discriminatedUnion("kind", [
        schema.object({ kind: schema.literal("column"), name: schema.string() }),
        schema.object({ kind: schema.enum(["add", "subtract"]), by: schema.int() }),
    ]);

    return schema.object({
        // strings, each format and its pattern
        title: schema.string().min(1).max(80).meta({ title: "Title" }),
        slug: schema.string().regex(/^[a-z-]+$/u),
        handle: schema
            .string()
            .regex(/^[a-z]/u)
            .regex(/[a-z]$/u),
        code: schema.string().length(3),
        link: schema.url(),
        mail: schema.email(),
        key: schema.uuidv7(),
        id: schema.guid(),
        payload: schema.base64(),
        day: schema.iso.date(),
        at: schema.iso.datetime(),
        digest: schema.hash("sha256"),
        mark: schema.emoji(),

        // numbers and their bounds
        count: schema.int().nonnegative(),
        rank: schema.int().positive().max(10),
        ratio: schema.number().gt(0).lt(1).multipleOf(0.25),

        // literals, enums and nullable values
        fixed: schema.literal("on"),
        level: schema.literal([1, 2, 3]),
        status: schema.enum(["open", "closed"]),
        note: schema.string().nullable(),
        mixed: schema.union([schema.string(), schema.number(), schema.null()]),
        anything: schema.json(),

        // lists, tuples, records and optional properties
        tags: schema.array(schema.string()).min(1).max(5).readonly(),
        pair: schema.tuple([schema.string(), schema.int()]),
        labels: schema.record(schema.string().regex(/^[a-z]+$/u), schema.string()),
        flags: schema.record(schema.enum(["a", "b"]), schema.boolean()),
        digests: schema.partialRecord(schema.enum(["md5", "sha1"]), schema.string()),
        sources: schema.record(
            schema.union([schema.enum(["home"]), schema.identifier("package")]),
            schema.boolean(),
        ),
        rest: schema.tuple([schema.string()], schema.int()),
        nothing: schema.tuple([]),
        option: schema.string().exactOptional(),

        // recursion and unions of objects
        tree: node,
        operands: schema.array(operand),
        either: schema.union([
            schema.object({ left: schema.string() }),
            schema.object({ right: schema.int() }),
        ]),
    });
}

test("describe a schema built from its own description alike, for every declarable builder", () => {
    // build the schema from its description and describe it again
    const described = toJsonSchema(declared());

    expect(toJsonSchema(fromJsonSchema(described))).toEqual(described);
});

test("describe random compositions of declarable builders alike once built from their descriptions", () => {
    // generate schemas from a fixed seed, so a failure repeats
    const random = seeded(GENERATOR_SEED);
    const mismatched = Array.from({ length: GENERATED_SCHEMAS }, () => generate(random, 0))
        .map((generated) => toJsonSchema(generated))
        .filter((described) => {
            const again = toJsonSchema(fromJsonSchema(described));

            return JSON.stringify(again) !== JSON.stringify(described);
        });

    expect(mismatched).toEqual([]);
});

test("validate as the declared schema does: formats, bounds, recursion and unions", () => {
    // parse values the declared schema accepts and refuses with the built schema
    const declaredSchema = schema.object({
        key: schema.uuidv7(),
        ratio: schema.number().gt(0).lt(1).multipleOf(0.25),
        tree: declared().shape.tree,
        operands: declared().shape.operands,
    });
    const built = fromJsonSchema(toJsonSchema(declaredSchema));
    const accepted = {
        key: "01890a5d-ac96-774b-bcce-b302099a8057",
        ratio: 0.5,
        tree: { name: "a", children: [{ name: "b", children: [] }] },
        operands: [
            { kind: "add", by: 2 },
            { kind: "column", name: "x" },
        ],
    };
    const values = [
        accepted,
        { ...accepted, key: "6f1c2b9e-1d2a-4c3b-9e8f-0a1b2c3d4e5f" },
        { ...accepted, ratio: 0.3 },
        { ...accepted, tree: { name: "a", children: [{ name: "", children: [] }] } },
        { ...accepted, operands: [{ kind: "multiply", by: 2 }] },
        { ...accepted, other: true },
    ];

    expect(values.map((value) => built.safeParse(value).success)).toEqual(
        values.map((value) => declaredSchema.safeParse(value).success),
    );
});

test("refuse a reference that does not resolve, naming its pointer", () => {
    expect(() => fromJsonSchema({ $ref: "#/$defs/missing" })).toThrow(
        new TypeError("schema reference #/$defs/missing does not resolve"),
    );
});

test("refuse keywords and shapes outside the declarable subset, naming them", () => {
    expect(() => fromJsonSchema({ type: "string", not: { type: "number" } })).toThrow(
        new TypeError("json schema not of a schema other than any value is not declarable"),
    );
    expect(() =>
        fromJsonSchema({ type: "array", items: { type: "string" }, uniqueItems: true }),
    ).toThrow(new TypeError("json schema keywords uniqueItems are not declarable"));
    expect(() =>
        fromJsonSchema({ type: "object", properties: { title: { type: "string" } } }),
    ).toThrow(
        new TypeError(
            'json schema object {"type":"object","properties":{"title":{"type":"string"}}} is not declarable',
        ),
    );
    expect(() => fromJsonSchema({ type: "string", default: "lunch" })).toThrow(
        new TypeError("json schema keywords default are not declarable"),
    );
    expect(() => fromJsonSchema({ enum: ["low", "high"] })).toThrow(
        new TypeError('json schema {"enum":["low","high"]} has no type'),
    );
    expect(() => fromJsonSchema({ type: "string", format: "uuid", pattern: "^[0-9]+$" })).toThrow(
        new TypeError("json schema format uuid with patterns ^[0-9]+$ is not declarable"),
    );
});

test("match patterns from descriptions in linear time, refusing lookarounds", () => {
    // match a pattern that backtracks exponentially in JavaScript's own engine, which would hang here
    const built = fromJsonSchema({ type: "string", pattern: "^(a+)+$" });
    expect([
        built.safeParse("a".repeat(40)).success,
        built.safeParse(`${"a".repeat(40)}b`).success,
    ]).toEqual([true, false]);

    // refuse a lookaround, which no linear engine matches
    expect(() => fromJsonSchema({ type: "string", pattern: "^(?!xn$)" })).toThrow(
        new TypeError("json schema pattern ^(?!xn$) is not declarable"),
    );
});

/** Draw numbers in [0, 1) from a seed, as a 32-bit linear congruential generator does. */
function seeded(seed: number): () => number {
    let state = seed >>> 0;

    return () => {
        state = (Math.imul(state, 1_664_525) + 1_013_904_223) >>> 0;

        return state / 2 ** 32;
    };
}

/** Pick one of some choices with a random draw. */
function pick<Value>(random: () => number, choices: readonly Value[]): Value {
    const chosen = choices[Math.floor(random() * choices.length)];
    if (chosen === undefined) {
        throw new RangeError("no choice to pick from");
    }

    return chosen;
}

/** Generate a declarable schema, composites above leaves until the depth runs out. */
function generate(random: () => number, depth: number): schema.Schema {
    // build a leaf at the deepest level, or by chance above it
    const isLeaf = depth >= GENERATED_DEPTH || random() < 0.35;
    if (isLeaf) {
        return pick(random, [
            () => schema.string(),
            () => schema.string().min(1).max(40),
            () => schema.string().regex(/^[a-z]+$/u),
            () => schema.email(),
            () => schema.url(),
            () => schema.int().min(0).max(100),
            () => schema.number(),
            () => schema.boolean(),
            () => schema.literal("fixed"),
            () => schema.enum(["first", "second", "third"]),
            () => schema.null(),
        ])();
    }

    // compose the next level's schemas
    const inner = () => generate(random, depth + 1);
    const composite = pick(random, [
        () =>
            schema.object(
                Object.fromEntries(
                    ["alpha", "beta", "gamma"]
                        .slice(0, 1 + Math.floor(random() * 3))
                        .map((key) => [key, random() < 0.3 ? inner().exactOptional() : inner()]),
                ),
            ),
        () => schema.array(inner()),
        () => schema.tuple([inner(), inner()]),
        () => schema.tuple([inner()], inner()),
        () => schema.record(schema.string(), inner()),
        () => schema.record(schema.enum(["north", "south"]), inner()),
        () => schema.partialRecord(schema.enum(["east", "west"]), inner()),
        () => schema.union([inner(), inner()]),
        () =>
            schema.discriminatedUnion("kind", [
                schema.object({ kind: schema.literal("one"), value: inner() }),
                schema.object({ kind: schema.literal("two"), other: inner() }),
            ]),
        () => inner().nullable(),
    ]);

    return composite();
}

test("describe a schema once, answering every caller the same frozen description", () => {
    // describe one schema twice
    const note = schema.object({ title: schema.string().min(1) });
    const first = toJsonSchema(note);
    const second = toJsonSchema(note);

    // answer one description, frozen at every depth, so no caller changes another's
    expect([second === first, Object.isFrozen(first), Object.isFrozen(first.properties)]).toEqual([
        true,
        true,
        true,
    ]);
    expect(() => Object.assign(first, { title: "changed" })).toThrow(TypeError);
});
