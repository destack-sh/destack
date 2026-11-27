import { expect, test } from "@destack/test";
import type { Dimension } from "../dimension/index.ts";
import { BoxSides, Corners } from "./index.ts";

/** A length in pixels. */
const px = (value: number): Dimension => ({ value, unit: "px" });

/** A length in the root's font size. */
const rem = (value: number): Dimension => ({ value, unit: "rem" });

test("write corners and sides clockwise from the top, as CSS does", () => {
    const corners = { ...Corners.all(px(8)), bottomLeft: px(0) };
    const sides = { ...BoxSides.all(rem(1)), left: rem(2) };

    expect([Corners.css(corners), BoxSides.css(sides)]).toEqual([
        "8px 8px 8px 0px",
        "1rem 1rem 1rem 2rem",
    ]);
});
