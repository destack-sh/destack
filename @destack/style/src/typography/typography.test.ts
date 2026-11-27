import { expect, test } from "@destack/test";
import { Typography } from "./index.ts";

test("set text as CSS's font properties, refusing a weight past CSS's range", () => {
    const body = {
        fontFamily: ["Inter", "sans-serif"],
        fontSize: { value: 1, unit: "rem" },
        fontWeight: 400,
        letterSpacing: { value: 0, unit: "em" },
        lineHeight: 1.5,
    } satisfies Typography;

    expect({
        css: Typography.css(body),
        refused: Typography.safeParse({ ...body, fontWeight: 1200 }).error?.issues.map(
            (issue) => issue.path,
        ),
    }).toEqual({
        css: {
            fontFamily: "Inter, sans-serif",
            fontSize: "1rem",
            fontWeight: "400",
            letterSpacing: "0em",
            lineHeight: "1.5",
        },
        refused: [["fontWeight"]],
    });
});
