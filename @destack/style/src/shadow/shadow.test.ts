import { expect, test } from "@destack/test";
import { Blur, ShadowStack } from "./index.ts";

test("write shadows as CSS's box-shadow, the first on top, and none for no shadow", () => {
    const drop = {
        color: "#00000033",
        x: "0",
        y: "4px",
        blur: "12px",
        spread: "0",
        isInset: false,
    };
    const inner = { ...drop, y: "1px", blur: "0", isInset: true };

    expect([ShadowStack.css([drop, inner]), ShadowStack.css([])]).toEqual([
        "0 4px 12px 0 #00000033, inset 0 1px 0 0 #00000033",
        "none",
    ]);
});

test("draw a blur of the box as its filter, and of what lies behind as its backdrop filter", () => {
    expect([
        Blur.css({ kind: "layer", radius: "4px" }),
        Blur.css({ kind: "backdrop", radius: "20px" }),
    ]).toEqual([{ filter: "blur(4px)" }, { backdropFilter: "blur(20px)" }]);
});
