import { expect, test } from "@destack/test";
import { ShadowStack } from "./index.ts";

test("write shadows as CSS's box-shadow, the first on top, and none for no shadow", () => {
    const drop = {
        color: "#00000033",
        offsetX: { value: 0, unit: "px" },
        offsetY: { value: 4, unit: "px" },
        blur: { value: 12, unit: "px" },
        spread: { value: 0, unit: "px" },
        inset: false,
    } as const;
    const inner = {
        ...drop,
        offsetY: { value: 1, unit: "px" },
        blur: { value: 0, unit: "px" },
        inset: true,
    } as const;

    expect([ShadowStack.css([drop, inner]), ShadowStack.css([])]).toEqual([
        "0px 4px 12px 0px #00000033, inset 0px 1px 0px 0px #00000033",
        "none",
    ]);
});
