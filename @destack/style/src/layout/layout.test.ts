import { expect, test } from "@destack/test";
import { BoxSides } from "../box/index.ts";
import { Layout } from "./index.ts";

test("lay a box out as CSS's flexbox properties", () => {
    const layout = {
        direction: "column",
        gap: { value: 8, unit: "px" },
        align: "stretch",
        justify: "space-between",
        isWrapping: false,
        padding: BoxSides.all({ value: 12, unit: "px" }),
    } as const;

    expect(Layout.css(layout)).toEqual({
        display: "flex",
        flexDirection: "column",
        gap: "8px",
        alignItems: "stretch",
        justifyContent: "space-between",
        flexWrap: "nowrap",
        padding: "12px 12px 12px 12px",
    });
});
