import { schema } from "@destack/schema";
import { Color } from "../color/index.ts";
import { Dimension } from "../dimension/index.ts";
import { type Bindable, Reference } from "../reference/index.ts";

/** A shadow a box casts, or one inside it, as the design token format's shadow. */
const shadowJson = schema
    .object({
        /** The shadow's color. */
        color: Color.bindable,
        /** How far it falls to the right. */
        offsetX: Dimension.bindable,
        /** How far it falls down. */
        offsetY: Dimension.bindable,
        /** How far it blurs. */
        blur: Dimension.bindable,
        /** How far it grows past the box before it blurs. */
        spread: Dimension.bindable,
        /** Whether it falls inside the box, as the format and CSS's `inset` keyword mark it. */
        inset: schema.boolean(),
    })
    .meta({ id: "style.shadow", title: "Shadow" });
/** A shadow a box casts, or one inside it. */
export type Shadow = schema.Infer<typeof shadowJson>;

/** A shadow a box casts, or one inside it, as the design token format's shadow. */
export const Shadow = Object.assign(shadowJson, {
    /** Write a shadow as one of CSS's `box-shadow` list. */
    css(shadow: Shadow): string {
        // write the lengths in CSS's order, inset first where it falls inside
        const lengths = [shadow.offsetX, shadow.offsetY, shadow.blur, shadow.spread];
        const written = lengths.map((length) => Dimension.css(length));

        return [...(shadow.inset ? ["inset"] : []), ...written, Color.css(shadow.color)].join(" ");
    },
});

/** The shadows of a box, the first on top, as CSS lists them. */
const shadowStackJson = schema.array(Shadow).meta({ id: "style.shadowStack", title: "Shadows" });

/** The shadows of a box, the first on top, as CSS lists them. */
export const ShadowStack = Object.assign(shadowStackJson, {
    /** Read the shadows of CSS's `box-shadow` as a browser computes it, none for other text. */
    read(text: string): Shadow[] | undefined {
        // read no shadow, or each of the list apart, refusing the list if one fails
        const shadows = text.trim() === "none" ? [] : split(text, ",").map((part) => read(part));

        return shadows.every((shadow) => shadow !== undefined) ? shadows : undefined;
    },

    /** Shadows, or a reference to a shadow token in their place. */
    bindable: Reference.or(shadowStackJson, "shadow"),

    /** Write the shadows as CSS's `box-shadow`, none for no shadow. */
    css(stack: Bindable<readonly Shadow[]>): string {
        return Reference.write(stack, (shadows) =>
            shadows.length === 0 ? "none" : shadows.map((shadow) => Shadow.css(shadow)).join(", "),
        );
    },
});

/** Read one shadow of CSS's `box-shadow` list: its color, its lengths, and `inset`. */
function read(text: string): Shadow | undefined {
    // take the color, then the lengths in order, and note inset
    const words = split(text, " ");
    const color = words.find((word) => Color.schema.safeParse(word).success);
    const lengths = words
        .filter((word) => word !== color && word !== "inset")
        .map((word) => Dimension.read(word));
    const [offsetX, offsetY, blur = Dimension.zero, spread = Dimension.zero] = lengths;

    // refuse a shadow without a color, its two offsets, or a length CSS writes otherwise
    if (color === undefined || offsetX === undefined || offsetY === undefined) {
        return undefined;
    } else if (lengths.length > 4 || lengths.some((length) => length === undefined)) {
        return undefined;
    }

    return { color, offsetX, offsetY, blur, spread, inset: words.includes("inset") };
}

/** Split CSS text at a separator outside parentheses, dropping empty parts. */
function split(text: string, separator: string): string[] {
    // track the depth of parentheses, and cut at each separator outside them
    const parts = [""];
    let depth = 0;
    for (const character of text) {
        depth += character === "(" ? 1 : character === ")" ? -1 : 0;
        if (depth === 0 && character === separator) {
            parts.push("");
        } else {
            parts[parts.length - 1] += character;
        }
    }

    return parts.map((part) => part.trim()).filter((part) => part.length > 0);
}
