import { expect, test } from "@destack/test";
import { Transform } from "./index.ts";

test("move, turn and scale a box as CSS's individual transform properties", () => {
    expect(Transform.css({ x: "10px", y: "-50%", rotate: 15, scaleX: 1, scaleY: 2 })).toEqual({
        translate: "10px -50%",
        rotate: "15deg",
        scale: "1 2",
    });
});
