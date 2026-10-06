import * as style from "@destack/style";
import { color, space, stroke } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { type JSX, merge, omit } from "@destack/view";

/** The variant of a marker that sets none. */
const DEFAULTS: Required<Pick<MarkerProperties, "variant">> = { variant: "default" };

/** The line a separator marker draws on each side of its content. */
const RULE = {
    content: "''",
    flex: 1,
    minWidth: 0,
    height: stroke.border,
    backgroundColor: color.border,
} as const;

/** The styles of a marker and its elements. */
const styles = style.create({
    marker: {
        position: "relative",
        display: "flex",
        alignItems: "center",
        gap: space[2],
        width: "100%",
        minHeight: "1em",
        color: color.mutedForeground,
        textAlign: "start",
    },
    icon: {
        display: "inline-flex",
        flexShrink: 0,
    },
    content: {
        minWidth: 0,
        overflowWrap: "anywhere",
    },
});

/** The decoration of each variant. */
const variants = style.create({
    default: {},
    separator: {
        "::before": RULE,
        "::after": RULE,
    },
    border: {
        paddingBottom: space[2],
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        borderBottomColor: color.border,
    },
});

/** How a marker sets itself apart: plain, between two rules, or above a border. */
export type MarkerVariant = "default" | "separator" | "border";

/** The properties of an element of a marker, the native element's attributes included. */
export type MarkerElementProperties<Attributes = JSX.HTMLAttributes<HTMLSpanElement>> = Omit<
    Attributes,
    "class"
> & {
    /** The StyleX styles applied after the element's styles. */
    readonly xstyle?: style.Styles;
};

/** The properties of a marker, the native element's attributes included. */
export interface MarkerProperties extends MarkerElementProperties<
    JSX.HTMLAttributes<HTMLDivElement>
> {
    /** The decoration, default by default. */
    readonly variant?: MarkerVariant;
}

/** Render a quiet line in a conversation, such as a date, a person joining or an unread mark. */
export function Marker(properties: MarkerProperties): JSX.Element {
    const marker = merge(DEFAULTS, properties);
    const rest = omit(marker, "variant", "xstyle", "style");

    return (
        <div
            data-slot="marker"
            data-variant={marker.variant}
            {...rest}
            {...style.attributes(
                [text.footnote, styles.marker, variants[marker.variant], marker.xstyle],
                marker.style,
            )}
        />
    );
}

/** Render a marker's icon, hidden from assistive technology. */
export function MarkerIcon(properties: MarkerElementProperties): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <span
            data-slot="marker-icon"
            aria-hidden="true"
            {...rest}
            {...style.attributes([styles.icon, properties.xstyle], properties.style)}
        />
    );
}

/** Render a marker's text. */
export function MarkerContent(properties: MarkerElementProperties): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <span
            data-slot="marker-content"
            {...rest}
            {...style.attributes([styles.content, properties.xstyle], properties.style)}
        />
    );
}
