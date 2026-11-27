import { expect, test } from "@destack/test";
import { type Fill, FillStack } from "./index.ts";

/** A gradient from blue to gold along a line to the right. */
const SKY = {
    shape: { kind: "linear", angle: 90 },
    stops: [
        { color: "#241d9a", position: 0 },
        { color: "#ffc53d", position: 1 },
    ],
    space: "oklab",
    hue: "shorter",
} as const;

test("draw a fill's shown layers as CSS background layers, top first", () => {
    // stack a tinted color over a tiled image over a gradient, with a hidden layer between
    const stack: readonly Fill[] = [
        { paint: { kind: "solid", color: "#0000ff80" }, blend: "multiply", isVisible: true },
        { paint: { kind: "solid", color: "#ff0000" }, blend: "normal", isVisible: false },
        {
            paint: { kind: "image", source: "/paper.png", fit: "tile", position: { x: 0.5, y: 0 } },
            blend: "normal",
            isVisible: true,
        },
        { paint: { kind: "gradient", gradient: SKY }, blend: "normal", isVisible: true },
    ];

    // expect each property to list the three shown layers in order
    expect([FillStack.css(stack), FillStack.css([])]).toEqual([
        {
            backgroundImage:
                'linear-gradient(#0000ff80, #0000ff80), url("/paper.png"), linear-gradient(90deg in oklab, #241d9a 0%, #ffc53d 100%)',
            backgroundSize: "100% 100%, auto, 100% 100%",
            backgroundPosition: "0% 0%, 50% 0%, 0% 0%",
            backgroundRepeat: "no-repeat, repeat, no-repeat",
            backgroundBlendMode: "multiply, normal, normal",
        },
        {
            backgroundImage: "none",
            backgroundSize: "",
            backgroundPosition: "",
            backgroundRepeat: "",
            backgroundBlendMode: "",
        },
    ]);
});

/** Read where a fill stack refuses a value, by each issue's path. */
function refused(value: unknown): string[] {
    return (FillStack.safeParse(value).error?.issues ?? []).map((issue) => issue.path.join("."));
}

test("refuse a fill whose paint or blend CSS does not draw", () => {
    // read a layer with an unknown blend, and one painting an unknown kind
    const blend = { paint: { kind: "solid", color: "#ff0000" }, blend: "glow", isVisible: true };
    const kind = { paint: { kind: "pattern" }, blend: "normal", isVisible: true };

    // expect each refused at the field CSS cannot draw
    expect([refused([blend]), refused([kind])]).toEqual([["0.blend"], ["0.paint.kind"]]);
});
