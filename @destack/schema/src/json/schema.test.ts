import { expect, test } from "@destack/test";
import { fromJsonSchema, toJsonSchema } from "./schema.ts";
import * as schema from "../validate/index.ts";

/** A tree node, recursive through its children. */
type Node = { readonly name: string; readonly children: readonly Node[] };

/** A schema using every declarable builder, so its description keeps every keyword the import reads. */
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
