import { expect, test } from "@destack/test";
import { Dimension } from "./index.ts";

test("write a dimension as CSS writes a length, refusing a unit CSS has no length in", () => {
    // write lengths and a percentage, then read a dimension in an unknown unit
    const dimensions = [
        { value: 12, unit: "px" },
        { value: -1.5, unit: "rem" },
        { value: 50, unit: "%" },
    ] as const;

    expect({
        css: dimensions.map((dimension) => Dimension.css(dimension)),
        refused: Dimension.safeParse({ value: 1, unit: "pt" }).error?.issues.map(
            (issue) => issue.path,
        ),
    }).toEqual({ css: ["12px", "-1.5rem", "50%"], refused: [["unit"]] });
});
