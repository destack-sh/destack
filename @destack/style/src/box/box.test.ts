import { expect, test } from "@destack/test";
import { BoxSides, Corners } from "./index.ts";

test("write corners and sides clockwise from the top, as CSS does", () => {
    const corners = { ...Corners.all("8px"), bottomLeft: "0" };
    const sides = { ...BoxSides.all("1rem"), left: "2rem" };

    expect([Corners.css(corners), BoxSides.css(sides)]).toEqual([
        "8px 8px 8px 0",
        "1rem 1rem 1rem 2rem",
    ]);
});
