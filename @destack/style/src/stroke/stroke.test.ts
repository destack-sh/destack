import { expect, test } from "@destack/test";
import { Stroke } from "./index.ts";

test("draw a stroke inside as a border and outside as an outline flush with the edge", () => {
    const stroke = {
        color: "#0000ff",
        width: { value: 2, unit: "px" },
        align: "inside",
        style: "double",
    } as const;

    expect([Stroke.css(stroke), Stroke.css({ ...stroke, align: "outside" })]).toEqual([
        { borderColor: "#0000ff", borderWidth: "2px", borderStyle: "double" },
        {
            outlineColor: "#0000ff",
            outlineWidth: "2px",
            outlineStyle: "double",
            outlineOffset: "0",
        },
    ]);
});
