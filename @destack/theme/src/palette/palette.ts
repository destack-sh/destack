import * as colors from "./colors.ts";

/** Neutral alpha scales for shadows. */
const ALPHA = {
    gray: [colors.grayA, colors.grayDarkA],
    mauve: [colors.mauveA, colors.mauveDarkA],
    slate: [colors.slateA, colors.slateDarkA],
    sage: [colors.sageA, colors.sageDarkA],
    olive: [colors.oliveA, colors.oliveDarkA],
    sand: [colors.sandA, colors.sandDarkA],
} as const;

/** Black alpha steps for shadows. */
export const blackAlpha = colors.blackA;

/** Light and dark scales used by the theme. */
const PALETTES = {
    gray: [colors.gray, colors.grayDark],
    mauve: [colors.mauve, colors.mauveDark],
    slate: [colors.slate, colors.slateDark],
    sage: [colors.sage, colors.sageDark],
    olive: [colors.olive, colors.oliveDark],
    sand: [colors.sand, colors.sandDark],
    tomato: [colors.tomato, colors.tomatoDark],
    red: [colors.red, colors.redDark],
    ruby: [colors.ruby, colors.rubyDark],
    crimson: [colors.crimson, colors.crimsonDark],
    pink: [colors.pink, colors.pinkDark],
    plum: [colors.plum, colors.plumDark],
    purple: [colors.purple, colors.purpleDark],
    violet: [colors.violet, colors.violetDark],
    iris: [colors.iris, colors.irisDark],
    indigo: [colors.indigo, colors.indigoDark],
    blue: [colors.blue, colors.blueDark],
    cyan: [colors.cyan, colors.cyanDark],
    teal: [colors.teal, colors.tealDark],
    jade: [colors.jade, colors.jadeDark],
    green: [colors.green, colors.greenDark],
    grass: [colors.grass, colors.grassDark],
    brown: [colors.brown, colors.brownDark],
    bronze: [colors.bronze, colors.bronzeDark],
    gold: [colors.gold, colors.goldDark],
    sky: [colors.sky, colors.skyDark],
    mint: [colors.mint, colors.mintDark],
    lime: [colors.lime, colors.limeDark],
    yellow: [colors.yellow, colors.yellowDark],
    amber: [colors.amber, colors.amberDark],
    orange: [colors.orange, colors.orangeDark],
} as const;

/** Palettes supported by the theme. */
export type Palette = keyof typeof PALETTES;

/** Neutral palettes used for text, backgrounds, and borders. */
export type GrayPalette = "gray" | "mauve" | "slate" | "sage" | "olive" | "sand";

/** Resolve a neutral alpha step in both appearances. */
export function paletteAlpha(palette: GrayPalette, step: number): string {
    // select matching alpha steps
    const [light, dark]: readonly [Record<string, string>, Record<string, string>] = ALPHA[palette];
    const key = `${palette}A${step}`;
    if (!light[key] || !dark[key]) {
        throw new RangeError(`Unknown palette step: ${key}`);
    }

    return `light-dark(${light[key]}, ${dark[key]})`;
}

/** Resolve a palette step in both appearances. */
export function paletteColor(palette: Palette, step: number): string {
    // select corresponding steps without choosing a device appearance
    const [light, dark]: readonly [Record<string, string>, Record<string, string>] =
        PALETTES[palette];
    const key = `${palette}${step}`;
    if (!light[key] || !dark[key]) {
        throw new RangeError(`Unknown palette step: ${key}`);
    }

    return `light-dark(${light[key]}, ${dark[key]})`;
}

/** Select text for solid palette backgrounds. */
export function paletteForeground(palette: Palette): string {
    // select contrasting text for the palette
    switch (palette) {
        case "sky":
            return "#1c2024";
        case "mint":
            return "#1a211e";
        case "lime":
            return "#1d211c";
        case "yellow":
        case "amber":
            return "#21201c";
        default:
            return "white";
    }
}
