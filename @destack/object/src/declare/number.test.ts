import { t } from "@destack/locale";
import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { field } from "../field/index.ts";
import { ActionInput } from "./input.ts";

/** Read what inputs take, and back from their descriptions. */
function read(inputs: Readonly<Record<string, Parameters<typeof ActionInput.of>[1]>>) {
    return Object.entries(inputs).map(([name, each]) => {
        const input = ActionInput.of(name, each);
        const back = ActionInput.from(input.describe(), () => {
            throw new TypeError("no input takes objects");
        });

        return [name, input.value, back.value];
    });
}

test("read the limits and the format of numbers", () => {
    const share = { minimum: 0, maximum: 1, format: { style: "percent" } } as const;
    const stars = { minimum: 0, maximum: 5, step: 1 };

    expect(
        read({
            any: field.number(),
            whole: field.integer(),
            stars: field.integer(schema.number().int().min(0).max(5)),
            share: field.number(schema.number().min(0).max(1)).formatted({ style: "percent" }),
            half: schema.number().multipleOf(0.5).default(1),
        }),
    ).toEqual([
        ["any", { kind: "number" }, { kind: "number" }],
        ["whole", { kind: "number", step: 1 }, { kind: "number", step: 1 }],
        ["stars", { kind: "number", ...stars }, { kind: "number", ...stars }],
        ["share", { kind: "number", ...share }, { kind: "number", ...share }],
        ["half", { kind: "number", step: 0.5 }, { kind: "number", step: 0.5 }],
    ]);
});

test("take numbers within the limits", () => {
    const stars = ActionInput.of("stars", field.integer(schema.number().int().min(0).max(5)));

    expect([-1, 0, 2.5, 5, 6, "3", Number.NaN].map((value) => stars.takes(value))).toEqual([
        false,
        true,
        false,
        true,
        false,
        false,
        false,
    ]);
});

test("refuse a format of no number, and a schema of other numbers", () => {
    const failures = [
        () => field.string().formatted({ style: "percent" }),
        () => field.integer(schema.number()),
        () => field.number().formatted({ style: "unit", unit: "pixel" }),
    ].map((make) => {
        try {
            make();

            return "made";
        } catch (error) {
            return String(error);
        }
    });

    expect(failures).toEqual([
        "TypeError: only number fields have a number format",
        "TypeError: an integer field takes a schema of whole numbers",
        'TypeError: no language writes the format {"style":"unit","unit":"pixel"}',
    ]);
});

test("describe a number field by its schema and format", () => {
    const described = field
        .number(schema.number().min(0))
        .labeled(t`Price`)
        .formatted({ style: "currency", currency: "EUR" })
        .describe()
        .map(({ builder, arguments: given }) => [builder, given.length]);

    expect(described).toEqual([
        ["number", 1],
        ["labeled", 1],
        ["formatted", 1],
    ]);
});
