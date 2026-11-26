import { expect, test } from "@destack/test";
import { field } from "../field/index.ts";
import { ActionInput } from "./input.ts";

test("read colors and lists of colors, and back from their descriptions", () => {
    const inputs = {
        accent: field.color(),
        stops: field.colors({ minimum: 2, maximum: 10 }),
        any: field.colors(),
    };

    // each input reads the same from its description
    expect(
        Object.entries(inputs).map(([name, each]) => {
            const input = ActionInput.of(name, each);
            const back = ActionInput.from(input.describe(), () => {
                throw new TypeError("no input takes objects");
            });

            return [name, input.value, back.value];
        }),
    ).toEqual([
        ["accent", { kind: "color" }, { kind: "color" }],
        [
            "stops",
            { kind: "colors", minimum: 2, maximum: 10 },
            { kind: "colors", minimum: 2, maximum: 10 },
        ],
        ["any", { kind: "colors" }, { kind: "colors" }],
    ]);
});

test("take colors CSS writes, as many as a list takes", () => {
    const accent = ActionInput.of("accent", field.color());
    const stops = ActionInput.of("stops", field.colors({ minimum: 2, maximum: 3 }));

    expect({
        accent: ["#f76b15", "oklch(69% 0.19 45)", "orange", "#12", 3].map((value) =>
            accent.takes(value),
        ),
        stops: [["#fff"], ["#fff", "#000"], ["#fff", "#000", "#f00", "#0f0"], ["#fff", "red"]].map(
            (value) => stops.takes(value),
        ),
    }).toEqual({
        accent: [true, true, false, false, false],
        stops: [false, true, false, false],
    });
});
