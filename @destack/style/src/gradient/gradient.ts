import { type Message, t } from "@destack/locale";
import { present, schema } from "@destack/schema";
import { Color, GRAY_CHROMA } from "../color/index.ts";

/** A space a gradient blends its colors in, as CSS interpolates them. */
export const GradientSpace = schema.enum(["oklab", "oklch", "srgb"]);
/** A space a gradient blends its colors in. */
export type GradientSpace = schema.Infer<typeof GradientSpace>;

/** A way a gradient goes round the hue wheel between two colors, in a space with hue. */
export const GradientHue = schema.enum(["shorter", "longer"]);
/** A way a gradient goes round the hue wheel. */
export type GradientHue = schema.Infer<typeof GradientHue>;

/** A kind of shape a gradient spreads its stops in, as CSS's gradient functions draw them. */
export const GradientShapeKind = schema.enum(["linear", "radial", "conic"]);
/** A kind of shape a gradient spreads its stops in. */
export type GradientShapeKind = schema.Infer<typeof GradientShapeKind>;

/** The spaces a gradient blends its colors in, titled and described. */
export const GRADIENT_SPACES = {
    oklab: {
        title: t.context("option")`OKLab`,
        description: t`Evenly as people see it, without passing through other hues.`,
    },
    oklch: {
        title: t.context("option")`OKLCH`,
        description: t`Round the hue wheel as people see it, keeping colors vivid.`,
    },
    srgb: {
        title: t.context("option")`sRGB`,
        description: t`By its red, green and blue light, as older tools blend.`,
    },
} satisfies Record<GradientSpace, { readonly title: Message; readonly description: Message }>;

/** The ways a gradient goes round the hue wheel, titled and described. */
export const GRADIENT_HUES = {
    shorter: {
        title: t.context("option")`Shorter`,
        description: t`The shorter way round the wheel.`,
    },
    longer: {
        title: t.context("option")`Longer`,
        description: t`The longer way round the wheel, through more hues.`,
    },
} satisfies Record<GradientHue, { readonly title: Message; readonly description: Message }>;

/** The kinds of shape a gradient spreads its stops in, titled and described. */
export const GRADIENT_SHAPES = {
    linear: {
        title: t.context("option")`Linear`,
        description: t`Along a straight line at an angle.`,
    },
    radial: {
        title: t.context("option")`Radial`,
        description: t`Outward from a center to the farthest corner.`,
    },
    conic: {
        title: t.context("option")`Conic`,
        description: t`Round a center, from an angle.`,
    },
} satisfies Record<GradientShapeKind, { readonly title: Message; readonly description: Message }>;

/** A place in a box, from its top left at 0 to its bottom right at 1. */
export const BoxPoint = Object.assign(
    schema.object({
        /** How far across, from 0 at the left to 1 at the right. */
        x: schema.number().min(0).max(1),
        /** How far down, from 0 at the top to 1 at the bottom. */
        y: schema.number().min(0).max(1),
    }),
    {
        /** The center of a box. */
        center: { x: 0.5, y: 0.5 } as const,

        /** Write a place in a box as CSS positions it, in percent. */
        css(point: BoxPoint): string {
            return `${String(point.x * 100)}% ${String(point.y * 100)}%`;
        },
    },
);
/** A place in a box. */
export type BoxPoint = schema.Infer<typeof BoxPoint>;

/** How a gradient spreads its stops over a box: along a line, out from a center, or round it. */
const gradientShapeJson = schema.discriminatedUnion("kind", [
    schema.object({
        /** Along a straight line. */
        kind: schema.literal("linear"),
        /** The line's direction in degrees, as CSS turns it: 0 to the top, 90 to the right. */
        angle: schema.number().min(0).max(360),
    }),
    schema.object({
        /** Outward from a center to the farthest corner, in an ellipse. */
        kind: schema.literal("radial"),
        /** The center. */
        center: BoxPoint,
    }),
    schema.object({
        /** Round a center. */
        kind: schema.literal("conic"),
        /** The center. */
        center: BoxPoint,
        /** The angle it starts at in degrees, 0 at the top. */
        angle: schema.number().min(0).max(360),
    }),
]);
/** How a gradient spreads its stops over a box. */
export type GradientShape = schema.Infer<typeof gradientShapeJson>;

/** How a gradient spreads its stops over a box, and the shape it starts in. */
export const GradientShape = Object.assign(gradientShapeJson, {
    /** A line to the right, as design tools start a gradient. */
    rightward: { kind: "linear", angle: 90 } as const,
});

/** A color at a place along a gradient, from its start at 0 to its end at 1. */
export const GradientStop = schema.object({
    /** The color, as CSS writes it. */
    color: Color.schema,
    /** The place along the gradient, from 0 to 1. */
    position: schema.number().min(0).max(1),
});
/** A color at a place along a gradient. */
export type GradientStop = schema.Infer<typeof GradientStop>;

/** A gradient as JSON stores it: its shape, its stops, and how it blends between them. */
const gradientJson = schema
    .object({
        /** The shape it spreads its stops in. */
        shape: GradientShape,
        /** The stops, in the order they were added. */
        stops: schema.array(GradientStop).min(2).readonly(),
        /** The space it blends in. */
        space: GradientSpace,
        /** The way it goes round the hue wheel, in a space with hue. */
        hue: GradientHue,
    })
    .meta({ id: "gradient", title: "Gradient" });
/** A gradient as JSON stores it. */
export type GradientJson = schema.Infer<typeof gradientJson>;

/** A gradient: colors at places along it, blended between in a color space. */
export class Gradient {
    /** The schema of a gradient as JSON stores it. */
    static readonly schema = gradientJson;

    /** The shape it spreads its stops in. */
    readonly shape: GradientShape;
    /** The stops, in the order they were added, which keeps each one's index as it moves. */
    readonly stops: readonly GradientStop[];
    /** The space it blends in. */
    readonly space: GradientSpace;
    /** The way it goes round the hue wheel, in a space with hue. */
    readonly hue: GradientHue;

    /** Say a gradient from its JSON. */
    constructor(json: GradientJson) {
        // keep the shape, the stops and how it blends
        this.shape = json.shape;
        this.stops = json.stops;
        this.space = json.space;
        this.hue = json.hue;
    }

    /** Read a gradient from JSON, refusing any other value. */
    static of(value: unknown): Gradient {
        return new Gradient(gradientJson.parse(value));
    }

    /** Make a gradient from one color to another, to the right in OKLab, as design tools start. */
    static between(from: string, to: string): Gradient {
        return new Gradient({
            shape: GradientShape.rightward,
            stops: [
                { color: from, position: 0 },
                { color: to, position: 1 },
            ],
            space: "oklab",
            hue: "shorter",
        });
    }

    /** Write the gradient as JSON stores it. */
    toJson(): GradientJson {
        return { shape: this.shape, stops: this.stops, space: this.space, hue: this.hue };
    }

    /** Read the color at a place along the gradient, the nearest stop's beyond its ends. */
    at(position: number): Color {
        // find the stops around the place, in order along the gradient
        const sorted = this.#sorted();
        const after = sorted.findIndex((stop) => stop.position >= position);
        const next = present(after === -1 ? sorted.at(-1) : sorted[after], "a gradient's stop");
        const previous = after <= 0 ? next : present(sorted[after - 1], "a gradient's stop");

        // blend the two by how far the place lies between them
        const span = next.position - previous.position;
        const share = span === 0 ? 0 : (position - previous.position) / span;

        return this.#mix(Color.parse(previous.color), Color.parse(next.color), share);
    }

    /** Read the stop at the gradient's start, the first along it. */
    first(): GradientStop {
        return present(this.#sorted()[0], "a gradient's first stop");
    }

    /** Add a stop at a place, in the color the gradient has there. */
    add(position: number): Gradient {
        // write it in sRGB only for an sRGB gradient, as hexadecimal would move a vivid color
        const color = this.at(position).toString(this.space === "srgb" ? "hex" : "oklch");

        return this.#with([...this.stops, { color, position }]);
    }

    /** Move a stop to another place. */
    move(index: number, position: number): Gradient {
        const clamped = Math.min(Math.max(position, 0), 1);

        return this.#with(
            this.stops.map((stop, place) =>
                place === index ? { ...stop, position: clamped } : stop,
            ),
        );
    }

    /** Give a stop another color. */
    recolor(index: number, color: string): Gradient {
        return this.#with(
            this.stops.map((stop, place) => (place === index ? { ...stop, color } : stop)),
        );
    }

    /** Remove a stop, refusing to leave fewer than two. */
    remove(index: number): Gradient {
        if (this.stops.length <= 2) {
            throw new TypeError("a gradient keeps at least two stops");
        }

        return this.#with(this.stops.toSpliced(index, 1));
    }

    /** Blend in another space, going round the hue wheel one way. */
    blend(space: GradientSpace, hue: GradientHue): Gradient {
        return new Gradient({ ...this.toJson(), space, hue });
    }

    /** Spread the stops in another shape. */
    reshape(shape: GradientShape): Gradient {
        return new Gradient({ ...this.toJson(), shape });
    }

    /** Spread the stops in another kind of shape, keeping the angle and center it has. */
    to(kind: GradientShapeKind): Gradient {
        // take the angle and the center over, else start right and at the center
        const { shape } = this;
        const angle = shape.kind === "radial" ? GradientShape.rightward.angle : shape.angle;
        const center = shape.kind === "linear" ? BoxPoint.center : shape.center;
        const shapes = {
            linear: { kind: "linear", angle },
            radial: { kind: "radial", center },
            conic: { kind: "conic", center, angle },
        } as const;

        return this.reshape(shapes[kind]);
    }

    /** Write the gradient as the CSS gradient function its shape draws. */
    css(): string {
        // name the space and, in one with hue, the way round the wheel
        const method = this.space === "oklch" ? `in oklch ${this.hue} hue` : `in ${this.space}`;
        const stops = this.#sorted().map((stop) => `${stop.color} ${String(stop.position * 100)}%`);
        const { shape } = this;

        // draw along a line, out from a center, or round it
        if (shape.kind === "linear") {
            return `linear-gradient(${String(shape.angle)}deg ${method}, ${stops.join(", ")})`;
        } else if (shape.kind === "radial") {
            return `radial-gradient(at ${BoxPoint.css(shape.center)} ${method}, ${stops.join(", ")})`;
        } else {
            return `conic-gradient(from ${String(shape.angle)}deg at ${BoxPoint.css(shape.center)} ${method}, ${stops.join(", ")})`;
        }
    }

    /** Read the stops in order along the gradient. */
    #sorted(): GradientStop[] {
        return this.stops.toSorted((first, second) => first.position - second.position);
    }

    /** Say the gradient with other stops. */
    #with(stops: readonly GradientStop[]): Gradient {
        return new Gradient({ ...this.toJson(), stops });
    }

    /** Blend two colors in the gradient's space, weighed by their alpha as CSS does. */
    #mix(first: Color, second: Color, share: number): Color {
        // weigh the alpha, then each color by its own
        const alpha = lerp(first.alpha, second.alpha, share);
        const weigh = (from: number, to: number): number =>
            alpha === 0 ? 0 : lerp(from * first.alpha, to * second.alpha, share) / alpha;

        // blend the light in sRGB
        if (this.space === "srgb") {
            const [red, green, blue] = first.to("rgb").values;
            const [red2, green2, blue2] = second.to("rgb").values;

            return new Color(
                "rgb",
                [weigh(red, red2), weigh(green, green2), weigh(blue, blue2)],
                alpha,
            );
        }
        // blend lightness and chroma, and the hue round the wheel, in OKLCH
        else if (this.space === "oklch") {
            const [lightness, chroma, hue] = first.to("oklch").values;
            const [lightness2, chroma2, hue2] = second.to("oklch").values;
            const from = chroma < GRAY_CHROMA ? hue2 : hue;
            const to = chroma2 < GRAY_CHROMA ? hue : hue2;

            return new Color(
                "oklch",
                [
                    weigh(lightness, lightness2),
                    weigh(chroma, chroma2),
                    turn(from, to, share, this.hue),
                ],
                alpha,
            );
        }
        // blend the lightness and the two axes of color in OKLab
        else {
            const [lightness, greenRed, blueYellow] = lab(first);
            const [lightness2, greenRed2, blueYellow2] = lab(second);
            const mixedGreenRed = weigh(greenRed, greenRed2);
            const mixedBlueYellow = weigh(blueYellow, blueYellow2);
            const hue = (Math.atan2(mixedBlueYellow, mixedGreenRed) * 180) / Math.PI;

            return new Color(
                "oklch",
                [
                    weigh(lightness, lightness2),
                    Math.hypot(mixedGreenRed, mixedBlueYellow),
                    (hue + 360) % 360,
                ],
                alpha,
            );
        }
    }
}

/** Go a share of the way from one number to another. */
function lerp(from: number, to: number, share: number): number {
    return from + (to - from) * share;
}

/** Go a share of the way round the hue wheel from one hue to another, the shorter or longer way. */
function turn(from: number, to: number, share: number, way: GradientHue): number {
    // take the difference between -180 and 180, then the other way round for the longer one
    const shorter = ((((to - from) % 360) + 540) % 360) - 180;
    const longer = shorter > 0 ? shorter - 360 : shorter + 360;
    const delta = way === "shorter" ? shorter : longer;

    return (from + delta * share + 360) % 360;
}

/** Say a color in OKLab: its lightness in percent, and its two axes of color. */
function lab(color: Color): readonly [number, number, number] {
    const [lightness, chroma, hue] = color.to("oklch").values;
    const radians = (hue * Math.PI) / 180;

    return [lightness, chroma * Math.cos(radians), chroma * Math.sin(radians)];
}
