import { t } from "@destack/locale";
import { ORIENTATION } from "../position/index.ts";
import * as style from "@destack/style";
import { color, stroke } from "@destack/theme/tokens.stylex";
import { type JSX, merge, omit } from "@destack/view";
import { type Render, renderPart } from "../part/index.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The line of a separator in each orientation. */
const orientations = style.create({
    horizontal: {
        flexShrink: 0,
        width: "100%",
        height: 0,
        borderTopWidth: stroke.border,
        borderColor: color.border,
    },
    vertical: {
        flexShrink: 0,
        alignSelf: "stretch",
        width: 0,
        borderLeftWidth: stroke.border,
        borderColor: color.border,
    },
});

/** The properties people change in a separator's controls and inspector. */
const SEPARATOR_PROPERTIES = {
    orientation: ORIENTATION.described(t`Whether the line runs across or down.`).default(
        "horizontal",
    ),
};

/** The properties of a separator besides its declared ones: its role and attributes. */
export interface SeparatorAttributes extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> {
    /** Whether the separator only decorates, hidden from assistive technology, true by default. */
    readonly decorative?: boolean;
    /** The StyleX styles applied after the separator's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with the separator's attributes, the native div by default. */
    readonly render?: Render;
}

/** Render a line between content, exposed as a separator unless it only decorates. */
export const Separator = defineComponent({
    name: "separator",
    properties: SEPARATOR_PROPERTIES,
    render(
        properties: RenderProperties<SeparatorAttributes, typeof SEPARATOR_PROPERTIES>,
    ): JSX.Element {
        // mark the part with its orientation and role over its styles
        const separator = merge({ decorative: true }, properties);
        const rest = omit(separator, "orientation", "decorative");

        return renderPart("div", "separator", rest, () => orientations[separator.orientation], {
            get "data-orientation"() {
                return separator.orientation;
            },
            get role() {
                return separator.decorative ? "none" : "separator";
            },
            get "aria-orientation"() {
                return !separator.decorative && separator.orientation === "vertical"
                    ? "vertical"
                    : undefined;
            },
        });
    },
});

/** The properties of a separator, its native attributes included. */
export type SeparatorProperties = ComponentProperties<typeof Separator>;
