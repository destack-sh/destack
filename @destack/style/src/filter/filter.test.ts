import { expect, test } from "@destack/test";
import { FilterStack } from "./index.ts";

test("write filters in order as CSS's filter list, and none for no filter", () => {
    expect([
        FilterStack.css([
            { kind: "blur", radius: { value: 4, unit: "px" } },
            { kind: "saturate", amount: 1.2 },
            { kind: "hue-rotate", angle: 90 },
        ]),
        FilterStack.css([]),
    ]).toEqual(["blur(4px) saturate(1.2) hue-rotate(90deg)", "none"]);
});
