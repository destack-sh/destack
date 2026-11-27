import { expect, test } from "@destack/test";
import { TextFormat, TextStroke } from "./index.ts";

test("lay text out as CSS's text properties, clamping its lines where it truncates", () => {
    const format = {
        align: "center",
        case: "uppercase",
        decoration: "none",
        wrap: "balance",
        truncation: { kind: "none" },
    } as const;

    expect({
        full: TextFormat.css(format),
        cut: TextFormat.css({ ...format, truncation: { kind: "lines", count: 2 } }),
        stroke: TextStroke.css({ color: "#000000", width: { value: 1, unit: "px" } }),
    }).toEqual({
        full: {
            textAlign: "center",
            textTransform: "uppercase",
            textDecorationLine: "none",
            textWrap: "balance",
        },
        cut: {
            textAlign: "center",
            textTransform: "uppercase",
            textDecorationLine: "none",
            textWrap: "balance",
            overflow: "hidden",
            display: "-webkit-box",
            WebkitBoxOrient: "vertical",
            WebkitLineClamp: "2",
        },
        stroke: "1px #000000",
    });
});
