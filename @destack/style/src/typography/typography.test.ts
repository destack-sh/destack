import { expect, test } from "@destack/test";
import { Typography } from "./index.ts";

test("set text as CSS's font properties, refusing a weight past CSS's range", () => {
    const body = {
        family: "Inter, sans-serif",
        size: "1rem",
        weight: 400,
        lineHeight: 1.5,
        letterSpacing: "0",
    };

    expect({
        css: Typography.css(body),
        refused: Typography.safeParse({ ...body, weight: 1200 }).error?.issues.map(
            (issue) => issue.path,
        ),
    }).toEqual({
        css: {
            fontFamily: "Inter, sans-serif",
            fontSize: "1rem",
            fontWeight: "400",
            lineHeight: "1.5",
            letterSpacing: "0",
        },
        refused: [["weight"]],
    });
});
