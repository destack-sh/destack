import { expect, test } from "vitest";
import { schema, toJsonSchema } from "@destack/schema";
import { Plan, type Compatibility } from "./plan.ts";
import { PlanError } from "../error/error.ts";

/** Plan a change between two schemas, reading a refusal as its message. */
function plan(
    before: schema.Schema,
    after: schema.Schema,
    compatibility: Compatibility,
    isConverted = false,
) {
    try {
        return Plan.values({
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
        [{ action: "update", risk: "safe", target: "value", detail: "wider values" }],
        "value: declare a conversion for 2026.10.0",
        [
            {
                action: "convert",
                risk: "data-dependent",
                target: "value",
                detail: "convert values to 2026.10.0",
            },
        ],
        [
            {
                action: "convert",
                risk: "data-dependent",
                target: "value",
                detail: "convert values to 2026.10.0",
            },
        ],
        [{ action: "update", risk: "safe", target: "value", detail: "narrower values" }],
        [
            {
                action: "update",
                risk: "backward-incompatible",
                target: "value",
                detail: "wider values: earlier readers keep their release",
            },
        ],
        [
            {
                action: "update",
                risk: "backward-incompatible",
                target: "value",
                detail: "incompatible values: earlier readers keep their release",
            },
        ],
    ]);
});

test("join plans in order, collecting every refusal into one error", () => {
    const step = (target: string) =>
        ({ action: "create", target, risk: "safe", detail: "add" }) as const;
    const refuse = (target: string) => () => {
        throw new PlanError([{ target, detail: "declare a conversion for 2026.10.0" }]);
    };

    // keep the steps in order, then report both refusals at once
    expect(Plan.join([() => ({ steps: [step("a")] }), () => ({ steps: [step("b")] })])).toEqual({
        steps: [step("a"), step("b")],
    });
    expect(() => Plan.join([refuse("a"), () => ({ steps: [step("b")] }), refuse("c")])).toThrow(
        "a: declare a conversion for 2026.10.0; c: declare a conversion for 2026.10.0",
    );
});
