import { schema } from "@destack/schema";
import { Seed } from "../palette/index.ts";

/** The body size of each text size relative to the default medium, from 14 to 23 points over 16. */
export const TEXT_SCALES = {
    xSmall: 14 / 16,
    small: 15 / 16,
    medium: 1,
    large: 17 / 16,
    xLarge: 19 / 16,
    xxLarge: 21 / 16,
    xxxLarge: 23 / 16,
} as const;

/** The spacing of each density, one step of 4 pixels on a 32-pixel control apart. */
export const DENSITY_SCALES = {
    compact: 0.875,
    regular: 1,
    spacious: 1.125,
} as const;

/** The appearance a person sees: the device preference, light or dark. */
export const Appearance = schema.enum(["system", "light", "dark"]);
/** The appearance a person sees. */
export type Appearance = schema.Infer<typeof Appearance>;

/** The text size a person reads at. */
export const TextSize = schema.enum([
    "xSmall",
    "small",
    "medium",
    "large",
    "xLarge",
    "xxLarge",
    "xxxLarge",
]);
/** The text size a person reads at. */
export type TextSize = schema.Infer<typeof TextSize>;

/** How tightly controls and content pack. */
export const Density = schema.enum(["compact", "regular", "spacious"]);
/** How tightly controls and content pack. */
export type Density = schema.Infer<typeof Density>;

/** How far text, lines and graphics stand out. */
export const Contrast = schema.union([schema.literal("system"), schema.number().min(0).max(1)]);
/** How far text, lines and graphics stand out. */
export type Contrast = schema.Infer<typeof Contrast>;

/** The motion of transitions: the device preference, full or reduced to none. */
export const Motion = schema.enum(["system", "full", "reduced"]);
/** The motion of transitions. */
export type Motion = schema.Infer<typeof Motion>;

/** How a person wants pages displayed, each choice read from its setting. */
export const Display = schema.object({
    /** The appearance. */
    appearance: Appearance,
    /** The text size. */
    textSize: TextSize,
    /** The density, or null for the theme's own. */
    density: Density.nullable(),
    /** The contrast. */
    contrast: Contrast,
    /** The motion. */
    motion: Motion,
    /** The accent, or null for the theme's own. */
    accent: Seed.nullable(),
});
/** How a person wants pages displayed. */
export type Display = schema.Infer<typeof Display>;

/** The display of a person who changed nothing: each setting's default. */
export const DEFAULT_DISPLAY: Display = {
    appearance: "system",
    textSize: "medium",
    density: null,
    contrast: "system",
    motion: "system",
    accent: null,
};
