import { expect, test } from "vitest";
import { schema } from "../index.ts";
import { compareJsonSchemas, type SchemaComparison } from "./compare.ts";
import { toJsonSchema } from "./schema.ts";

/** A tree node for recursive schemas. */
type Node = { readonly name: string; readonly children?: readonly Node[] };

/** A recursive tree schema. */
const Node: schema.Schema<Node> = schema.lazy(() =>
    schema.object({ name: schema.string(), children: schema.array(Node).optional() }).strict(),
);

/** A recursive tree schema with numbered nodes. */
const NumberedNode: schema.Schema<Node> = schema.lazy(() =>
    schema
        .object({
            name: schema.string().regex(/^\d+$/),
            children: schema.array(NumberedNode).optional(),
        })
        .strict(),
);

/** Schema pairs by their expected change, before then after. */
const CHANGES: readonly (readonly [string, schema.Schema, schema.Schema, SchemaComparison])[] = [
    // strings
    ["keep a string", schema.string(), schema.string(), "same"],
    ["drop a minimum length", schema.string().min(1), schema.string(), "wider"],
    ["add a maximum length", schema.string(), schema.string().max(5), "narrower"],
    ["drop an email format", schema.string().email(), schema.string(), "wider"],
    ["swap a pattern", schema.string().regex(/^a/), schema.string().regex(/^b/), "incompatible"],

    // enums and literals
    ["add an enum value", schema.enum(["a", "b"]), schema.enum(["a", "b", "c"]), "wider"],
    ["swap enum values", schema.enum(["a", "b"]), schema.enum(["b", "c"]), "incompatible"],
    ["widen a literal to a string", schema.literal("a"), schema.string(), "wider"],
    ["narrow a string to an enum", schema.string(), schema.enum(["a"]), "narrower"],

    // numbers
    ["widen integers to numbers", schema.number().int(), schema.number(), "wider"],
    ["include a lower bound", schema.number().gt(0), schema.number().gte(0), "wider"],
    ["halve a step", schema.number().multipleOf(4), schema.number().multipleOf(2), "wider"],
    ["change a number to a string", schema.number(), schema.string(), "incompatible"],

    // unions and nullability
    [
        "spell nullable as a union",
        schema.string().nullable(),
        schema.union([schema.string(), schema.null()]),
        "same",
    ],
    [
        "add a union member",
        schema.string(),
        schema.union([schema.string(), schema.number()]),
        "wider",
    ],
    ["drop null", schema.string().nullable(), schema.string(), "narrower"],

    // objects
    [
        "add an optional property",
        schema.object({ title: schema.string() }).strict(),
        schema.object({ title: schema.string(), size: schema.number().optional() }).strict(),
        "wider",
    ],
    [
        "require an optional property",
        schema.object({ title: schema.string(), size: schema.number().optional() }).strict(),
        schema.object({ title: schema.string(), size: schema.number() }).strict(),
        "narrower",
    ],
    [
        "add a required property",
        schema.object({ title: schema.string() }).strict(),
        schema.object({ title: schema.string(), size: schema.number() }).strict(),
        "incompatible",
    ],
    [
        "change a property type",
        schema.object({ title: schema.string() }).strict(),
        schema.object({ title: schema.number() }).strict(),
        "incompatible",
    ],
    [
        "add a union variant",
        schema.discriminatedUnion("kind", [schema.object({ kind: schema.literal("a") }).strict()]),
        schema.discriminatedUnion("kind", [
            schema.object({ kind: schema.literal("a") }).strict(),
            schema.object({ kind: schema.literal("b"), size: schema.number() }).strict(),
        ]),
        "wider",
    ],

    // records, arrays and tuples
    [
        "widen record values to JSON",
        schema.record(schema.string(), schema.boolean()),
        schema.record(schema.string(), schema.json()),
        "wider",
    ],
    [
        "drop a maximum count",
        schema.array(schema.string()).max(3),
        schema.array(schema.string()),
        "wider",
    ],
    [
        "widen a tuple to an array",
        schema.tuple([schema.string()]),
        schema.array(schema.string()),
        "wider",
    ],
    [
        "add a tuple rest",
        schema.tuple([schema.string()]),
        schema.tuple([schema.string()], schema.number()),
        "wider",
    ],

    // recursion and JSON
    ["keep a recursive tree", Node, Node, "same"],
    ["widen a recursive tree", NumberedNode, Node, "wider"],
    ["narrow JSON to a string", schema.json(), schema.string(), "narrower"],
    ["keep JSON", schema.json(), schema.json(), "same"],
];

/** Sample values for the soundness sweep. */
const VALUES: readonly unknown[] = [
    null,
    true,
    0,
    -1,
    1.5,
    2,
    4,
    "",
    "a",
    "b",
    "c",
    "1",
    "abcdef",
    "user@example.com",
    [],
    ["a"],
    ["a", 1],
    ["a", "b", "c", "d"],
    {},
    { title: "a" },
    { title: 1 },
    { title: "a", size: 1 },
    { kind: "a" },
    { kind: "b", size: 1 },
    { flag: true },
    { flag: [1] },
    { name: "1" },
    { name: "a", children: [{ name: "b" }] },
    { name: "1", children: [{ name: "2" }] },
];

test("classify schema changes by the values each side accepts", () => {
    const changes = CHANGES.map(([name, before, after]) => [
        name,
        compareJsonSchemas(toJsonSchema(before), toJsonSchema(after)),
    ]);

    expect(changes).toEqual(CHANGES.map(([name, , , change]) => [name, change]));
});

test("claim an inclusion only when every sample value the included schema accepts stays accepted", () => {
    // gather every schema of the table
    const schemas = CHANGES.flatMap(([, before, after]) => [before, after]);

    // collect the pairs whose claimed inclusion rejects a sample value
    const violations = [];
    for (const [beforeIndex, before] of schemas.entries()) {
        for (const [afterIndex, after] of schemas.entries()) {
            const change = compareJsonSchemas(toJsonSchema(before), toJsonSchema(after));
            const isWider = change === "same" || change === "wider";
            const isNarrower = change === "same" || change === "narrower";
            for (const value of VALUES) {
                const isBefore = before.safeParse(value).success;
                const isAfter = after.safeParse(value).success;
                if ((isWider && isBefore && !isAfter) || (isNarrower && isAfter && !isBefore)) {
                    violations.push({ beforeIndex, afterIndex, change, value });
                }
            }
        }
    }

    expect(violations).toEqual([]);
});
