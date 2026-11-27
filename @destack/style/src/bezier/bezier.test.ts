import { expect, test } from "@destack/test";
import { CubicBezier, EASINGS } from "./index.ts";

/** A curve CSS names no keyword for. */
const CUSTOM: CubicBezier = [0.2, 0, 0, 1];

test("write a curve by its CSS keyword where CSS names it, else as cubic-bezier()", () => {
    expect({
        css: [EASINGS["ease-out"].curve, CUSTOM].map((curve) => CubicBezier.css(curve)),
        refused: CubicBezier.safeParse([1.5, 0, 0, 1]).error?.issues.map((issue) => issue.path),
    }).toEqual({ css: ["ease-out", "cubic-bezier(0.2, 0, 0, 1)"], refused: [[0]] });
});
