import { Color, Gradient } from "@destack/color";
import { JsonValue, schema, toJsonSchema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { field } from "../field/index.ts";
import { ActionInput } from "./input.ts";

/** Colors in order, at least two of them. */
const PAIR = schema.array(Color.schema).min(2);

/** A gradient from blue to gold. */
const BLUE_TO_GOLD = {
    stops: [
        { color: "#241d9a", position: 0 },
        { color: "#ffc53d", position: 1 },
    ],
    space: "oklab",
    hue: "shorter",
};

test("read a JSON field as JSON its schema takes, named such as a color's, and back from its description", () => {
    const inputs = {
        accent: field.json(Color.schema),
        pair: field.json(PAIR),
        ramp: field.json(Gradient.schema),
    };

    // each input carries its schema, its name for a color or a gradient, and reads the same back
    expect(
        Object.entries(inputs).map(([name, each]) => {
            const input = ActionInput.of(name, each);
            const back = ActionInput.from(input.describe(), () => {
                throw new TypeError("no input takes objects");
            });

            return [name, input.value, back.value];
        }),
    ).toEqual([
        ["accent", described(Color.schema), described(Color.schema)],
        ["pair", described(PAIR), described(PAIR)],
        ["ramp", described(Gradient.schema), described(Gradient.schema)],
    ]);
});

test("take only JSON its schema takes, even from a description", () => {
    const accent = ActionInput.of("accent", field.json(Color.schema));
    const pair = ActionInput.of("pair", field.json(PAIR));
    const ramp = ActionInput.of("ramp", field.json(Gradient.schema)).describe();
    const back = ActionInput.from(ramp, () => {
        throw new TypeError("no input takes objects");
    });

    expect({
        accent: ["#f76b15", "orange", 3].map((value) => accent.takes(value)),
        pair: [["#fff", "#000"], ["#fff"]].map((value) => pair.takes(value)),
        ramp: [BLUE_TO_GOLD, { ...BLUE_TO_GOLD, stops: [] }].map((value) => back.takes(value)),
    }).toEqual({
        accent: [true, false, false],
        pair: [true, false],
        ramp: [true, false],
    });
});

/** Describe what a JSON input of a schema takes. */
function described(definition: schema.Schema): {
    readonly kind: "json";
    readonly schema: JsonValue;
} {
    return { kind: "json", schema: JsonValue.object(toJsonSchema(definition)) };
}
