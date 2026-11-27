import { expect, test } from "@destack/test";
import { Gradient } from "./index.ts";

/** A gradient from red to blue through its middle, in a space. */
function redToBlue(space: "oklab" | "oklch" | "srgb", hue: "shorter" | "longer" = "shorter") {
    return new Gradient({
        shape: { kind: "linear", angle: 90 },
        stops: [
            { color: "#ff0000", position: 0 },
            { color: "#0000ff", position: 1 },
        ],
        space,
        hue,
    });
}

test("blend the middle of a gradient in its space, round the hue wheel the way it goes", () => {
    // each space and way gives its own middle, as the browser's color-mix() does, the ends their stops
    expect({
        start: redToBlue("srgb").at(0).toString("hex"),
        end: redToBlue("srgb").at(1).toString("hex"),
        beyond: redToBlue("srgb").at(-1).toString("hex"),
        srgb: redToBlue("srgb").at(0.5).toString("hex"),
        oklab: redToBlue("oklab").at(0.5).toString("hex"),
        shorter: redToBlue("oklch").at(0.5).toString("oklch"),
        longer: redToBlue("oklch", "longer").at(0.5).toString("oklch"),
    }).toEqual({
        start: "#ff0000",
        end: "#0000ff",
        beyond: "#ff0000",
        srgb: "#800080",
        oklab: "#8c53a2",
        shorter: "oklch(54% 0.2854 326.64)",
        longer: "oklch(54% 0.2854 146.64)",
    });
});

test("add a stop in the color it has there, move, recolor and remove stops by their index", () => {
    // stops keep their index as they move past each other
    const gradient = redToBlue("srgb").add(0.5).move(0, 0.75).recolor(1, "#00ff00");

    expect([gradient.toJson().stops, gradient.remove(2).toJson().stops]).toEqual([
        [
            { color: "#ff0000", position: 0.75 },
            { color: "#00ff00", position: 1 },
            { color: "#800080", position: 0.5 },
        ],
        [
            { color: "#ff0000", position: 0.75 },
            { color: "#00ff00", position: 1 },
        ],
    ]);
});

test("spread the stops in another kind of shape, keeping the angle and the center", () => {
    // turn a line at 45 degrees round a center, then out from it, then back along a line
    const conic = redToBlue("oklab").reshape({ kind: "linear", angle: 45 }).shaped("conic");
    const radial = conic
        .reshape({ kind: "conic", center: { x: 0.2, y: 0.8 }, angle: 45 })
        .shaped("radial");

    expect([conic.shape, radial.shape, radial.shaped("linear").shape]).toEqual([
        { kind: "conic", center: { x: 0.5, y: 0.5 }, angle: 45 },
        { kind: "radial", center: { x: 0.2, y: 0.8 } },
        { kind: "linear", angle: 90 },
    ]);
});

test("refuse a gradient of fewer than two stops", () => {
    expect(() => redToBlue("srgb").remove(0)).toThrow(
        new TypeError("a gradient keeps at least two stops"),
    );
});

test("write a gradient as CSS in order along it, in its shape, naming its space and its way round the hues", () => {
    const gradient = redToBlue("oklch", "longer").add(0.25).move(0, 0.5);
    const center = { x: 0.25, y: 0.5 };

    // the added stop takes the color a quarter of the longer way round, in OKLCH
    expect([
        gradient.css(),
        redToBlue("srgb").reshape({ kind: "linear", angle: 45 }).css(),
        redToBlue("oklab").reshape({ kind: "radial", center }).css(),
        redToBlue("oklab").reshape({ kind: "conic", center, angle: 30 }).css(),
    ]).toEqual([
        "linear-gradient(90deg in oklch longer hue, oklch(58.4% 0.2716 87.94) 25%, #ff0000 50%, #0000ff 100%)",
        "linear-gradient(45deg in srgb, #ff0000 0%, #0000ff 100%)",
        "radial-gradient(at 25% 50% in oklab, #ff0000 0%, #0000ff 100%)",
        "conic-gradient(from 30deg at 25% 50% in oklab, #ff0000 0%, #0000ff 100%)",
    ]);
});
