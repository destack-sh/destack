import { expect, test } from "@destack/test";
import { Transform } from "./index.ts";

test("move, turn and scale a box as CSS's individual transform properties", () => {
    const transform = {
        x: { value: 10, unit: "px" },
        y: { value: -50, unit: "%" },
        rotate: 15,
        scaleX: 1,
        scaleY: 2,
    } as const;

    expect(Transform.css(transform)).toEqual({
        translate: "10px -50%",
        rotate: "15deg",
        scale: "1 2",
    });
});
