import { expect, test } from "@destack/test";
import { Size } from "./index.ts";

test("size a box as CSS's width, height and limits, fit and fill as their keywords", () => {
    expect(
        Size.css({
            width: { kind: "fill" },
            height: { kind: "fit" },
            minWidth: { kind: "length", length: { value: 200, unit: "px" } },
            maxWidth: { kind: "none" },
            minHeight: { kind: "none" },
            maxHeight: { kind: "length", length: { value: 50, unit: "vh" } },
        }),
    ).toEqual({
        width: "stretch",
        height: "fit-content",
        minWidth: "200px",
        maxWidth: "none",
        minHeight: "auto",
        maxHeight: "50vh",
    });
});
