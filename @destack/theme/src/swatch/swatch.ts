import type { Hue } from "@destack/color";
import { swatch } from "../token/tokens.stylex.ts";

/** The name of a swatch: the accent, a status or a hue. */
export type SwatchName = "accent" | "destructive" | "success" | "warning" | "info" | Hue;

/** The colors of one swatch by role, each a CSS value. */
export class Swatch {
    /** The solid fill, such as a filled button. */
    readonly solid: string;
    /** The label reading on the solid fill. */
    readonly label: string;
    /** The tint, such as a tag's background. */
    readonly tint: string;
    /** The text reading on the tint. */
    readonly tintText: string;
    /** The text on every surface, such as a link. */
    readonly text: string;
    /** The graphics on the page, such as a chart's line or a tag's dot. */
    readonly mark: string;

    /** Read a swatch's colors from the theme's tokens. */
    constructor(name: SwatchName) {
        // fills and what reads on them
        this.solid = swatch[`${name}Solid`];
        this.label = swatch[`${name}Label`];
        this.tint = swatch[`${name}Tint`];
        this.tintText = swatch[`${name}TintText`];

        // text and graphics on the page
        this.text = swatch[`${name}Text`];
        this.mark = swatch[`${name}Mark`];
    }
}

/** Every swatch of the theme by name. */
export const swatches = {
    accent: new Swatch("accent"),
    destructive: new Swatch("destructive"),
    success: new Swatch("success"),
    warning: new Swatch("warning"),
    info: new Swatch("info"),
    gray: new Swatch("gray"),
    brown: new Swatch("brown"),
    red: new Swatch("red"),
    orange: new Swatch("orange"),
    yellow: new Swatch("yellow"),
    green: new Swatch("green"),
    cyan: new Swatch("cyan"),
    blue: new Swatch("blue"),
    purple: new Swatch("purple"),
    pink: new Swatch("pink"),
} satisfies Readonly<Record<SwatchName, Swatch>>;
