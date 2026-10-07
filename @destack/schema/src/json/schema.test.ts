import { expect, test } from "@destack/test";
import { fromJsonSchema } from "./schema.ts";

test("follow local references of any pointer, recursive ones included", () => {
    // a document keeping its schemas where an API description keeps them
    const tree = fromJsonSchema({
        type: "object",
        properties: { root: { $ref: "#/components/schemas/Node" } },
        required: ["root"],
        components: {
            schemas: {
                Node: {
                    type: "object",
                    properties: {
                        name: { type: "string", minLength: 1 },
                        children: { type: "array", items: { $ref: "#/components/schemas/Node" } },
                    },
                    required: ["name"],
                },
            },
        },
    });

    expect([
        tree.safeParse({ root: { name: "a", children: [{ name: "b" }] } }).success,
        tree.safeParse({ root: { name: "a", children: [{ name: "" }] } }).success,
    ]).toEqual([true, false]);
});

test("refuse a reference that does not resolve, naming its pointer", () => {
    expect(() => fromJsonSchema({ $ref: "#/definitions/missing" })).toThrow(
        new TypeError("schema reference #/definitions/missing does not resolve"),
    );
});
