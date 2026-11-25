import { media } from "@destack/style/media.stylex";
import * as style from "@destack/style";
import { state } from "@destack/theme/state";
import { text } from "@destack/theme/text";
import { color, motion, radius, space, stroke, swatch } from "@destack/theme/tokens.stylex";
import { type Accessor, createContext, type JSX, merge, omit, useContext } from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

/** The light that runs over the words of a status under way. */
const shimmer = style.keyframes({
    from: { backgroundPosition: "150% 0" },
    to: { backgroundPosition: "-50% 0" },
});

/** The variant of a marker that sets none. */
const DEFAULTS = { variant: "default" } as const;

/** The styles of a marker and its parts. */
const styles = style.create({
    marker: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
        minWidth: 0,
        width: "100%",
        paddingInline: 0,
        borderWidth: 0,
        backgroundColor: "transparent",
        color: color.mutedForeground,
        textAlign: "start",
        overflowWrap: "anywhere",
    },
    interactive: {
        borderRadius: radius[2],
        cursor: "pointer",
        textDecorationLine: { default: null, ":is(a)": "underline" },
        textUnderlineOffset: "0.2em",
        color: {
            default: color.mutedForeground,
            ":hover": { default: null, [media.hover]: color.foreground },
        },
    },
    error: {
        color: swatch.destructiveText,
    },
    description: {
        flexShrink: 0,
        fontVariantNumeric: "tabular-nums",
        color: color.mutedForeground,
    },
    icon: {
        display: "inline-flex",
        flexShrink: 0,
    },
    content: {
        minWidth: 0,
    },
    running: {
        color: { default: "transparent", [media.forcedColors]: "CanvasText" },
        backgroundClip: "text",
        backgroundImage: {
            default: `linear-gradient(90deg, ${color.mutedForeground} 35%, ${color.foreground} 50%, ${color.mutedForeground} 65%)`,
            [media.forcedColors]: "none",
        },
        backgroundSize: "200% 100%",
        animationName: { default: shimmer, [media.motionReduce]: "none" },
        animationDuration: `calc(6 * ${motion.durationLong})`,
        animationTimingFunction: "linear",
        animationIterationCount: "infinite",
    },
});

/** The layout of each variant. */
const variants = style.create({
    default: {},
    border: {
        paddingBlockEnd: space[2],
        borderBlockEndWidth: stroke.border,
        borderBlockEndStyle: "solid",
        borderBlockEndColor: color.border,
    },
    separator: {
        justifyContent: "center",
        textAlign: "center",
        "::before": {
            content: "''",
            flex: 1,
            borderBlockStartWidth: stroke.border,
            borderBlockStartStyle: "solid",
            borderBlockStartColor: color.border,
        },
        "::after": {
            content: "''",
            flex: 1,
            borderBlockStartWidth: stroke.border,
            borderBlockStartStyle: "solid",
            borderBlockStartColor: color.border,
        },
    },
});

/** The layout of a marker: inline, with a line below, or between two lines. */
export type MarkerVariant = keyof typeof variants;

/** The work a marker reports: under way, or failed. */
export type MarkerStatus = "running" | "error";

/** The status of the nearest marker. */
const StatusContext = createContext<Accessor<MarkerStatus | undefined>>(() => undefined);

/** The properties of a marker, the native element's attributes included. */
export type MarkerProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> &
    ElementPartProperties & {
        /** The layout, inline by default. */
        readonly variant?: MarkerVariant;
        /** The work it reports, which its words say too, none by default. */
        readonly status?: MarkerStatus | undefined;
    };

/** Render a note in a stream, such as a status, a day or someone joining. */
export function Marker(properties: MarkerProperties): JSX.Element {
    const marker = merge(DEFAULTS, properties);
    const rest = omit(properties, "variant", "status");

    return (
        <StatusContext value={() => properties.status}>
            {renderPart(
                "div",
                "marker",
                rest,
                () => [
                    text.sm,
                    styles.marker,
                    variants[marker.variant],
                    marker.render !== undefined && [state.focus, styles.interactive],
                    properties.status === "error" && styles.error,
                ],
                {
                    get "data-variant"() {
                        return marker.variant;
                    },
                    get "data-status"() {
                        return properties.status;
                    },
                },
            )}
        </StatusContext>
    );
}

/** The properties of a part of a marker, the native element's attributes included. */
export type MarkerElementProperties = Omit<JSX.HTMLAttributes<HTMLSpanElement>, "class"> &
    ElementPartProperties;

/** Render the icon of a marker, which its words say alone. */
export function MarkerIcon(properties: MarkerElementProperties): JSX.Element {
    return renderPart("span", "marker-icon", properties, styles.icon, {
        "aria-hidden": "true" as const,
    });
}

/** Render the words of a marker, which a light runs over while their work is under way. */
export function MarkerContent(properties: MarkerElementProperties): JSX.Element {
    const status = useContext(StatusContext);

    return renderPart("span", "marker-content", properties, () => [
        styles.content,
        status() === "running" && styles.running,
    ]);
}

/** Render a detail after the words of a marker, such as a count or how long it took. */
export function MarkerDescription(properties: MarkerElementProperties): JSX.Element {
    return renderPart("span", "marker-description", properties, styles.description);
}
