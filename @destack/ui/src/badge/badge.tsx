import { Hue } from "@destack/color";
import * as style from "@destack/style";
import { radius, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { type JSX, omit } from "@destack/view";
import { Fill } from "../fill/index.ts";
import { type Render, renderPart } from "../part/index.ts";
import {
    type ComponentProperties,
    type DeclaredProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";
import { field } from "@destack/object";
import { t } from "@destack/locale";

/** The properties people change in a badge's controls and inspector. */
const BADGE_PROPERTIES = {
    variant: field
        .enum(Fill.choices)
        .labeled(t.context("label")`Variant`)
        .described(t`How the badge is filled in its color.`)
        .default("solid"),
    color: field
        .enum(Hue.choices)
        .labeled(t.context("label")`Color`)
        .described(t`The hue it is drawn in, such as a tag's, the accent when absent.`)
        .optional(),
};

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
});

/** The properties of a badge besides its declared ones: its native attributes and its part. */
export interface BadgeAttributes extends Omit<JSX.HTMLAttributes<HTMLSpanElement>, "class"> {
    /** The StyleX styles applied after the badge's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with the badge's attributes, the native span by default. */
    readonly render?: Render;
}

/** The look of a badge: its variant and its color. */
type BadgeLook = DeclaredProperties<typeof BADGE_PROPERTIES>;

/** Return the StyleX styles of a badge: its fill in its hue. */
function badgeVariants(look: BadgeLook): style.Styles {
    return [text.xs, styles.badge, Fill.styles(look.variant, look.color)];
}

/** Render a short label such as a status or count in a variant, its styles at hand for others. */
export const Badge = Object.assign(
    defineComponent({
        name: "badge",
        properties: BADGE_PROPERTIES,
        render(
            properties: RenderProperties<BadgeAttributes, typeof BADGE_PROPERTIES>,
        ): JSX.Element {
            // mark the part and its variant over its styles
            const rest = omit(properties, "variant", "color");

            return renderPart("span", "badge", rest, () => badgeVariants(properties), {
                get "data-variant"() {
                    return properties.variant;
                },
                get "data-color"() {
                    return properties.color;
                },
            });
        },
    }),
    {
        /** Read the styles of a badge, for an element that only looks like one. */
        styles: (look: Partial<BadgeLook>): style.Styles =>
            badgeVariants({ ...Badge.defaults, ...look }),
    },
);

/** The properties of a badge, the native span's attributes included. */
export type BadgeProperties = ComponentProperties<typeof Badge>;
