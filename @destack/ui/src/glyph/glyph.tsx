import * as style from "@destack/style";
import type { IconName } from "@destack/icon";
import { LazyIcon } from "@destack/icon/lazy";
import { color, radius, size } from "@destack/theme/tokens.stylex";
import { type JSX, omit } from "@destack/view";

/** What a glyph shows: an emoji, an icon by name, or an image by its address. */
export type GlyphValue =
    | { readonly emoji: string }
    | { readonly icon: IconName }
    | { readonly source: string };

/** The size of a glyph, from smallest to largest. */
export type GlyphSize = "sm" | "default" | "lg" | "xl";

/** The properties of a glyph, the native element's attributes included. */
export interface GlyphProperties extends Omit<JSX.HTMLAttributes<HTMLSpanElement>, "class"> {
    /** What the glyph shows. */
    readonly glyph: GlyphValue;
    /** The width and height, default by default. */
    readonly size?: GlyphSize;
    /** The accessible name, which exposes the glyph as an image instead of hiding it. */
    readonly label?: string;
    /** The StyleX styles applied after the glyph's styles. */
    readonly xstyle?: style.Styles;
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
export function Glyph(properties: GlyphProperties): JSX.Element {
    // keep the native attributes apart from the glyph's own
    const rest = omit(properties, "glyph", "size", "label", "xstyle", "style");

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

    return (
        <span
            data-slot="glyph"
            data-size={properties.size ?? "default"}
            {...rest}
            role={properties.label === undefined ? undefined : "img"}
            aria-label={properties.label}
            aria-hidden={properties.label === undefined ? "true" : undefined}
            {...style.attributes(
                [styles.glyph, sizes[properties.size ?? "default"], properties.xstyle],
                properties.style,
            )}
        >
            {content()}
        </span>
    );
}
