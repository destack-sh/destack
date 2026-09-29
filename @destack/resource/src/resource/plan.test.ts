import { expect, test } from "vitest";
import { schema, toJsonSchema } from "@destack/schema";
import { Plan, type Compatibility } from "./plan.ts";

/** Plan a change between two schemas, reading a refusal as its message. */
function plan(
    before: schema.Schema,
    after: schema.Schema,
    compatibility: Compatibility,
    isConverted = false,
) {
    try {
        return Plan.schema({
            target: "value",
            before: toJsonSchema(before),
            after: toJsonSchema(after),
            release: "2026.10.0",
            compatibility,
            isConverted,
        }).steps;
    } catch (error) {
        return (error as Error).message;
    }
}

test("plan schema changes by the readers each must serve", () => {
    const small = schema.enum(["a"]);
    const large = schema.enum(["a", "b"]);
    const other = schema.number();

    expect([
        plan(small, small, "backward"),
        plan(small, large, "backward"),
        plan(large, small, "backward"),
        plan(large, small, "backward", true),
        plan(small, other, "backward", true),
        plan(large, small, "forward"),
        plan(small, large, "forward"),
        plan(small, other, "forward"),
    ]).toEqual([
        [],
        [{ kind: "wider", risk: "safe", target: "value", detail: "wider value" }],
        "value: declare a conversion for 2026.10.0",
        [
            {
                kind: "convert",
                risk: "data-dependent",
                target: "value",
                detail: "convert value to 2026.10.0",
            },
        ],
        [
            {
                kind: "convert",
                risk: "data-dependent",
                target: "value",
                detail: "convert value to 2026.10.0",
            },
        ],
        [{ kind: "narrower", risk: "safe", target: "value", detail: "narrower value" }],
        [
            {
                kind: "wider",
                risk: "backward-incompatible",
                target: "value",
                detail: "wider value: earlier readers keep their release",
            },
        ],
        [
            {
                kind: "incompatible",
                risk: "backward-incompatible",
                target: "value",
                detail: "incompatible value: earlier readers keep their release",
            },
        ],
    ]);
});
