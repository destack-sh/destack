import { type Message, t } from "@destack/locale";
import { schema } from "@destack/schema";
import { Color, GRAY_CHROMA } from "../color/index.ts";

/** The spaces a gradient blends its colors in, as CSS interpolates them. */
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
} satisfies Record<string, { readonly title: Message; readonly description: Message }>;

/** A space a gradient blends its colors in. */
export type GradientSpace = keyof typeof GRADIENT_SPACES;

/** The ways a gradient goes round the hue wheel between two colors, in a space with hue. */
export const GRADIENT_HUES = {
    shorter: {
        title: t.context("option")`Shorter`,
        description: t`The shorter way round the wheel.`,
    },
    longer: {
        title: t.context("option")`Longer`,
        description: t`The longer way round the wheel, through more hues.`,
    },
} satisfies Record<string, { readonly title: Message; readonly description: Message }>;

/** A way a gradient goes round the hue wheel. */
export type GradientHue = keyof typeof GRADIENT_HUES;

/** A color at a place along a gradient, from its start at 0 to its end at 1. */
export const GradientStop = schema.object({
    /** The color, as CSS writes it. */
    color: Color.schema,
    /** The place along the gradient, from 0 to 1. */
    position: schema.number().min(0).max(1),
});
/** A color at a place along a gradient. */
export type GradientStop = schema.Infer<typeof GradientStop>;

/** A gradient as JSON stores it: its stops, and how it blends between them. */
const gradientJson = schema
    .object({
        /** The stops, in the order they were added. */
        stops: schema.array(GradientStop).min(2).readonly(),
        /** The space it blends in. */
        space: schema.enum(["oklab", "oklch", "srgb"]),
        /** The way it goes round the hue wheel, in a space with hue. */
        hue: schema.enum(["shorter", "longer"]),
    })
    .meta({ id: "gradient", title: "Gradient" });
/** A gradient as JSON stores it. */
export type GradientJson = schema.Infer<typeof gradientJson>;

/** A gradient: colors at places along it, blended between in a color space. */
export class Gradient {
    /** The schema of a gradient as JSON stores it. */
    static readonly schema = gradientJson;

    /** The stops, in the order they were added, which keeps each one's index as it moves. */
    readonly stops: readonly GradientStop[];
    /** The space it blends in. */
    readonly space: GradientSpace;
    /** The way it goes round the hue wheel, in a space with hue. */
    readonly hue: GradientHue;

    /** Say a gradient from its JSON. */
    constructor(json: GradientJson) {
        this.stops = json.stops;
        this.space = json.space;
        this.hue = json.hue;
    }

    /** Read a gradient from JSON, refusing any other value. */
    static of(value: unknown): Gradient {
        return new Gradient(gradientJson.parse(value));
    }

    /** Write the gradient as JSON stores it. */
    toJson(): GradientJson {
        return { stops: this.stops, space: this.space, hue: this.hue };
    }

    /** Read the color at a place along the gradient, the nearest stop's beyond its ends. */
    at(position: number): Color {
        // find the stops around the place, in order along the gradient
        const sorted = this.#sorted();
        const after = sorted.findIndex((stop) => stop.position >= position);
        const end = sorted.at(-1) ?? sorted[0];
        const next = after === -1 ? end : sorted[after];
        const previous = after <= 0 ? next : sorted[after - 1];
        if (previous === undefined || next === undefined) {
            throw new TypeError("a gradient has no stops");
        }

        // blend the two by how far the place lies between them
        const span = next.position - previous.position;
        const share = span === 0 ? 0 : (position - previous.position) / span;

        return this.#mix(Color.parse(previous.color), Color.parse(next.color), share);
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
        return new Gradient({ stops: this.stops, space, hue });
    }

    /** Write the gradient as a CSS linear gradient in a direction, such as `to right`. */
    linear(direction: string): string {
        // name the space and, in one with hue, the way round the wheel
        const method = this.space === "oklch" ? `in oklch ${this.hue} hue` : `in ${this.space}`;
        const stops = this.#sorted().map((stop) => `${stop.color} ${String(stop.position * 100)}%`);

        return `linear-gradient(${direction} ${method}, ${stops.join(", ")})`;
    }

    /** Read the stops in order along the gradient. */
    #sorted(): GradientStop[] {
        return this.stops.toSorted((first, second) => first.position - second.position);
    }

    /** Say the gradient with other stops. */
    #with(stops: readonly GradientStop[]): Gradient {
        return new Gradient({ stops, space: this.space, hue: this.hue });
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
