import { type Message, t } from "@destack/locale";
import { schema } from "@destack/schema";
import { type Bindable, Reference, type Resolve } from "../reference/index.ts";

/** The channels of each color space, in the order CSS writes them. */
export const COLOR_CHANNELS = {
    rgb: ["red", "green", "blue"],
    hsl: ["hue", "saturation", "lightness"],
    hsb: ["hue", "saturation", "brightness"],
    hwb: ["hue", "whiteness", "blackness"],
    lab: ["lightness", "greenRed", "blueYellow"],
    lch: ["lightness", "chroma", "hue"],
    oklab: ["lightness", "greenRed", "blueYellow"],
    oklch: ["lightness", "chroma", "hue"],
    "srgb-linear": ["red", "green", "blue"],
    "display-p3": ["red", "green", "blue"],
    "a98-rgb": ["red", "green", "blue"],
    "prophoto-rgb": ["red", "green", "blue"],
    rec2020: ["red", "green", "blue"],
    "xyz-d50": ["x", "y", "z"],
    "xyz-d65": ["x", "y", "z"],
} as const;

/** A way to say a color in three numbers, as CSS Color 4 and design tools name them. */
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
        hwb: {
            title: t.context("option")`HWB`,
            description: t`By its hue, and how much white and black it mixes in.`,
        },
        lab: {
            title: t.context("option")`Lab`,
            description: t`By its lightness and two opposed colors, as CIE measures them.`,
        },
        lch: {
            title: t.context("option")`LCH`,
            description: t`By its lightness, chroma and hue, as CIE measures them.`,
        },
        oklab: {
            title: t.context("option")`OKLab`,
            description: t`By its lightness and two opposed colors, as people see them.`,
        },
        oklch: {
            title: t.context("option")`OKLCH`,
            description: t`By its lightness, chroma and hue, as people see it.`,
        },
        "srgb-linear": {
            title: t.context("option")`Linear sRGB`,
            description: t`By its red, green and blue light, before a screen's curve.`,
        },
        "display-p3": {
            title: t.context("option")`Display P3`,
            description: t`By the red, green and blue of a wide gamut screen.`,
        },
        "a98-rgb": {
            title: t.context("option")`Adobe RGB`,
            description: t`By the red, green and blue of the 1998 print gamut.`,
        },
        "prophoto-rgb": {
            title: t.context("option")`ProPhoto RGB`,
            description: t`By the red, green and blue of a photo gamut wider than the eye's.`,
        },
        rec2020: {
            title: t.context("option")`Rec. 2020`,
            description: t`By the red, green and blue of ultra high definition video.`,
        },
        "xyz-d50": {
            title: t.context("option")`XYZ D50`,
            description: t`By the light the eye's cones take in, under printed paper's white.`,
        },
        "xyz-d65": {
            title: t.context("option")`XYZ D65`,
            description: t`By the light the eye's cones take in, under daylight's white.`,
        },
    } satisfies Record<ColorSpace, { readonly title: Message; readonly description: Message }>,
};

/** One number of a color. */
export type ColorChannel = (typeof COLOR_CHANNELS)[ColorSpace][number] | "alpha";

/** The way a color is written in CSS: hexadecimal, or a function of any space but HSB. */
export type ColorFormat = "hex" | Exclude<ColorSpace, "hsb">;

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

/** The red, green and blue of a screen's light from 0 to 1, the shares predefined spaces take. */
const SHARES = [
    { min: 0, max: 1, step: 0.001 },
    { min: 0, max: 1, step: 0.001 },
    { min: 0, max: 1, step: 0.001 },
] as const;

/** A hue in degrees around the circle. */
const HUE = { min: 0, max: 360, step: 1 } as const;

/** A percentage. */
const PERCENT = { min: 0, max: 100, step: 1 } as const;

/** The values each channel of each space takes, its max also what 100% means, as CSS reads it. */
const RANGES: Readonly<
    Record<ColorSpace, readonly [ColorChannelRange, ColorChannelRange, ColorChannelRange]>
> = {
    rgb: [
        { min: 0, max: 255, step: 1 },
        { min: 0, max: 255, step: 1 },
        { min: 0, max: 255, step: 1 },
    ],
    hsl: [HUE, PERCENT, PERCENT],
    hsb: [HUE, PERCENT, PERCENT],
    hwb: [HUE, PERCENT, PERCENT],
    lab: [PERCENT, { min: -125, max: 125, step: 1 }, { min: -125, max: 125, step: 1 }],
    lch: [PERCENT, { min: 0, max: 150, step: 1 }, HUE],
    oklab: [PERCENT, { min: -0.4, max: 0.4, step: 0.005 }, { min: -0.4, max: 0.4, step: 0.005 }],
    // the most vivid color a screen of P3 shows is below 0.4
    oklch: [PERCENT, { min: 0, max: 0.4, step: 0.005 }, HUE],
    "srgb-linear": SHARES,
    "display-p3": SHARES,
    "a98-rgb": SHARES,
    "prophoto-rgb": SHARES,
    rec2020: SHARES,
    "xyz-d50": SHARES,
    "xyz-d65": SHARES,
};

/** The values an alpha takes. */
const ALPHA: ColorChannelRange = { min: 0, max: 1, step: 0.01 };

/** The spaces of sRGB, which CSS clamps to their ranges and a color maps into first. */
const SRGB_SPACES: ReadonlySet<ColorSpace> = new Set(["rgb", "hsl", "hsb", "hwb"]);

/** The predefined spaces `color()` reads, sRGB's as RGB. */
const PREDEFINED = [
    "srgb",
    "srgb-linear",
    "display-p3",
    "a98-rgb",
    "prophoto-rgb",
    "rec2020",
    "xyz-d50",
    "xyz-d65",
    "xyz",
] as const;

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
export const GRAY_CHROMA = 0.001;

/** The lightness below which orange is brown. */
const BROWN_LIGHTNESS = 0.68;

/** The lightness below which yellow is yellow green. */
const YELLOW_GREEN_LIGHTNESS = 0.85;

/** The lightness below which a color is dark. */
const DARK_LIGHTNESS = 0.55;

/** A color's hexadecimal digits, one or two for each channel. */
const HEX = /^#(?:[\da-f]{3,4}|[\da-f]{6}|[\da-f]{8})$/iu;

/** A color function of CSS and its arguments. */
const FUNCTION = /^(rgba?|hsla?|hwb|lab|lch|oklab|oklch|color)\(([^()]+)\)$/iu;

/** A number of a color function, with the unit it may have. */
const NUMBER = /^([+-]?(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?)(%|deg)?$/iu;

/** A number of a color function as a pattern, in any case as CSS reads it. */
const NUMBER_PATTERN = String.raw`[+-]?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?`;

/** A channel of a color function: none, or a number or a share of its range. */
const CHANNEL_PATTERN = `(?:[nN][oO][nN][eE]|${NUMBER_PATTERN}%?)`;

/** A hue of a color function: none, or a number, a share of the circle or degrees. */
const HUE_PATTERN = `(?:[nN][oO][nN][eE]|${NUMBER_PATTERN}(?:%|[dD][eE][gG])?)`;

/** The separators between the numbers of a color function. */
const SEPARATOR_PATTERN = String.raw`[\s,/]+`;

/** Write the arguments of a color function: three channels, then an alpha. */
function argumentsPattern(first: string, second: string, third: string): string {
    const channels = [first, second, third].join(SEPARATOR_PATTERN);

    return String.raw`\(\s*${channels}(?:${SEPARATOR_PATTERN}${CHANNEL_PATTERN})?\s*\)`;
}

/** The predefined spaces `color()` reads, to tell them from the functions. */
const PREDEFINED_SPACES: ReadonlySet<string> = new Set(PREDEFINED);

/** Every color space, in the order of their channels' table. */
const COLOR_SPACES = [
    "rgb",
    "hsl",
    "hsb",
    "hwb",
    "lab",
    "lch",
    "oklab",
    "oklch",
    "srgb-linear",
    "display-p3",
    "a98-rgb",
    "prophoto-rgb",
    "rec2020",
    "xyz-d50",
    "xyz-d65",
] as const satisfies readonly ColorSpace[];

/** The predefined spaces of `color()` as a pattern, in any case as CSS reads them. */
const PREDEFINED_PATTERN = `(?:${PREDEFINED.map((name) => name.replaceAll(/[a-z]/gu, (letter) => `[${letter}${letter.toUpperCase()}]`)).join("|")})`;

/** A color CSS writes, as `Color.read` takes it: hexadecimal, or a function of CSS Color 4. */
const CSS_COLOR = new RegExp(
    String.raw`^\s*(?:` +
        [
            String.raw`#(?:[\da-fA-F]{3,4}|[\da-fA-F]{6}|[\da-fA-F]{8})`,
            `[rR][gG][bB][aA]?${argumentsPattern(CHANNEL_PATTERN, CHANNEL_PATTERN, CHANNEL_PATTERN)}`,
            `[hH][sS][lL][aA]?${argumentsPattern(HUE_PATTERN, CHANNEL_PATTERN, CHANNEL_PATTERN)}`,
            `[hH][wW][bB]${argumentsPattern(HUE_PATTERN, CHANNEL_PATTERN, CHANNEL_PATTERN)}`,
            `[lL][aA][bB]${argumentsPattern(CHANNEL_PATTERN, CHANNEL_PATTERN, CHANNEL_PATTERN)}`,
            `[lL][cC][hH]${argumentsPattern(CHANNEL_PATTERN, CHANNEL_PATTERN, HUE_PATTERN)}`,
            `[oO][kK][lL][aA][bB]${argumentsPattern(CHANNEL_PATTERN, CHANNEL_PATTERN, CHANNEL_PATTERN)}`,
            `[oO][kK][lL][cC][hH]${argumentsPattern(CHANNEL_PATTERN, CHANNEL_PATTERN, HUE_PATTERN)}`,
            String.raw`[cC][oO][lL][oO][rR]\(\s*${PREDEFINED_PATTERN}\s+${CHANNEL_PATTERN}${SEPARATOR_PATTERN}${CHANNEL_PATTERN}${SEPARATOR_PATTERN}${CHANNEL_PATTERN}(?:${SEPARATOR_PATTERN}${CHANNEL_PATTERN})?\s*\)`,
        ].join("|") +
        String.raw`)\s*$`,
    "u",
);

/** The most a color moves in OKLab as hexadecimal rounds each channel to 1/255, and some room. */
const HEX_ROUNDING = 0.005;

/** The room a channel may leave its range by before a color is out of a screen's. */
const GAMUT_TOLERANCE = 0.0001;

/** The halvings that find the chroma a screen shows, a step below a millionth. */
const GAMUT_STEPS = 20;

/** The decimal places of a token's sRGB numbers, a millionth, far below one 8-bit step. */
const TOKEN_PRECISION = 6;

/** The sRGB channel exponent of APCA-W3 0.0.98G. */
const MAIN_TRC = 2.4;

/** The red, green and blue luminance coefficients of APCA-W3 0.0.98G. */
const SCREEN_LUMINANCE = [0.2126729, 0.7151522, 0.072175] as const;

/** The background exponent for dark text on a light background, APCA-W3 0.0.98G `normBG`. */
const NORMAL_BACKGROUND = 0.56;

/** The text exponent for dark text on a light background, APCA-W3 0.0.98G `normTXT`. */
const NORMAL_TEXT = 0.57;

/** The text exponent for light text on a dark background, APCA-W3 0.0.98G `revTXT`. */
const REVERSE_TEXT = 0.62;

/** The background exponent for light text on a dark background, APCA-W3 0.0.98G `revBG`. */
const REVERSE_BACKGROUND = 0.65;

/** The luminance below which APCA soft-clamps near-black, APCA-W3 0.0.98G `blkThrs`. */
const BLACK_THRESHOLD = 0.022;

/** The exponent of the near-black soft clamp, APCA-W3 0.0.98G `blkClmp` of 1.414. */
const BLACK_CLAMP = 1414 / 1000;

/** The output scale of both polarities, APCA-W3 0.0.98G `scaleBoW` and `scaleWoB`. */
const SCALE = 1.14;

/** The output offset of both polarities, APCA-W3 0.0.98G `loBoWoffset` and `loWoBoffset`. */
const OFFSET = 0.027;

/** The output magnitude below which contrast reads as zero, APCA-W3 0.0.98G `loClip`. */
const LOW_CLIP = 0.1;

/** The luminance difference below which contrast reads as zero, APCA-W3 0.0.98G `deltaYmin`. */
const DELTA_MIN = 0.0005;

/** The WCAG 2.2 relative luminance coefficients of red, green and blue. */
const RELATIVE_LUMINANCE = [0.2126, 0.7152, 0.0722] as const;

/** The flare WCAG 2.2 adds to both luminances of a contrast ratio. */
const FLARE = 0.05;

/** A six-digit sRGB hex color such as `#3e63dd`, in either letter case. */
export const HexColor = schema.string().regex(/^#[\da-f]{6}$/iu);

/** The luminances of a color the contrast measures take. */
interface Luminances {
    /** The screen luminance APCA-W3 0.0.98G estimates, from 0 to 1. */
    readonly screen: number;
    /** The relative luminance WCAG 2.2 defines, from 0 to 1. */
    readonly relative: number;
}

/** A measure of contrast: APCA-W3's lightness contrast Lc, or WCAG 2.2's ratio. */
export type ContrastMeasure = "apca" | "wcag";

/** The color spaces of the design token format, each of CSS Color 4. */
export const TokenColorSpace = schema.enum([
    "srgb",
    "srgb-linear",
    "hsl",
    "hwb",
    "lab",
    "lch",
    "oklab",
    "oklch",
    "display-p3",
    "a98-rgb",
    "prophoto-rgb",
    "rec2020",
    "xyz-d50",
    "xyz-d65",
]);
/** A color space of the design token format. */
export type TokenColorSpace = schema.Infer<typeof TokenColorSpace>;

/** A color as the design token format writes one: its space, three numbers, alpha and hex. */
export const ColorToken = schema.object({
    /** The space. */
    colorSpace: TokenColorSpace,
    /** The numbers, as the format scales them: RGB from 0 to 1, and OK lightness too. */
    components: schema.tuple([schema.number(), schema.number(), schema.number()]),
    /** How much it covers what is behind it, from 0 to 1, opaque where absent. */
    alpha: schema.number().min(0).max(1).exactOptional(),
    /** The six-digit hexadecimal form, which an sRGB color's numbers match. */
    hex: HexColor.exactOptional(),
});
/** A color as the design token format writes one. */
export type ColorToken = schema.Infer<typeof ColorToken>;

/** A color in one space, which another has the nearest color of. */
export class Color {
    /** A color CSS writes as text: hexadecimal, rgb(), hsl() or oklch(). */
    static readonly schema = schema
        .string()
        .regex(CSS_COLOR, { message: "a color is hexadecimal, rgb(), hsl() or oklch()" })
        .meta({ id: "style.color", title: "Color" });

    /** A color, or a reference to a color token in its place. */
    static readonly bindable = Reference.or(Color.schema, "color");

    /** Parse a color as CSS writes it, a reference's as its token is drawn. */
    static resolve(color: Bindable<string>, resolve: Resolve): Color {
        return Color.parse(Reference.is(color) ? resolve(color) : color);
    }

    /** Write a color as CSS reads it, a reference as its variable. */
    static css(color: Bindable<string>): string {
        return Reference.write(color, (written) => written);
    }

    /** The space. */
    readonly space: ColorSpace;
    /** The numbers, in the order of the space's channels. */
    readonly values: ColorValues;
    /** How much it covers what is behind it, from 0 to 1. */
    readonly alpha: number;
    /** The luminances contrast measures take, computed on the first measure. */
    #luminance: Luminances | undefined = undefined;

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

        // read the space of a function, and of color() its first word
        const [, name = "", list = ""] = FUNCTION.exec(trimmed) ?? [];
        const lower = name.toLowerCase().replace(/a$/u, "");
        const words = list.split(/[\s,/]+/u).filter((part) => part !== "");
        const named = lower === "color" ? words.shift()?.toLowerCase() : lower;
        const space = spaceOf(named);
        if (space === undefined || words.length < 3 || words.length > 4) {
            return undefined;
        }

        // read each number in its channel's range, sRGB's shares of color() as bytes
        const isShared = named === "srgb";
        const numbers = words.map((word, place) => readNumber(word, space, place, isShared));
        const [first, second, third, alpha = 1] = numbers;
        if (first === undefined || second === undefined || third === undefined) {
            return undefined;
        }

        return numbers.includes(undefined)
            ? undefined
            : new Color(space, [first, second, third], alpha);
    }

    /** Read a color the design token format writes. */
    static fromToken(token: ColorToken): Color {
        // scale sRGB to bytes and OK lightness to a percentage, as the color stores them
        const [first, second, third] = token.components;
        const alpha = token.alpha ?? 1;
        const space = token.colorSpace === "srgb" ? "rgb" : token.colorSpace;
        const values: ColorValues =
            space === "rgb"
                ? [first * 255, second * 255, third * 255]
                : space === "oklab" || space === "oklch"
                  ? [first * 100, second, third]
                  : [first, second, third];

        return new Color(space, values, alpha);
    }

    /** Read the values a channel of the color's space takes, its max also what 100% means. */
    range(channel: ColorChannel): ColorChannelRange {
        return channel === "alpha" ? ALPHA : (RANGES[this.space][this.#place(channel)] ?? ALPHA);
    }

    /** Say the color in another space, mapped into sRGB's gamut for a space of sRGB. */
    to(space: ColorSpace): Color {
        // say a space of sRGB from what a screen shows, and any other exactly through XYZ
        if (space === this.space) {
            return this;
        } else if (SRGB_SPACES.has(space)) {
            return new Color(space, fromRgb(space, this.#rgb()), this.alpha);
        } else if (space === "oklch" && this.space === "rgb") {
            return new Color(space, rgbToOklch(this.values), this.alpha);
        } else {
            return new Color(space, fromXyz(space, this.#xyz()), this.alpha);
        }
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
        const { min, max } = this.range(channel);
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

    /** Measure the color's contrast as text on a background, after Color.js's `contrast`. */
    contrast(background: Color, measure: ContrastMeasure): number {
        return measure === "apca"
            ? lightnessContrast(this.#luminances().screen, background.#luminances().screen)
            : contrastRatio(this.#luminances().relative, background.#luminances().relative);
    }

    /** Blend the color over an opaque one, as CSS composites in gamma-encoded sRGB. */
    composite(under: Color): Color {
        // weigh each channel by the color's alpha
        const over = this.#rgb();
        const ground = under.#rgb();
        const blend = (index: 0 | 1 | 2): number =>
            over[index] * this.alpha + ground[index] * (1 - this.alpha);

        return new Color("rgb", [blend(0), blend(1), blend(2)]);
    }

    /** Measure the OKLab distance to a color, by lightness, chroma and the angle between. */
    distance(other: Color): number {
        // read both in OKLCH, the lightness as a share
        const [lightness, chroma, hue] = this.to("oklch").values;
        const [otherLightness, otherChroma, otherHue] = other.to("oklch").values;
        const angle = ((hue - otherHue) * Math.PI) / 180;
        const chromas = chroma ** 2 + otherChroma ** 2 - 2 * chroma * otherChroma * Math.cos(angle);

        return Math.sqrt(((lightness - otherLightness) / 100) ** 2 + Math.max(0, chromas));
    }

    /** Write the color as the design token format does, a color of HSB in sRGB. */
    token(): ColorToken {
        // write sRGB and HSB as sRGB shares with the hex they match
        const alpha = this.alpha === 1 ? {} : { alpha: this.alpha };
        const [first, second, third] = this.values;
        if (this.space === "rgb" || this.space === "hsb") {
            const [red, green, blue] = this.#rgb();
            const hex = new Color("rgb", [red, green, blue]).toString("hex");
            const share = (byte: number): number => Number((byte / 255).toFixed(TOKEN_PRECISION));
            const components: [number, number, number] = [share(red), share(green), share(blue)];

            return { colorSpace: "srgb", components, ...alpha, hex };
        }
        // write OK lightness as a share, and every other space in its numbers
        else {
            const isOk = this.space === "oklab" || this.space === "oklch";
            const components: [number, number, number] = [
                isOk ? first / 100 : first,
                second,
                third,
            ];

            return { colorSpace: this.space, components, ...alpha };
        }
    }

    /** Write the color as plainly as CSS reads it: hexadecimal where sRGB shows it, else OKLCH. */
    plain(): string {
        // keep hexadecimal while it loses no more than its own rounding
        const hex = this.toString("hex");

        return Color.parse(hex).distance(this) <= HEX_ROUNDING ? hex : this.toString("oklch");
    }

    /** Write the color as CSS reads it. */
    toString(format: ColorFormat = "hex"): string {
        // write the hexadecimal digits, the alpha's only where it is see-through
        if (format === "hex") {
            const rgb = this.#rgb().map(Math.round);
            const alpha = Math.round(this.alpha * 255);

            return `#${new Uint8Array(alpha === 255 ? rgb : [...rgb, alpha]).toHex()}`;
        }

        // write a function of the numbers, color() for a predefined space
        const [first, second, third] = this.to(format).values;
        const alpha = this.alpha === 1 ? "" : ` / ${round(this.alpha, 2)}`;
        const percent = (value: number) => `${round(value, 2)}%`;
        const numbers =
            format === "rgb"
                ? `${round(first, 0)} ${round(second, 0)} ${round(third, 0)}`
                : format === "hsl" || format === "hwb"
                  ? `${round(first, 1)} ${round(second, 1)}% ${round(third, 1)}%`
                  : format === "oklch"
                    ? `${percent(first)} ${round(second, 4)} ${round(third, 2)}`
                    : format === "oklab"
                      ? `${percent(first)} ${round(second, 4)} ${round(third, 4)}`
                      : format === "lab" || format === "lch"
                        ? `${percent(first)} ${round(second, 2)} ${round(third, 2)}`
                        : `${format} ${round(first, 5)} ${round(second, 5)} ${round(third, 5)}`;
        const isFunction = !PREDEFINED_SPACES.has(format);

        return isFunction ? `${format}(${numbers}${alpha})` : `color(${numbers}${alpha})`;
    }

    /** Read the luminances contrast measures take, computed once as a color never changes. */
    #luminances(): Luminances {
        // read the gamma-encoded channels from 0 to 1, then weigh them as each measure does
        if (this.#luminance === undefined) {
            const [red, green, blue] = this.#rgb();
            const channels: ColorValues = [red / 255, green / 255, blue / 255];
            this.#luminance = {
                screen: screenLuminance(channels),
                relative: relativeLuminance(channels),
            };
        }

        return this.#luminance;
    }

    /** Read the red, green and blue a screen shows, from 0 to 255, as vivid as sRGB shows it. */
    #rgb(): ColorValues {
        // read the spaces of sRGB directly, and map any other through OKLCH into sRGB's gamut
        const [first, second, third] = this.values;
        if (this.space === "rgb") {
            return this.values;
        } else if (this.space === "oklch") {
            return oklchToRgb(first, second, third);
        } else if (this.space === "hsl") {
            return hslToRgb(first, second, third);
        } else if (this.space === "hsb") {
            return hsbToRgb(first, second, third);
        } else if (this.space === "hwb") {
            return hwbToRgb(first, second, third);
        } else {
            const [lightness, chroma, hue] = fromXyz("oklch", this.#xyz());

            return oklchToRgb(lightness, chroma, hue);
        }
    }

    /** Read the light the color gives, as XYZ under daylight's white. */
    #xyz(): ColorValues {
        // read sRGB's spaces by their light, and every other by its curve and primaries
        const [first, second, third] = this.values;
        if (SRGB_SPACES.has(this.space)) {
            const [red, green, blue] = this.space === "rgb" ? this.values : this.#rgb();

            return multiply(XYZ_FROM_SRGB, [linear(red), linear(green), linear(blue)]);
        }

        return toXyz(this.space, [first, second, third]);
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

/** Read a number of a color function in its channel's range, undefined where it is none. */
function readNumber(
    text: string,
    space: ColorSpace,
    place: number,
    isShared: boolean,
): number | undefined {
    // read a number, a share of the channel, or none of it
    if (text.toLowerCase() === "none") {
        return 0;
    }
    const channel = place === 3 ? "alpha" : COLOR_CHANNELS[space][place];
    const range = place === 3 ? ALPHA : RANGES[space][place];
    const [, digits, unit] = NUMBER.exec(text) ?? [];
    if (digits === undefined || channel === undefined || range === undefined) {
        return undefined;
    } else if (unit === "deg" && channel !== "hue") {
        return undefined;
    }

    // scale a percentage by the range, OK lightness and sRGB's color() shares by theirs
    const value = Number(digits);
    const isOkLightness = (space === "oklab" || space === "oklch") && channel === "lightness";
    const scaled =
        unit === "%"
            ? (value / 100) * range.max
            : isOkLightness || (isShared && channel !== "alpha")
              ? value * range.max
              : value;

    // turn a hue past the circle back, clamp sRGB numbers to their range, and lightness to 0
    if (channel === "hue") {
        return ((scaled % 360) + 360) % 360;
    } else if (SRGB_SPACES.has(space) || channel === "alpha") {
        return Math.min(Math.max(scaled, range.min), range.max);
    } else if (channel === "lightness" || channel === "chroma") {
        return Math.max(scaled, 0);
    } else {
        return scaled;
    }
}

/** Read the space a color function names, undefined for none. */
function spaceOf(name: string | undefined): ColorSpace | undefined {
    const space = name === "srgb" ? "rgb" : name === "xyz" ? "xyz-d65" : name;

    return COLOR_SPACES.find((each) => each === space);
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
    if (!SRGB_SPACES.has(space)) {
        return fromXyz(
            space,
            multiply(XYZ_FROM_SRGB, [linear(values[0]), linear(values[1]), linear(values[2])]),
        );
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

    // say how much white and black it mixes in, as HWB does
    return space === "hsl"
        ? [hue, hslSaturation * 100, lightness * 100]
        : space === "hwb"
          ? [hue, least * 100, (1 - most) * 100]
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

/** Estimate a screen's luminance of gamma-encoded channels, as APCA-W3 0.0.98G does. */
function screenLuminance(channels: ColorValues): number {
    const [red, green, blue] = channels;
    const [redShare, greenShare, blueShare] = SCREEN_LUMINANCE;

    return (
        redShare * red ** MAIN_TRC + greenShare * green ** MAIN_TRC + blueShare * blue ** MAIN_TRC
    );
}

/** Compute the relative luminance of gamma-encoded channels, as WCAG 2.2 defines it. */
function relativeLuminance(channels: ColorValues): number {
    const [red, green, blue] = channels;
    const [redShare, greenShare, blueShare] = RELATIVE_LUMINANCE;

    return (
        redShare * linear(red * 255) +
        greenShare * linear(green * 255) +
        blueShare * linear(blue * 255)
    );
}

/** Measure the APCA-W3 0.0.98G lightness contrast Lc of text on a background by luminance. */
function lightnessContrast(foreground: number, background: number): number {
    // soft-clamp both luminances near black
    const text = softClamp(foreground);
    const ground = softClamp(background);
    if (Math.abs(ground - text) < DELTA_MIN) {
        return 0;
    }

    // measure dark text on a light background
    if (ground > text) {
        const contrast = (ground ** NORMAL_BACKGROUND - text ** NORMAL_TEXT) * SCALE;

        return contrast < LOW_CLIP ? 0 : (contrast - OFFSET) * 100;
    }
    // measure light text on a dark background
    else {
        const contrast = (ground ** REVERSE_BACKGROUND - text ** REVERSE_TEXT) * SCALE;

        return contrast > -LOW_CLIP ? 0 : (contrast + OFFSET) * 100;
    }
}

/** Soft-clamp a luminance near black, as APCA-W3 0.0.98G does. */
function softClamp(value: number): number {
    return value > BLACK_THRESHOLD ? value : value + (BLACK_THRESHOLD - value) ** BLACK_CLAMP;
}

/** Measure the WCAG 2.2 contrast ratio of two relative luminances, from 1 to 21. */
function contrastRatio(first: number, second: number): number {
    return (Math.max(first, second) + FLARE) / (Math.min(first, second) + FLARE);
}

/** A 3x3 matrix that turns three numbers into three others. */
type Matrix = readonly [ColorValues, ColorValues, ColorValues];

/** Linear sRGB to XYZ under daylight's white, from CSS Color 4. */
const XYZ_FROM_SRGB: Matrix = [
    [0.41239079926595934, 0.357584339383878, 0.1804807884018343],
    [0.21263900587151027, 0.715168678767756, 0.07219231536073371],
    [0.01933081871559182, 0.11919477979462598, 0.9505321522496607],
];

/** XYZ under daylight's white to linear sRGB, from CSS Color 4. */
const SRGB_FROM_XYZ: Matrix = [
    [3.2409699419045226, -1.537383177570094, -0.4986107602930034],
    [-0.9692436362808796, 1.8759675015077202, 0.04155505740717559],
    [0.05563007969699366, -0.20397695888897652, 1.0569715142428786],
];

/** Linear Display P3 to XYZ under daylight's white, from CSS Color 4. */
const XYZ_FROM_P3: Matrix = [
    [0.4865709486482162, 0.26566769316909306, 0.1982172852343625],
    [0.2289745640697488, 0.6917385218365064, 0.079286914093745],
    [0, 0.04511338185890264, 1.043944368900976],
];

/** XYZ under daylight's white to linear Display P3, from CSS Color 4. */
const P3_FROM_XYZ: Matrix = [
    [2.493496911941425, -0.9313836179191239, -0.40271078445071684],
    [-0.8294889695615747, 1.7626640603183463, 0.023624685841943577],
    [0.03584583024378447, -0.07617238926804182, 0.9568845240076872],
];

/** Linear Adobe RGB to XYZ under daylight's white, from CSS Color 4. */
const XYZ_FROM_A98: Matrix = [
    [0.5766690429101305, 0.1855582379065463, 0.1882286462349947],
    [0.29734497525053605, 0.6273635662554661, 0.07529145849399788],
    [0.02703136138641234, 0.07068885253582723, 0.9913375368376388],
];

/** XYZ under daylight's white to linear Adobe RGB, from CSS Color 4. */
const A98_FROM_XYZ: Matrix = [
    [2.0415879038107465, -0.5650069742788596, -0.34473135077832956],
    [-0.9692436362808795, 1.8759675015077202, 0.04155505740717557],
    [0.013444280632031142, -0.11836239223101838, 1.0151749943912054],
];

/** Linear ProPhoto RGB to XYZ under printed paper's white, from CSS Color 4. */
const XYZ50_FROM_PROPHOTO: Matrix = [
    [0.7977604896723027, 0.13518583717574031, 0.0313493495815248],
    [0.2880711282292934, 0.7118432178101014, 0.00008565396060525902],
    [0, 0, 0.8251046025104601],
];

/** XYZ under printed paper's white to linear ProPhoto RGB, from CSS Color 4. */
const PROPHOTO_FROM_XYZ50: Matrix = [
    [1.3457989731028281, -0.25558010007997534, -0.05110628506753401],
    [-0.5446224939028347, 1.5082327413132781, 0.02053603239147973],
    [0, 0, 1.2119675456389454],
];

/** Linear Rec. 2020 to XYZ under daylight's white, from CSS Color 4. */
const XYZ_FROM_REC2020: Matrix = [
    [0.6369580483012914, 0.14461690358620832, 0.1688809751641721],
    [0.2627002120112671, 0.6779980715188708, 0.05930171646986196],
    [0, 0.028072693049087428, 1.060985057710791],
];

/** XYZ under daylight's white to linear Rec. 2020, from CSS Color 4. */
const REC2020_FROM_XYZ: Matrix = [
    [1.7166511879712674, -0.35567078377639233, -0.25336628137365974],
    [-0.6666843518324892, 1.6164812366349395, 0.01576854581391113],
    [0.017639857445310783, -0.042770613257808524, 0.9421031212354738],
];

/** Bradford's adaptation from daylight's white to printed paper's, from CSS Color 4. */
const D50_FROM_D65: Matrix = [
    [1.0479297925449969, 0.022946870601609652, -0.05019226628920524],
    [0.02962780877005599, 0.9904344267538799, -0.017073799063418826],
    [-0.009243040646204504, 0.015055191490298152, 0.7518742814281371],
];

/** Bradford's adaptation from printed paper's white to daylight's, from CSS Color 4. */
const D65_FROM_D50: Matrix = [
    [0.955473421488075, -0.02309845494876471, 0.06325924320057072],
    [-0.0283697093338637, 1.0099953980813041, 0.021041441191917323],
    [0.012314014864481998, -0.020507649298898964, 1.330365926242124],
];

/** Printed paper's white, D50, as XYZ, from CSS Color 4. */
const D50_WHITE: ColorValues = [0.3457 / 0.3585, 1, (1 - 0.3457 - 0.3585) / 0.3585];

/** The CIE Lab constant κ, 24389/27, of CSS Color 4. */
const LAB_KAPPA = 24389 / 27;

/** The CIE Lab constant ε, 216/24389, of CSS Color 4. */
const LAB_EPSILON = 216 / 24389;

/** The exponent of Adobe RGB's curve, 563/256. */
const A98_GAMMA = 563 / 256;

/** The exponent of ProPhoto RGB's curve. */
const PROPHOTO_GAMMA = 1.8;

/** The linear part's end of ProPhoto RGB's curve, 16/512. */
const PROPHOTO_LINEAR = 16 / 512;

/** Rec. 2020's curve constant α, from ITU-R BT.2020. */
const REC2020_ALPHA = 1.09929682680944;

/** Rec. 2020's curve constant β, from ITU-R BT.2020. */
const REC2020_BETA = 0.018053968510807;

/** Multiply a matrix by three numbers. */
function multiply(matrix: Matrix, [x, y, z]: ColorValues): ColorValues {
    const [first, second, third] = matrix;

    return [
        first[0] * x + first[1] * y + first[2] * z,
        second[0] * x + second[1] * y + second[2] * z,
        third[0] * x + third[1] * y + third[2] * z,
    ];
}

/** Turn three numbers by a curve, keeping each sign, as CSS Color 4 extends curves below 0. */
function curve(values: ColorValues, turn: (magnitude: number) => number): ColorValues {
    const signed = (value: number) => Math.sign(value) * turn(Math.abs(value));

    return [signed(values[0]), signed(values[1]), signed(values[2])];
}

/** Read the light of sRGB's and Display P3's shares, by their shared curve. */
function srgbLinear(share: number): number {
    return share <= 0.04045 ? share / 12.92 : ((share + 0.055) / 1.055) ** 2.4;
}

/** Write a light as sRGB's and Display P3's shares, by their shared curve. */
function srgbEncoded(light: number): number {
    return light <= 0.0031308 ? light * 12.92 : 1.055 * light ** (1 / 2.4) - 0.055;
}

/** Read the light of Rec. 2020's shares. */
function rec2020Linear(share: number): number {
    return share < REC2020_BETA * 4.5
        ? share / 4.5
        : ((share + REC2020_ALPHA - 1) / REC2020_ALPHA) ** (1 / 0.45);
}

/** Write a light as Rec. 2020's shares. */
function rec2020Encoded(light: number): number {
    return light > REC2020_BETA ? REC2020_ALPHA * light ** 0.45 - (REC2020_ALPHA - 1) : 4.5 * light;
}

/** Read the light of ProPhoto RGB's shares. */
function prophotoLinear(share: number): number {
    return share <= PROPHOTO_LINEAR ? share / 16 : share ** PROPHOTO_GAMMA;
}

/** Write a light as ProPhoto RGB's shares. */
function prophotoEncoded(light: number): number {
    return light >= 1 / 512 ? light ** (1 / PROPHOTO_GAMMA) : 16 * light;
}

/** Turn a polar hue and chroma into two opposed colors. */
function cartesian(chroma: number, hue: number): readonly [number, number] {
    const radians = (hue * Math.PI) / 180;

    return [chroma * Math.cos(radians), chroma * Math.sin(radians)];
}

/** Turn two opposed colors into a chroma and a hue, no hue for a gray. */
function polar(first: number, second: number): readonly [number, number] {
    const chroma = Math.hypot(first, second);
    const hue = ((Math.atan2(second, first) * 180) / Math.PI + 360) % 360;

    return chroma < GAMUT_TOLERANCE ? [0, 0] : [chroma, hue];
}

/** Turn OKLab, its lightness a percentage, into linear sRGB, by Björn Ottosson's matrices. */
function linearFromOklab(lightness: number, redness: number, blueness: number): ColorValues {
    // turn to the light of the three cones, then to red, green and blue
    const light = lightness / 100;
    const long = (light + 0.3963377774 * redness + 0.2158037573 * blueness) ** 3;
    const medium = (light - 0.1055613458 * redness - 0.0638541728 * blueness) ** 3;
    const short = (light - 0.0894841775 * redness - 1.291485548 * blueness) ** 3;

    return [
        4.0767416621 * long - 3.3077115913 * medium + 0.2309699292 * short,
        -1.2684380046 * long + 2.6097574011 * medium - 0.3413193965 * short,
        -0.0041960863 * long - 0.7034186147 * medium + 1.707614701 * short,
    ];
}

/** Turn linear sRGB into OKLab, its lightness a percentage, by Björn Ottosson's matrices. */
function oklabFromLinear([red, green, blue]: ColorValues): ColorValues {
    // read the light of the three cones, then a lightness and two opposed colors
    const long = Math.cbrt(0.4122214708 * red + 0.5363325363 * green + 0.0514459929 * blue);
    const medium = Math.cbrt(0.2119034982 * red + 0.6806995451 * green + 0.1073969566 * blue);
    const short = Math.cbrt(0.0883024619 * red + 0.2817188376 * green + 0.6299787005 * blue);

    return [
        (0.2104542553 * long + 0.793617785 * medium - 0.0040720468 * short) * 100,
        1.9779984951 * long - 2.428592205 * medium + 0.4505937099 * short,
        0.0259040371 * long + 0.7827717662 * medium - 0.808675766 * short,
    ];
}

/** Turn CIE Lab under printed paper's white into XYZ under it. */
function xyz50FromLab([lightness, greenRed, blueYellow]: ColorValues): ColorValues {
    // undo the cube roots of the three responses, linear near black
    const fy = (lightness + 16) / 116;
    const fx = greenRed / 500 + fy;
    const fz = fy - blueYellow / 200;
    const x = fx ** 3 > LAB_EPSILON ? fx ** 3 : (116 * fx - 16) / LAB_KAPPA;
    const y = lightness > LAB_KAPPA * LAB_EPSILON ? fy ** 3 : lightness / LAB_KAPPA;
    const z = fz ** 3 > LAB_EPSILON ? fz ** 3 : (116 * fz - 16) / LAB_KAPPA;

    // scale by the white
    return [x * D50_WHITE[0], y * D50_WHITE[1], z * D50_WHITE[2]];
}

/** Turn XYZ under printed paper's white into CIE Lab under it. */
function labFromXyz50([x, y, z]: ColorValues): ColorValues {
    // take the cube roots of the three responses to the white, linear near black
    const [fx, fy, fz] = [
        labCurve(x / D50_WHITE[0]),
        labCurve(y / D50_WHITE[1]),
        labCurve(z / D50_WHITE[2]),
    ];

    return [116 * fy - 16, 500 * (fx - fy), 200 * (fy - fz)];
}

/** Take the cube root CIE Lab takes of a response, linear near black. */
function labCurve(value: number): number {
    return value > LAB_EPSILON ? Math.cbrt(value) : (LAB_KAPPA * value + 16) / 116;
}

/** Turn a color's numbers in a space into XYZ under daylight's white. */
function toXyz(space: ColorSpace, values: ColorValues): ColorValues {
    const [first, second, third] = values;
    switch (space) {
        case "rgb":
        case "hsl":
        case "hsb":
        case "hwb": {
            const rgb = space === "rgb" ? values : new Color(space, values).to("rgb").values;

            return multiply(XYZ_FROM_SRGB, [linear(rgb[0]), linear(rgb[1]), linear(rgb[2])]);
        }
        case "srgb-linear":
            return multiply(XYZ_FROM_SRGB, values);
        case "display-p3":
            return multiply(XYZ_FROM_P3, curve(values, srgbLinear));
        case "a98-rgb":
            return multiply(
                XYZ_FROM_A98,
                curve(values, (share) => share ** A98_GAMMA),
            );
        case "prophoto-rgb":
            return multiply(
                D65_FROM_D50,
                multiply(XYZ50_FROM_PROPHOTO, curve(values, prophotoLinear)),
            );
        case "rec2020":
            return multiply(XYZ_FROM_REC2020, curve(values, rec2020Linear));
        case "xyz-d50":
            return multiply(D65_FROM_D50, values);
        case "xyz-d65":
            return values;
        case "lab":
            return multiply(D65_FROM_D50, xyz50FromLab(values));
        case "lch": {
            const [greenRed, blueYellow] = cartesian(second, third);

            return multiply(D65_FROM_D50, xyz50FromLab([first, greenRed, blueYellow]));
        }
        case "oklab":
            return multiply(XYZ_FROM_SRGB, linearFromOklab(first, second, third));
        case "oklch": {
            const [redness, blueness] = cartesian(second, third);

            return multiply(XYZ_FROM_SRGB, linearFromOklab(first, redness, blueness));
        }
    }
}

/** Turn XYZ under daylight's white into a color's numbers in a space, unmapped. */
function fromXyz(space: ColorSpace, xyz: ColorValues): ColorValues {
    switch (space) {
        case "rgb":
        case "hsl":
        case "hsb":
        case "hwb": {
            const bytes = curve(multiply(SRGB_FROM_XYZ, xyz), srgbEncoded).map(
                (share) => share * 255,
            );
            const [red = 0, green = 0, blue = 0] = bytes;

            return fromRgb(space, [red, green, blue]);
        }
        case "srgb-linear":
            return multiply(SRGB_FROM_XYZ, xyz);
        case "display-p3":
            return curve(multiply(P3_FROM_XYZ, xyz), srgbEncoded);
        case "a98-rgb":
            return curve(multiply(A98_FROM_XYZ, xyz), (light) => light ** (1 / A98_GAMMA));
        case "prophoto-rgb":
            return curve(
                multiply(PROPHOTO_FROM_XYZ50, multiply(D50_FROM_D65, xyz)),
                prophotoEncoded,
            );
        case "rec2020":
            return curve(multiply(REC2020_FROM_XYZ, xyz), rec2020Encoded);
        case "xyz-d50":
            return multiply(D50_FROM_D65, xyz);
        case "xyz-d65":
            return xyz;
        case "lab":
            return labFromXyz50(multiply(D50_FROM_D65, xyz));
        case "lch": {
            const [lightness, greenRed, blueYellow] = labFromXyz50(multiply(D50_FROM_D65, xyz));
            const [chroma, hue] = polar(greenRed, blueYellow);

            return [lightness, chroma, hue];
        }
        case "oklab":
            return oklabFromLinear(multiply(SRGB_FROM_XYZ, xyz));
        case "oklch": {
            const [lightness, redness, blueness] = oklabFromLinear(multiply(SRGB_FROM_XYZ, xyz));
            const [chroma, hue] = polar(redness, blueness);

            return [lightness, chroma, hue];
        }
    }
}

/** Say a hue, a whiteness and a blackness in red, green and blue. */
function hwbToRgb(hue: number, whiteness: number, blackness: number): ColorValues {
    // gray where white and black fill it, else the pure hue between them
    const white = whiteness / 100;
    const black = blackness / 100;
    if (white + black >= 1) {
        const gray = (white / (white + black)) * 255;

        return [gray, gray, gray];
    }
    const pure = hslToRgb(hue, 100, 50);
    const mix = (channel: number) => channel * (1 - white - black) + white * 255;

    return [mix(pure[0]), mix(pure[1]), mix(pure[2])];
}
