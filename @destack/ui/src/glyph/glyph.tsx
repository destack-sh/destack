import { t } from "@destack/locale";
import { field } from "@destack/object";
import { renderPart, type Render } from "../part/index.ts";
import * as style from "@destack/style";
import type { IconName } from "@destack/icon";
import { LazyIcon } from "@destack/ui/icon";
import { color, radius, size } from "@destack/theme/tokens.stylex";
import { type JSX, omit } from "@destack/view";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** What a glyph shows: an emoji, an icon by name, or an image by its address. */
export type GlyphValue =
    | { readonly emoji: string }
    | { readonly icon: IconName }
    | { readonly source: string };

/** The size of a glyph, from smallest to largest. */
export type GlyphSize = keyof typeof GLYPH_SIZES;

/** The sizes of a glyph, from smallest to largest. */
const GLYPH_SIZES = {
    sm: { title: t.context("option")`Small` },
    default: { title: t.context("option")`Default` },
    lg: { title: t.context("option")`Large` },
    xl: { title: t.context("option")`Extra large` },
};

/** The properties people change in a glyph's controls and inspector. */
const GLYPH_PROPERTIES = {
    size: field
        .enum(GLYPH_SIZES)
        .labeled(t.context("label")`Size`)
        .described(t`How large the glyph draws.`)
        .default("default"),
};

/** The properties of a glyph besides its declared ones: what it shows and its name. */
export interface GlyphAttributes extends Omit<JSX.HTMLAttributes<HTMLSpanElement>, "class"> {
    /** What the glyph shows. */
    readonly glyph: GlyphValue;
    /** The accessible name, which exposes the glyph as an image instead of hiding it. */
    readonly label?: string;
    /** The StyleX styles applied after the glyph's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with its attributes, its own by default. */
    readonly render?: Render;
}

/** The styles of a glyph and what it shows. */
const styles = style.create({
    glyph: {
        display: "inline-flex",
        flexShrink: 0,
        alignItems: "center",
        justifyContent: "center",
        lineHeight: 1,
        color: color.foreground,
        userSelect: "none",
    },
    image: {
        width: "100%",
        height: "100%",
        borderRadius: radius[1],
        objectFit: "cover",
    },
});

/** The width, height and emoji size of each glyph size. */
const sizes = style.create({
    sm: { width: size[1], height: size[1], fontSize: `calc(${size[1]} * 0.8)` },
    default: { width: size[2], height: size[2], fontSize: `calc(${size[2]} * 0.8)` },
    lg: { width: size[3], height: size[3], fontSize: `calc(${size[3]} * 0.8)` },
    xl: { width: size[4], height: size[4], fontSize: `calc(${size[4]} * 0.8)` },
});

/** Render an emoji, an icon or an image at one size. */
export const Glyph = defineComponent({
    name: "glyph",
    properties: GLYPH_PROPERTIES,
    render(properties: RenderProperties<GlyphAttributes, typeof GLYPH_PROPERTIES>): JSX.Element {
        // split the native attributes from the glyph's own
        const rest = omit(properties, "glyph", "size", "label", "children");

        // draw the emoji, the icon or the image
        const content = (): JSX.Element => {
            const shown = properties.glyph;
            if ("emoji" in shown) {
                return shown.emoji;
            } else if ("icon" in shown) {
                return <LazyIcon name={shown.icon} size="100%" />;
            } else {
                return <img src={shown.source} alt="" {...style.attributes(styles.image)} />;
            }
        };

        return renderPart("span", "glyph", rest, () => [styles.glyph, sizes[properties.size]], {
            get "data-size"() {
                return properties.size;
            },
            get role() {
                return properties.label === undefined ? undefined : "img";
            },
            get "aria-label"() {
                return properties.label;
            },
            get "aria-hidden"() {
                return properties.label === undefined ? "true" : undefined;
            },
            get children() {
                return content();
            },
        });
    },
});

/** The properties of a glyph, its native attributes included. */
export type GlyphProperties = ComponentProperties<typeof Glyph>;
