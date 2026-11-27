import { expect, test } from "@destack/test";
import { Stroke } from "./index.ts";

test("draw a stroke inside as a border and outside as an outline flush with the edge", () => {
    const stroke = { color: "#0000ff", width: "2px", align: "inside", style: "dashed" } as const;

    expect([Stroke.css(stroke), Stroke.css({ ...stroke, align: "outside" })]).toEqual([
        { borderColor: "#0000ff", borderWidth: "2px", borderStyle: "dashed" },
        {
            outlineColor: "#0000ff",
            outlineWidth: "2px",
            outlineStyle: "dashed",
            outlineOffset: "0",
        },
    ]);
});
