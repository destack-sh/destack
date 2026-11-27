import { expect, test } from "@destack/test";
import { CubicBezier, Dimension, ShadowStack, Typography } from "../index.ts";
import { Reference } from "./index.ts";

test("read and write a reference to a token, as its path and its custom property", () => {
    const references = ["{space.4}", "{color.mutedForeground}", "{motion.duration.short}"];

    expect({
        paths: references.map((reference) => Reference.path(reference)),
        css: references.map((reference) => Reference.css(reference)),
        written: Reference.to(["radius", "2"]),
        taken: ["{space.4}", "space.4", "{}", "{space..4}", "{4.space}", 4].map((value) =>
            Reference.is(value),
        ),
    }).toEqual({
        paths: [
            ["space", "4"],
            ["color", "mutedForeground"],
            ["motion", "duration", "short"],
        ],
        css: [
            "var(--destack-space-4)",
            "var(--destack-color-muted-foreground)",
            "var(--destack-motion-duration-short)",
        ],
        written: "{radius.2}",
        taken: [true, false, false, false, false, false],
    });
});

test("write a bound value as its token's variable, and a literal as itself", () => {
    expect({
        dimensions: [Dimension.css("{radius.2}"), Dimension.css({ value: 12, unit: "px" })],
        shadows: ShadowStack.css("{shadow.raised}"),
        typography: Typography.css("{text.base}"),
        parsed: [
            Dimension.bindable.safeParse("{radius.2}").success,
            Dimension.bindable.safeParse("radius.2").success,
        ],
    }).toEqual({
        dimensions: ["var(--destack-radius-2)", "12px"],
        shadows: "var(--destack-shadow-raised)",
        typography: {
            fontFamily: "var(--destack-text-base-font-family)",
            fontSize: "var(--destack-text-base-font-size)",
            fontWeight: "var(--destack-text-base-font-weight)",
            letterSpacing: "var(--destack-text-base-letter-spacing)",
            lineHeight: "var(--destack-text-base-line-height)",
        },
        parsed: [true, false],
    });
});

test("read the values a browser computes, as detaching a token writes them", () => {
    expect({
        dimensions: ["16px", "-1.5rem", "0.25em", "12", "auto"].map((text) => Dimension.read(text)),
        curves: ["ease-out", "cubic-bezier(0.2, 0, 0, 1)", "steps(4)"].map((text) =>
            CubicBezier.read(text),
        ),
        shadows: [
            "rgba(0, 0, 0, 0.06) 0px 0px 0px 1px, rgb(0, 0, 0) 0px 1px 3px 0px inset",
            "none",
            "0px 1px",
        ].map((text) => ShadowStack.read(text)),
    }).toEqual({
        dimensions: [
            { value: 16, unit: "px" },
            { value: -1.5, unit: "rem" },
            { value: 0.25, unit: "em" },
            undefined,
            undefined,
        ],
        curves: [[0, 0, 0.58, 1], [0.2, 0, 0, 1], undefined],
        shadows: [
            [
                {
                    color: "rgba(0, 0, 0, 0.06)",
                    offsetX: { value: 0, unit: "px" },
                    offsetY: { value: 0, unit: "px" },
                    blur: { value: 0, unit: "px" },
                    spread: { value: 1, unit: "px" },
                    inset: false,
                },
                {
                    color: "rgb(0, 0, 0)",
                    offsetX: { value: 0, unit: "px" },
                    offsetY: { value: 1, unit: "px" },
                    blur: { value: 3, unit: "px" },
                    spread: { value: 0, unit: "px" },
                    inset: true,
                },
            ],
            [],
            undefined,
        ],
    });
});
