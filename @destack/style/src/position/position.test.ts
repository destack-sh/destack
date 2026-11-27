import { expect, test } from "@destack/test";
import { Position } from "./index.ts";

test("place a box by its constraints: pinned, centered past the middle, and scaled", () => {
    expect([
        Position.css({
            kind: "absolute",
            x: { kind: "center", offset: { value: 10, unit: "px" } },
            y: { kind: "end", offset: { value: 8, unit: "px" } },
        }),
        Position.css({
            kind: "absolute",
            x: { kind: "scale", start: 0.25, end: 0.75 },
            y: { kind: "both", start: { value: 0, unit: "px" }, end: { value: 16, unit: "px" } },
        }),
    ]).toEqual([
        {
            position: "absolute",
            top: "auto",
            right: "0",
            bottom: "8px",
            left: "20px",
            marginInline: "auto",
            marginBlock: "0",
        },
        {
            position: "absolute",
            top: "0px",
            right: "25%",
            bottom: "16px",
            left: "25%",
            marginInline: "0",
            marginBlock: "0",
        },
    ]);
});
