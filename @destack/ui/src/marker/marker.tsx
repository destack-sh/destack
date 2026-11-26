import { t } from "@destack/locale";
import { field } from "@destack/object";
import { media } from "@destack/style/media.stylex";
import * as style from "@destack/style";
import { state } from "@destack/theme/state";
import { text } from "@destack/theme/text";
import { color, motion, radius, space, stroke, swatch } from "@destack/theme/tokens.stylex";
import { type Accessor, createContext, type JSX, omit, useContext } from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The light that runs over the words of a status under way. */
const shimmer = style.keyframes({
    from: { backgroundPosition: "150% 0" },
    to: { backgroundPosition: "-50% 0" },
});

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
export type MarkerVariant = keyof typeof MARKER_VARIANTS;

/** The work a marker reports: under way, or failed. */
export type MarkerStatus = keyof typeof MARKER_STATUSES;

/** The status of the nearest marker. */
const StatusContext = createContext<Accessor<MarkerStatus | undefined>>(() => undefined);

/** The layouts of a marker. */
const MARKER_VARIANTS = {
    default: {
        title: t.context("option")`Default`,
        description: t`In line with the stream.`,
    },
    border: {
        title: t.context("option")`Border`,
        description: t`With a line below.`,
    },
    separator: {
        title: t.context("option")`Separator`,
        description: t`Between two lines, as a day is.`,
    },
};

/** The work a marker reports. */
const MARKER_STATUSES = {
    running: { title: t.context("option")`Running` },
    error: { title: t.context("option")`Error` },
};

/** The properties people change in a marker's controls and inspector. */
const MARKER_PROPERTIES = {
    variant: field
        .enum(MARKER_VARIANTS)
        .labeled(t.context("label")`Variant`)
        .described(t`Whether the marker sits in line, above a line or between two.`)
        .default("default"),
    status: field
        .enum(MARKER_STATUSES)
        .labeled(t.context("label")`Status`)
        .described(t`The work it reports, which its words say too.`)
        .optional(),
};

/** The properties of a marker besides its declared ones: its native attributes. */
export type MarkerAttributes = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> &
    ElementPartProperties & {};

/** Render a note in a stream, such as a status, a day or someone joining. */
export const Marker = defineComponent({
    name: "marker",
    properties: MARKER_PROPERTIES,
    render(properties: RenderProperties<MarkerAttributes, typeof MARKER_PROPERTIES>): JSX.Element {
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
                        variants[properties.variant],
                        properties.render !== undefined && [state.focus, styles.interactive],
                        properties.status === "error" && styles.error,
                    ],
                    {
                        get "data-variant"() {
                            return properties.variant;
                        },
                        get "data-status"() {
                            return properties.status;
                        },
                    },
                )}
            </StatusContext>
        );
    },
});

/** The properties of a marker, its native attributes included. */
export type MarkerProperties = ComponentProperties<typeof Marker>;

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
