import * as style from "@destack/style";
import { color, radius, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import type { JSX } from "@solidjs/web";
import { merge, omit } from "solid-js";

/** The variant of a badge that sets none. */
const DEFAULTS: Required<Pick<BadgeProperties, "variant">> = { variant: "default" };

/** The styles every badge shares. */
const styles = style.create({
    badge: {
        display: "inline-flex",
        flexShrink: 0,
        alignItems: "center",
        justifyContent: "center",
        gap: space[1],
        width: "fit-content",
        overflow: "hidden",
        paddingBlock: `calc(${space[1]} / 2)`,
        paddingInline: space[2],
        borderStyle: "solid",
        borderWidth: stroke.border,
        borderColor: "transparent",
        borderRadius: radius.full,
        fontWeight: weight.medium,
        whiteSpace: "nowrap",
    },
});

/** The colors of each variant. */
const variants = style.create({
    default: { backgroundColor: color.primary, color: color.primaryForeground },
    secondary: { backgroundColor: color.secondary, color: color.secondaryForeground },
    destructive: { backgroundColor: color.destructive, color: color.destructiveForeground },
    outline: { borderColor: color.border, color: color.foreground },
    ghost: { color: color.foreground },
    link: {
        color: color.primary,
        textDecoration: { default: "none", ":hover": "underline" },
        textUnderlineOffset: "0.25em",
    },
});

/** The look of a badge, from most to least prominent. */
export type BadgeVariant = "default" | "secondary" | "destructive" | "outline" | "ghost" | "link";

/** The properties of a badge, the native span's attributes included. */
export interface BadgeProperties
    extends Omit<JSX.HTMLAttributes<HTMLSpanElement>, "class" | "style">, BadgeStyleOptions {
    /** The StyleX styles applied after the badge's styles. */
    readonly style?: style.Styles;
}

/** The variant of a badge's styles. */
export interface BadgeStyleOptions {
    /** The look, default by default. */
    readonly variant?: BadgeVariant;
}

/** Return the StyleX styles of a badge in a variant, for links and other elements that look like badges. */
export function badgeStyle(options: BadgeStyleOptions): style.Styles {
    return [text.caption, styles.badge, variants[options.variant ?? DEFAULTS.variant]];
}

/** Render a short label such as a status or count in a variant. */
export function Badge(properties: BadgeProperties): JSX.Element {
    const badge = merge(DEFAULTS, properties);
    const rest = omit(badge, "variant", "style");

    return (
        <span
            data-slot="badge"
            data-variant={badge.variant}
            {...rest}
            {...style.attrs(badgeStyle(badge), badge.style)}
        />
    );
}
