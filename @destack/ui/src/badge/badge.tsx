import type { Hue } from "@destack/color";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, radius, space, stroke, swatch, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { type JSX, merge, omit } from "@destack/view";
import { type PartAttributes, type Render, useRender } from "../part/index.ts";

/** The variant of a badge that sets none. */
const DEFAULTS: Required<Pick<BadgeProperties, "variant">> = { variant: "default" };

/** The styles every badge shares. */
const styles = style.create({
    badge: {
        fontVariantNumeric: "tabular-nums",
        display: "inline-flex",
        flexShrink: 0,
        alignItems: "center",
        justifyContent: "center",
        gap: space[1],
        width: "fit-content",
        overflow: "hidden",
        paddingBlock: `calc(${space[1]} / 2)`,
        paddingInline: space[2],
        borderWidth: stroke.border,
        borderColor: "transparent",
        borderRadius: radius[2],
        fontWeight: weight.medium,
        whiteSpace: "nowrap",
    },
    tinted: (background: string, foreground: string) => ({
        backgroundColor: background,
        color: foreground,
    }),
});

/** The colors of each variant. */
const variants = style.create({
    default: { backgroundColor: swatch.accentSolid, color: swatch.accentLabel },
    secondary: { backgroundColor: color.muted, color: color.foreground },
    destructive: { backgroundColor: swatch.destructiveSolid, color: swatch.destructiveLabel },
    outline: { borderColor: color.border, color: color.foreground },
    ghost: { color: color.foreground },
    link: {
        color: swatch.accentText,
        textDecoration: {
            default: "none",
            ":hover": { default: null, [media.hover]: "underline" },
        },
        textUnderlineOffset: "0.25em",
    },
});

/** The look of a badge, from most to least prominent. */
export type BadgeVariant = "default" | "secondary" | "destructive" | "outline" | "ghost" | "link";

/** The properties of a badge, the native span's attributes included. */
export interface BadgeProperties
    extends Omit<JSX.HTMLAttributes<HTMLSpanElement>, "class">, BadgeVariantProperties {
    /** The StyleX styles applied after the badge's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with the badge's attributes, the native span by default. */
    readonly render?: Render;
}

/** The variant of a badge's styles. */
export interface BadgeVariantProperties {
    /** The look, default by default. */
    readonly variant?: BadgeVariant;
    /** The swatch a badge tints itself in, such as a tag's, the variant's colors when absent. */
    readonly color?: Hue;
}

/** Return the StyleX styles of a badge in a variant. */
export function badgeVariants(options: BadgeVariantProperties): style.Styles {
    const tint = options.color;

    return [
        text.xs,
        styles.badge,
        variants[options.variant ?? DEFAULTS.variant],
        tint !== undefined && styles.tinted(swatch[`${tint}Tint`], swatch[`${tint}TintText`]),
    ];
}

/** Render a short label such as a status or count in a variant. */
export function Badge(properties: BadgeProperties): JSX.Element {
    // mark the part and its variant over its styles
    const badge = merge(DEFAULTS, properties);
    const rest = omit(badge, "variant", "color", "xstyle", "style", "render");
    const part: PartAttributes = merge(
        () => style.attributes([badgeVariants(badge), badge.xstyle], badge.style),
        {
            "data-slot": "badge",
            get "data-variant"() {
                return badge.variant;
            },
            get "data-color"() {
                return badge.color;
            },
        },
    );

    return useRender({
        render: badge.render,
        defaultTagName: "span",
        part: part,
        properties: rest,
    });
}
