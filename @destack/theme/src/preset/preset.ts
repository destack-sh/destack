import { defineSchema, schema } from "@destack/schema";

/** The preset scale names, neutral grays first. */
export const PRESET_NAMES = [
    "gray",
    "mauve",
    "slate",
    "sage",
    "olive",
    "sand",
    "tomato",
    "red",
    "ruby",
    "crimson",
    "pink",
    "plum",
    "purple",
    "violet",
    "iris",
    "indigo",
    "blue",
    "cyan",
    "teal",
    "jade",
    "green",
    "grass",
    "brown",
    "bronze",
    "gold",
    "sky",
    "mint",
    "lime",
    "yellow",
    "amber",
    "orange",
] as const;

/** The neutral presets for backgrounds, borders and text. */
export const GRAY_PRESETS = ["gray", "mauve", "slate", "sage", "olive", "sand"] as const;

/** The solid step 9 of each neutral preset in the light and dark appearance, which its scale grows from. */
export const GRAY_SOLIDS: Readonly<Record<GrayPreset, Solids>> = {
    gray: { light: "#8d8d8d", dark: "#6e6e6e" },
    mauve: { light: "#8e8c99", dark: "#6f6d78" },
    slate: { light: "#8b8d98", dark: "#696e77" },
    sage: { light: "#868e8b", dark: "#63706b" },
    olive: { light: "#898e87", dark: "#687066" },
    sand: { light: "#8d8d86", dark: "#6f6d66" },
};

/** The solid step 9 of each accent preset, alike in both appearances, which its scale grows from. */
export const ACCENT_SOLIDS: Readonly<Record<AccentPreset, string>> = {
    tomato: "#e54d2e",
    red: "#e5484d",
    ruby: "#e54666",
    crimson: "#e93d82",
    pink: "#d6409f",
    plum: "#ab4aba",
    purple: "#8e4ec6",
    violet: "#6e56cf",
    iris: "#5b5bd6",
    indigo: "#3e63dd",
    blue: "#0090ff",
    cyan: "#00a2c7",
    teal: "#12a594",
    jade: "#29a383",
    green: "#30a46c",
    grass: "#46a758",
    brown: "#ad7f58",
    bronze: "#a18072",
    gold: "#978365",
    sky: "#7ce2fe",
    mint: "#86ead4",
    lime: "#bdee63",
    yellow: "#ffe629",
    amber: "#ffc53d",
    orange: "#f76b15",
};

/** A preset scale name. */
export const Preset = defineSchema(schema.enum(PRESET_NAMES));
/** A preset scale name. */
export type Preset = schema.Infer<typeof Preset>;

/** A neutral preset scale. */
export type GrayPreset = (typeof GRAY_PRESETS)[number];

/** An accent preset scale. */
export type AccentPreset = Exclude<Preset, GrayPreset>;

/** The solid step 9 of a scale in the light and dark appearance. */
export interface Solids {
    /** The light solid. */
    readonly light: string;
    /** The dark solid. */
    readonly dark: string;
}
