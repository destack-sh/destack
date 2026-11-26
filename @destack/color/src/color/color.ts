import { type Message, t } from "@destack/locale";
import { schema } from "@destack/schema";

/** The channels of each color space, in the order CSS writes them. */
export const COLOR_CHANNELS = {
    rgb: ["red", "green", "blue"],
    hsl: ["hue", "saturation", "lightness"],
    hsb: ["hue", "saturation", "brightness"],
    oklch: ["lightness", "chroma", "hue"],
} as const;

/** A way to say a color in three numbers: by its light, or as people see it. */
export type ColorSpace = keyof typeof COLOR_CHANNELS;

/** The ways to say a color in three numbers, with their choice in pickers. */
export const ColorSpace = {
    /** Each color space as a picker offers it: its name and how it says a color. */
    choices: {
        rgb: {
            title: t.context("option")`RGB`,
            description: t`By its red, green and blue light, as screens mix it.`,
        },
        hsl: {
            title: t.context("option")`HSL`,
            description: t`By its hue, saturation and lightness, as CSS writes it.`,
        },
        hsb: {
            title: t.context("option")`HSB`,
            description: t`By its hue, saturation and brightness, as design tools pick it.`,
        },
        oklch: {
            title: t.context("option")`OKLCH`,
            description: t`By its lightness, chroma and hue, as people see it.`,
        },
    } satisfies Record<ColorSpace, { readonly title: Message; readonly description: Message }>,
};

/** One number of a color. */
export type ColorChannel = (typeof COLOR_CHANNELS)[ColorSpace][number] | "alpha";

/** The way a color is written in CSS. */
export type ColorFormat = "hex" | "rgb" | "hsl" | "oklch";

/** The numbers of a color without its alpha. */
export type ColorValues = readonly [first: number, second: number, third: number];

/** A hue, black, white or gray as people call it. */
export type ColorHueName = (typeof HUE_NAMES)[number][1] | "brown" | "black" | "white" | "gray";

/** What people call a color, each part for a language to write. */
export interface ColorName {
    /** How dark or light it is, undefined in the middle. */
    readonly lightness: "very dark" | "dark" | "light" | "very light" | undefined;
    /** How strong it is, undefined in the middle. */
    readonly chroma: "grayish" | "pale" | "vibrant" | undefined;
    /** Its hue, and the next one where it is between two. */
    readonly hues: readonly ColorHueName[];
}

/** The values a channel takes. */
export interface ColorChannelRange {
    /** The lowest value. */
    readonly min: number;
    /** The highest value. */
    readonly max: number;
    /** The step between values. */
    readonly step: number;
}

/** The values each channel takes, whichever space it is of. */
const RANGES: Readonly<Record<ColorChannel, ColorChannelRange>> = {
    red: { min: 0, max: 255, step: 1 },
    green: { min: 0, max: 255, step: 1 },
    blue: { min: 0, max: 255, step: 1 },
    hue: { min: 0, max: 360, step: 1 },
    saturation: { min: 0, max: 100, step: 1 },
    lightness: { min: 0, max: 100, step: 1 },
    brightness: { min: 0, max: 100, step: 1 },
    // the most vivid color a screen of P3 shows is below 0.4
    chroma: { min: 0, max: 0.4, step: 0.005 },
    alpha: { min: 0, max: 1, step: 0.01 },
};

/** The hues a color is called by, with the angle each begins at, after React Aria. */
const HUE_NAMES = [
    [0, "pink"],
    [15, "red"],
    [48, "orange"],
    [94, "yellow"],
    [135, "green"],
    [175, "cyan"],
    [264, "blue"],
    [284, "purple"],
    [320, "magenta"],
    [349, "pink"],
    [360, "pink"],
] as const;

/** The chroma below which a color is gray. */
const GRAY_CHROMA = 0.001;

/** The lightness below which orange is brown. */
const BROWN_LIGHTNESS = 0.68;

/** The lightness below which yellow is yellow green. */
const YELLOW_GREEN_LIGHTNESS = 0.85;

/** The lightness below which a color is dark. */
const DARK_LIGHTNESS = 0.55;

/** A color's hexadecimal digits, one or two for each channel. */
const HEX = /^#(?:[\da-f]{3,4}|[\da-f]{6}|[\da-f]{8})$/iu;

/** A color function of CSS and its arguments. */
const FUNCTION = /^(rgb|hsl|oklch)a?\(([^()]+)\)$/iu;

/** A number of a color function, with the unit it may have. */
const NUMBER = /^([+-]?(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?)(%|deg)?$/iu;

/** The room a channel may leave its range by before a color is out of a screen's. */
const GAMUT_TOLERANCE = 0.0001;

/** The halvings that find the chroma a screen shows, precise to a millionth. */
const GAMUT_STEPS = 18;

/** A color in one space, which another has the nearest color of. */
export class Color {
    /** A color CSS writes as text: hexadecimal, rgb(), hsl() or oklch(). */
    static readonly schema = schema.string().refine((text) => Color.read(text) !== undefined, {
        message: "a color is hexadecimal, rgb(), hsl() or oklch()",
    });

    /** The space. */
    readonly space: ColorSpace;
    /** The numbers, in the order of the space's channels. */
    readonly values: ColorValues;
    /** How much it covers what is behind it, from 0 to 1. */
    readonly alpha: number;

    /** Say a color in a space. */
    constructor(space: ColorSpace, values: ColorValues, alpha = 1) {
        this.space = space;
        this.values = values;
        this.alpha = alpha;
    }

    /** Read a color CSS writes: hexadecimal, rgb(), hsl() or oklch(), refusing any other. */
    static parse(text: string): Color {
        const color = Color.read(text);
        if (color === undefined) {
            throw new TypeError(`color is invalid: '${text}'`);
        }

        return color;
    }

    /** Read a color CSS writes, undefined where it is none. */
    static read(text: string): Color | undefined {
        // read the hexadecimal digits, each twice in the short form
        const trimmed = text.trim();
        if (HEX.test(trimmed)) {
            const digits = trimmed.slice(1);
            const long = digits.length > 4 ? digits : digits.replaceAll(/./gu, "$&$&");
            const [red = 0, green = 0, blue = 0, alpha = 255] = Uint8Array.fromHex(long);

            return new Color("rgb", [red, green, blue], alpha / 255);
        }

        // read the numbers of a function
        const [, name = "", list = ""] = FUNCTION.exec(trimmed) ?? [];
        const space = (["rgb", "hsl", "oklch"] as const).find(
            (each) => each === name.toLowerCase(),
        );
        const parts = list.split(/[\s,/]+/u).filter((part) => part !== "");
        if (space === undefined || parts.length < 3 || parts.length > 4) {
            return undefined;
        }
        const channels: readonly ColorChannel[] = [...COLOR_CHANNELS[space], "alpha"];
        const numbers = parts.map((part, index) =>
            readNumber(part, channels[index] ?? "alpha", space),
        );
        const [first, second, third, alpha = 1] = numbers;
        if (first === undefined || second === undefined || third === undefined) {
            return undefined;
        }

        return numbers.includes(undefined)
            ? undefined
            : new Color(space, [first, second, third], alpha);
    }

    /** Read the values a channel takes. */
    static range(channel: ColorChannel): ColorChannelRange {
        return RANGES[channel];
    }

    /** Say the color in another space, the nearest a screen shows for one it does not. */
    to(space: ColorSpace): Color {
        if (space === this.space) {
            return this;
        }

        return new Color(space, fromRgb(space, this.#rgb()), this.alpha);
    }

    /** Report whether another color says the same in the same space. */
    equals(other: Color): boolean {
        return (
            this.space === other.space &&
            this.alpha === other.alpha &&
            this.values.every((value, place) => value === other.values[place])
        );
    }

    /** Read a channel of the color's own space. */
    get(channel: ColorChannel): number {
        return channel === "alpha" ? this.alpha : (this.values[this.#place(channel)] ?? 0);
    }

    /** Change a channel of the color's own space, within its range. */
    with(channel: ColorChannel, value: number): Color {
        // replace the alpha or one of the numbers
        const { min, max } = RANGES[channel];
        const next = Math.min(Math.max(value, min), max);
        if (channel === "alpha") {
            return new Color(this.space, this.values, next);
        }
        const place = this.#place(channel);
        const [first, second, third] = this.values;

        return new Color(
            this.space,
            [place === 0 ? next : first, place === 1 ? next : second, place === 2 ? next : third],
            this.alpha,
        );
    }

    /** Say what people call the color, after React Aria. */
    name(): ColorName {
        // say black, white and gray by their lightness alone
        const [percent, chroma, hue] = this.to("oklch").values;
        const light = percent / 100;
        const isExtreme = light > 0.999 || light < 0.001;
        if (isExtreme) {
            const hues = [light > 0.5 ? ("white" as const) : ("black" as const)];

            return { lightness: undefined, chroma: undefined, hues };
        }

        // find the hue, brown where orange is dark, whose lightness starts further up
        const place = HUE_NAMES.findIndex((_, index) => {
            const next = HUE_NAMES[index + 1];

            return next !== undefined && hue < next[0];
        });
        const [start, first] = HUE_NAMES[place] ?? HUE_NAMES[0];
        const [end, second] = HUE_NAMES[place + 1] ?? HUE_NAMES[0];
        const isOrange = first === "orange";
        const isBrown = isOrange && light < BROWN_LIGHTNESS;
        const shown = isOrange && !isBrown ? light - BROWN_LIGHTNESS + DARK_LIGHTNESS : light;

        // add the next hue from halfway to it, and green to a dark yellow
        const named = isBrown ? "brown" : first;
        const isBetween = hue > start + (end - start) / 2 && named !== second;
        const isYellowGreen = first === "yellow" && light < YELLOW_GREEN_LIGHTNESS;
        const isGray = chroma < GRAY_CHROMA;
        const hues: readonly ColorHueName[] = isGray
            ? ["gray"]
            : isBetween
              ? [named, second]
              : isYellowGreen
                ? ["yellow", "green"]
                : [named];

        return {
            lightness:
                shown < 0.3
                    ? "very dark"
                    : shown < DARK_LIGHTNESS
                      ? "dark"
                      : shown < 0.7
                        ? undefined
                        : shown < 0.85
                          ? "light"
                          : "very light",
            chroma: isGray
                ? undefined
                : chroma <= 0.1
                  ? shown >= 0.7
                      ? "pale"
                      : "grayish"
                  : chroma >= 0.15
                    ? "vibrant"
                    : undefined,
            hues,
        };
    }

    /** Write the color as CSS reads it. */
    toString(format: ColorFormat = "hex"): string {
        // write the hexadecimal digits, the alpha's only where it is see-through
        if (format === "hex") {
            const rgb = this.#rgb().map(Math.round);
            const alpha = Math.round(this.alpha * 255);

            return `#${new Uint8Array(alpha === 255 ? rgb : [...rgb, alpha]).toHex()}`;
        }

        // write a function of the numbers
        const [first, second, third] = this.to(format).values;
        const alpha = this.alpha === 1 ? "" : ` / ${round(this.alpha, 2)}`;
        const numbers = {
            rgb: `${round(first, 0)} ${round(second, 0)} ${round(third, 0)}`,
            hsl: `${round(first, 1)} ${round(second, 1)}% ${round(third, 1)}%`,
            oklch: `${round(first, 2)}% ${round(second, 4)} ${round(third, 2)}`,
        };

        return `${format}(${numbers[format]}${alpha})`;
    }

    /** Read the red, green and blue a screen shows, from 0 to 255. */
    #rgb(): ColorValues {
        const [first, second, third] = this.values;

        return this.space === "rgb"
            ? this.values
            : this.space === "oklch"
              ? oklchToRgb(first, second, third)
              : this.space === "hsl"
                ? hslToRgb(first, second, third)
                : hsbToRgb(first, second, third);
    }

    /** Find the place of a channel in the color's own space, refusing one of another. */
    #place(channel: ColorChannel): number {
        const place = COLOR_CHANNELS[this.space].findIndex((each) => each === channel);
        if (place === -1) {
            throw new RangeError(`color has no channel ${channel}`);
        }

        return place;
    }
}

/** Read a number of a color function in a channel's range, undefined where it is none. */
function readNumber(text: string, channel: ColorChannel, space: ColorSpace): number | undefined {
    // read a number, a share of the channel, or none of it
    if (text.toLowerCase() === "none") {
        return 0;
    }
    const [, digits, unit] = NUMBER.exec(text) ?? [];
    if (digits === undefined || (unit === "deg" && channel !== "hue")) {
        return undefined;
    }
    const value = Number(digits);
    const { max } = RANGES[channel];
    const isShare = unit === "%" || (space === "oklch" && channel === "lightness" && value <= 1);
    const whole = unit === "%" ? 100 : 1;
    const scaled = isShare ? (value / whole) * max : value;

    // turn a hue past the circle back, and keep the others in range
    return channel === "hue" ? ((scaled % 360) + 360) % 360 : Math.min(Math.max(scaled, 0), max);
}

/** Write a number to some decimals, without the zeros at its end. */
function round(value: number, decimals: number): string {
    return String(Number(value.toFixed(decimals)));
}

/** Say red, green and blue in a space. */
function fromRgb(space: ColorSpace, values: ColorValues): ColorValues {
    // read the brightest and the darkest channel, and the hue between them
    if (space === "rgb") {
        return values;
    }
    if (space === "oklch") {
        return rgbToOklch(values);
    }
    const [red, green, blue] = [values[0] / 255, values[1] / 255, values[2] / 255];
    const most = Math.max(red, green, blue);
    const least = Math.min(red, green, blue);
    const spread = most - least;
    const turn =
        spread === 0
            ? 0
            : most === red
              ? ((green - blue) / spread + 6) % 6
              : most === green
                ? (blue - red) / spread + 2
                : (red - green) / spread + 4;

    // say how strong and how light it is
    const hue = turn * 60;
    const lightness = (most + least) / 2;
    const hslSaturation = spread === 0 ? 0 : spread / (1 - Math.abs(2 * lightness - 1));

    return space === "hsl"
        ? [hue, hslSaturation * 100, lightness * 100]
        : [hue, most === 0 ? 0 : (spread / most) * 100, most * 100];
}

/** Say a hue, a saturation and a lightness in red, green and blue. */
function hslToRgb(hue: number, saturation: number, lightness: number): ColorValues {
    // reach from the lightness as far as the saturation says
    const light = lightness / 100;
    const reach = (saturation / 100) * Math.min(light, 1 - light);
    const channel = (offset: number): number => {
        // follow the hue around the circle
        const turn = (offset + hue / 30) % 12;

        return 255 * (light - reach * Math.max(-1, Math.min(turn - 3, 9 - turn, 1)));
    };

    return [channel(0), channel(8), channel(4)];
}

/** Say a hue, a saturation and a brightness in red, green and blue. */
function hsbToRgb(hue: number, saturation: number, brightness: number): ColorValues {
    const channel = (offset: number): number => {
        // follow the hue around the circle
        const turn = (offset + hue / 60) % 6;
        const share = Math.max(0, Math.min(turn, 4 - turn, 1));

        return 2.55 * brightness * (1 - (saturation / 100) * share);
    };

    return [channel(5), channel(3), channel(1)];
}

/** Say red, green and blue as people see them, by Björn Ottosson's OKLab. */
function rgbToOklch(values: ColorValues): ColorValues {
    // read the light of the three cones
    const [red, green, blue] = [linear(values[0]), linear(values[1]), linear(values[2])];
    const long = Math.cbrt(0.4122214708 * red + 0.5363325363 * green + 0.0514459929 * blue);
    const medium = Math.cbrt(0.2119034982 * red + 0.6806995451 * green + 0.1073969566 * blue);
    const short = Math.cbrt(0.0883024619 * red + 0.2817188376 * green + 0.6299787005 * blue);

    // turn them to a lightness and two opposed colors
    const lightness = 0.2104542553 * long + 0.793617785 * medium - 0.0040720468 * short;
    const redness = 1.9779984951 * long - 2.428592205 * medium + 0.4505937099 * short;
    const blueness = 0.0259040371 * long + 0.7827717662 * medium - 0.808675766 * short;
    const chroma = Math.hypot(redness, blueness);
    const isGray = chroma < GAMUT_TOLERANCE;
    const hue = isGray ? 0 : ((Math.atan2(blueness, redness) * 180) / Math.PI + 360) % 360;

    return [lightness * 100, isGray ? 0 : chroma, hue];
}

/** Say a lightness, a chroma and a hue in red, green and blue, as vivid as a screen shows it. */
function oklchToRgb(lightness: number, chroma: number, hue: number): ColorValues {
    // halve the chroma's range until it is the most a screen shows
    let shown = oklabToRgb(lightness, chroma, hue);
    if (!isInGamut(shown)) {
        let low = 0;
        let high = chroma;
        for (let step = 0; step < GAMUT_STEPS; step += 1) {
            const middle = (low + high) / 2;
            const isShown = isInGamut(oklabToRgb(lightness, middle, hue));
            low = isShown ? middle : low;
            high = isShown ? high : middle;
        }
        shown = oklabToRgb(lightness, low, hue);
    }

    return [encoded(shown[0]), encoded(shown[1]), encoded(shown[2])];
}

/** Say a lightness, a chroma and a hue in the light of red, green and blue, a screen's or not. */
function oklabToRgb(lightness: number, chroma: number, hue: number): ColorValues {
    // turn to the light of the three cones
    const light = lightness / 100;
    const redness = chroma * Math.cos((hue * Math.PI) / 180);
    const blueness = chroma * Math.sin((hue * Math.PI) / 180);
    const long = (light + 0.3963377774 * redness + 0.2158037573 * blueness) ** 3;
    const medium = (light - 0.1055613458 * redness - 0.0638541728 * blueness) ** 3;
    const short = (light - 0.0894841775 * redness - 1.291485548 * blueness) ** 3;

    return [
        4.0767416621 * long - 3.3077115913 * medium + 0.2309699292 * short,
        -1.2684380046 * long + 2.6097574011 * medium - 0.3413193965 * short,
        -0.0041960863 * long - 0.7034186147 * medium + 1.707614701 * short,
    ];
}

/** Report whether a screen shows a light of red, green and blue. */
function isInGamut(values: ColorValues): boolean {
    return values.every((value) => value >= -GAMUT_TOLERANCE && value <= 1 + GAMUT_TOLERANCE);
}

/** Read the light of a channel from 0 to 255, by sRGB's curve. */
function linear(value: number): number {
    const share = value / 255;

    return share <= 0.04045 ? share / 12.92 : ((share + 0.055) / 1.055) ** 2.4;
}

/** Write a light as a channel from 0 to 255, by sRGB's curve. */
function encoded(light: number): number {
    const share = Math.min(Math.max(light, 0), 1);

    return 255 * (share <= 0.0031308 ? share * 12.92 : 1.055 * share ** (1 / 2.4) - 0.055);
}
