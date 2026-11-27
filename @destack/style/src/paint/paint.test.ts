import { expect, test } from "@destack/test";
import { Paint } from "./index.ts";

/** Refuse to resolve a reference, for paints of literal colors. */
function unresolved(reference: string): never {
    throw new TypeError(`no token for '${reference}'`);
}

/** A gradient from blue to gold. */
const SKY = {
    shape: { kind: "radial", center: { x: 0.5, y: 0.5 } },
    stops: [
        { color: "#241d9a", position: 0 },
        { color: "#ffc53d", position: 1 },
    ],
    space: "oklch",
    hue: "longer",
} as const;

/** An image tiled from the top left. */
const PAPER = {
    kind: "image",
    source: "/paper.png",
    fit: "tile",
    position: { x: 0, y: 0 },
} as const;

test("turn a paint into a color or a gradient, as a picker changes its kind", () => {
    // turn a color, a gradient and an image into each other kind
    const solid = { kind: "solid", color: "#0000ff" } as const;
    const gradient = { kind: "gradient", gradient: SKY } as const;
    const added = { ...SKY, stops: [{ color: "#ffc53d", position: 1 }, ...SKY.stops.slice(0, 1)] };

    // a gradient turns into its first stop along it, not the first added
    expect({
        colors: [solid, gradient, { kind: "gradient", gradient: added } as const, PAPER].map(
            (paint) => Paint.color(paint),
        ),
        gradients: [solid, gradient, PAPER].map((paint) => Paint.gradient(paint, unresolved)),
    }).toEqual({
        colors: ["#0000ff", "#241d9a", "#241d9a", "#000000"],
        gradients: [
            {
                shape: { kind: "linear", angle: 90 },
                stops: [
                    { color: "#0000ff", position: 0 },
                    { color: "#0000ff00", position: 1 },
                ],
                space: "oklab",
                hue: "shorter",
            },
            SKY,
            {
                shape: { kind: "linear", angle: 90 },
                stops: [
                    { color: "#000000", position: 0 },
                    { color: "#ffffff", position: 1 },
                ],
                space: "oklab",
                hue: "shorter",
            },
        ],
    });
});

test("make an image paint covering the box from its center", () => {
    expect(Paint.image("/paper.png")).toEqual({
        kind: "image",
        source: "/paper.png",
        fit: "cover",
        position: { x: 0.5, y: 0.5 },
    });
});

test("draw each kind of paint as one CSS background layer", () => {
    expect([
        Paint.layer({ kind: "solid", color: "#0000ff" }),
        Paint.layer({ kind: "gradient", gradient: SKY }),
        Paint.layer(PAPER),
        Paint.layer({ ...PAPER, fit: "contain", position: { x: 1, y: 0.5 } }),
    ]).toEqual([
        {
            image: "linear-gradient(#0000ff, #0000ff)",
            size: "100% 100%",
            position: "0% 0%",
            repeat: "no-repeat",
        },
        {
            image: "radial-gradient(at 50% 50% in oklch longer hue, #241d9a 0%, #ffc53d 100%)",
            size: "100% 100%",
            position: "0% 0%",
            repeat: "no-repeat",
        },
        { image: 'url("/paper.png")', size: "auto", position: "0% 0%", repeat: "repeat" },
        { image: 'url("/paper.png")', size: "contain", position: "100% 50%", repeat: "no-repeat" },
    ]);
});
