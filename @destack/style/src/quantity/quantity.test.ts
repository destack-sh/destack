import { expect, test } from "@destack/test";
import { Dimension } from "../index.ts";
import { Quantity } from "./index.ts";

/** Read an expression as its number and unit, none for what fails. */
function read(text: string): readonly [number, string | undefined] | undefined {
    const quantity = Quantity.parse(text);

    return quantity === undefined ? undefined : [quantity.value, quantity.unit];
}

test("read what a person types: numbers, units, operators by precedence and parentheses", () => {
    const texts = [
        "12",
        "12px",
        "-1.5REM",
        "50%",
        ".5em",
        "1e2",
        "2em",
        "8 + 4 * 2",
        "(8 + 4) * 2px",
        "100 / 3",
        "--4",
        "10px - 4",
        "12px + 1rem",
        "12 *",
        "(4",
        "auto",
        "",
        "1 / 0",
    ];

    expect(texts.map((text) => read(text))).toEqual([
        [12, undefined],
        [12, "px"],
        [-1.5, "rem"],
        [50, "%"],
        [0.5, "em"],
        [100, undefined],
        [2, "em"],
        [16, undefined],
        [24, "px"],
        [100 / 3, undefined],
        [4, undefined],
        [6, "px"],
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
        undefined,
    ]);
});

test("read a length in a field's unit, refusing units no length has", () => {
    expect(
        ["12", "12px * 2", "50%", "3fr", "1.5"].map((text) => Dimension.read(text, "rem")),
    ).toEqual([
        { value: 12, unit: "rem" },
        { value: 24, unit: "px" },
        { value: 50, unit: "%" },
        undefined,
        { value: 1.5, unit: "rem" },
    ]);
});
