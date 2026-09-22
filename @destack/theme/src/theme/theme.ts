import {
    blackAlpha,
    type GrayPalette,
    type Palette,
    paletteAlpha,
    paletteColor,
    paletteForeground,
} from "../palette/index.ts";

/** Appearance inherited from the device or selected explicitly. */
export type Appearance = "system" | "light" | "dark";

/** Corner treatments supported by the shared controls. */
export type Radius = "none" | "small" | "medium" | "large" | "full";

/** Proportional interface scales. */
export type Scaling = "90%" | "95%" | "100%" | "105%" | "110%";

/** Values selected by the application or user preferences. */
export interface ThemeOptions {
    /** Light, dark, or the device preference. */
    appearance?: Appearance;
    /** Palette for primary actions and selection. */
    accent?: Palette;
    /** Neutral palette for backgrounds and text. */
    gray?: GrayPalette;
    /** Corner treatment for controls. */
    radius?: Radius;
    /** Interface spacing and type scale. */
    scaling?: Scaling;
    /** Font stack for interface text. */
    fontFamily?: string;
    /** Font stack for code and tabular text. */
    monospaceFontFamily?: string;
}

/** Scoped CSS declarations accepted by an element's style attribute. */
export type ThemeStyle = {
    /** Browser appearance used by light-dark() and native controls. */
    "color-scheme": "light dark" | "light" | "dark";
    /** Named values consumed by the shared tokens. */
    [variable: `--destack-${string}`]: string;
};

/** Attributes applied to an application or nested theme root. */
export interface Theme {
    /** Appearance used by the theme stylesheet. */
    "data-destack-theme": Appearance;
    /** Corner treatment. */
    "data-radius": Radius;
    /** Palette and scale variables for this root. */
    style: ThemeStyle;
}

/** Create scoped CSS variables without accessing the document or preferences. */
export function createTheme(options: ThemeOptions = {}): Theme {
    // choose palettes independently of appearance
    const accent = options.accent ?? "indigo";
    const gray = options.gray ?? "slate";
    const neutral = (step: number) => paletteColor(gray, step);
    const primary = (step: number) => paletteColor(accent, step);
    const scaling = Number.parseInt(options.scaling ?? "100%", 10) / 100;

    // pair semantic backgrounds and foregrounds with their interaction states
    const roles = {
        background: neutral(1),
        foreground: neutral(12),
        card: neutral(2),
        cardForeground: neutral(12),
        popover: neutral(2),
        popoverForeground: neutral(12),
        primary: primary(9),
        primaryForeground: paletteForeground(accent),
        secondary: neutral(3),
        secondaryForeground: neutral(12),
        muted: neutral(3),
        mutedForeground: neutral(11),
        accent: primary(3),
        accentForeground: primary(12),
        border: neutral(6),
        input: neutral(7),
        ring: primary(8),
        destructive: paletteColor("red", 9),
        sidebar: neutral(2),
        sidebarForeground: neutral(12),
        sidebarPrimary: primary(9),
        sidebarPrimaryForeground: paletteForeground(accent),
        sidebarAccent: primary(3),
        sidebarAccentForeground: primary(12),
        sidebarBorder: neutral(6),
        sidebarRing: primary(8),
    };

    // expose CSS variables on the element that establishes the theme
    const style: ThemeStyle = {
        "color-scheme":
            options.appearance === undefined || options.appearance === "system"
                ? "light dark"
                : options.appearance,
        "--destack-scaling": String(scaling),
    };
    for (const [name, value] of Object.entries(roles)) {
        style[`--destack-color-${name}`] = value;
    }

    // set neutral and alpha scales for shadows
    for (let step = 1; step <= 12; step++) {
        style[`--destack-gray-${step}`] = neutral(step);
        style[`--destack-gray-a${step}`] = paletteAlpha(gray, step);
        style[`--destack-black-a${step}`] = blackAlpha[`blackA${step}` as keyof typeof blackAlpha];
    }

    // apply font overrides
    if (options.fontFamily !== undefined) {
        style["--destack-default-font-family"] = options.fontFamily;
    }
    if (options.monospaceFontFamily !== undefined) {
        style["--destack-code-font-family"] = options.monospaceFontFamily;
    }

    return {
        "data-destack-theme": options.appearance ?? "system",
        "data-radius": options.radius ?? "medium",
        style,
    };
}
