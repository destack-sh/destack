import { expect, test } from "@destack/test";
import { Position } from "./index.ts";

test("place a box by its constraints: pinned to a token, centered past the middle, and scaled", () => {
    expect([
        Position.css({
            kind: "absolute",
            x: { kind: "center", offset: { value: 10, unit: "px" } },
            y: { kind: "end", offset: "{space.2}" },
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
            right: "max(0px, -2 * 10px)",
            bottom: "var(--destack-space-2)",
            left: "max(0px, 2 * 10px)",
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
