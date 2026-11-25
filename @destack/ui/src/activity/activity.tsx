import type { Hue } from "@destack/color";
import { media } from "@destack/style/media.stylex";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, radius, size, space, stroke, swatch } from "@destack/theme/tokens.stylex";
import { createContext, type JSX, omit, useContext } from "@destack/view";
import { type ElementPartProperties, renderPart } from "../part/index.ts";

/** The styles of an activity and its parts. */
const styles = style.create({
    activity: {
        display: "flex",
        flexDirection: "column",
        margin: 0,
        padding: 0,
        listStyleType: "none",
        minWidth: 0,
    },
    item: {
        position: "relative",
        display: "flex",
        alignItems: "flex-start",
        gap: space[3],
        minWidth: 0,
        paddingBlock: space[2],
        // run the line behind the badges, from the first to the last
        "::before": {
            content: "''",
            position: "absolute",
            zIndex: 0,
            insetInlineStart: `calc((${size[2]} - ${stroke.border}) / 2)`,
            insetBlockStart: 0,
            insetBlockEnd: 0,
            borderInlineStartWidth: stroke.border,
            borderInlineStartStyle: "solid",
            borderInlineStartColor: color.border,
        },
    },
    // begin and end the line at the badges
    clipStart: {
        "::before": { insetBlockStart: { default: 0, ":first-child": space[4] } },
    },
    clipEnd: {
        "::before": { height: { default: null, ":last-child": space[4] } },
    },
    condensed: {
        paddingBlock: space[1],
    },
    badge: {
        position: "relative",
        zIndex: 1,
        display: "flex",
        flexShrink: 0,
        alignItems: "center",
        justifyContent: "center",
        width: size[2],
        height: size[2],
        borderRadius: radius.full,
        backgroundColor: color.muted,
        color: color.mutedForeground,
        // keep the line off the badge's edge
        boxShadow: `0 0 0 ${stroke.ring} ${color.background}`,
        outlineStyle: { default: null, [media.forcedColors]: "solid" },
        outlineWidth: { default: null, [media.forcedColors]: stroke.border },
    },
    condensedBadge: {
        width: size[1],
        height: size[1],
        marginInline: `calc((${size[2]} - ${size[1]}) / 2)`,
    },
    condensedRow: {
        minHeight: size[1],
    },
    hue: (background: string, foreground: string) => ({
        backgroundColor: background,
        color: foreground,
    }),
    body: {
        // cover the line where nothing of it stands beside a badge
        position: "relative",
        zIndex: 1,
        display: "flex",
        flexDirection: "column",
        flexGrow: 1,
        gap: space[1],
        minWidth: 0,
        minHeight: size[2],
        justifyContent: "center",
        color: color.mutedForeground,
        overflowWrap: "anywhere",
    },
    actions: {
        display: "flex",
        flexShrink: 0,
        alignItems: "center",
        gap: space[1],
        minHeight: size[2],
    },
    break: {
        position: "relative",
        zIndex: 1,
        height: space[3],
        marginInlineStart: 0,
        borderBlockStartWidth: stroke.border,
        borderBlockStartStyle: "solid",
        borderBlockStartColor: color.border,
        backgroundColor: color.background,
    },
});

/** The ends of the nearest line that end at an item. */
const ClipContext = createContext<() => boolean | "start" | "end" | "both">(() => true);

/** Whether the nearest item takes less room. */
const CondensedContext = createContext<() => boolean>(() => false);

/** The properties of an element of an activity, the native element's attributes included. */
export type ActivityElementProperties<Target extends HTMLElement> = Omit<
    JSX.HTMLAttributes<Target>,
    "class"
> &
    ElementPartProperties;

/** The properties of an activity. */
export type ActivityProperties = ActivityElementProperties<HTMLOListElement> & {
    /** Whether the line ends at the first and the last item, as it does at both by default. */
    readonly clip?: boolean | "start" | "end" | "both";
};

/** Render what happened to something, in order along a line. */
export function Activity(properties: ActivityProperties): JSX.Element {
    const rest = omit(properties, "clip");

    return (
        <ClipContext value={() => properties.clip ?? true}>
            {renderPart("ol", "activity", rest, styles.activity)}
        </ClipContext>
    );
}

/** The properties of one thing that happened. */
export type ActivityItemProperties = ActivityElementProperties<HTMLLIElement> & {
    /** Whether it takes less room, as small changes between bigger ones do. */
    readonly condensed?: boolean;
};

/** Render one thing that happened: its badge beside its body and actions. */
export function ActivityItem(properties: ActivityItemProperties): JSX.Element {
    // end the line where the list does
    const clip = useContext(ClipContext);
    const isClipped = (end: "start" | "end"): boolean =>
        clip() === true || clip() === "both" || clip() === end;
    const rest = omit(properties, "condensed");

    return (
        <CondensedContext value={() => properties.condensed === true}>
            {renderPart(
                "li",
                "activity-item",
                rest,
                () => [
                    styles.item,
                    isClipped("start") && styles.clipStart,
                    isClipped("end") && styles.clipEnd,
                    properties.condensed === true && styles.condensed,
                ],
                {
                    get "data-condensed"() {
                        return properties.condensed === true ? "" : undefined;
                    },
                },
            )}
        </CondensedContext>
    );
}

/** The properties of the badge on the line. */
export type ActivityBadgeProperties = ActivityElementProperties<HTMLDivElement> & {
    /** The swatch it takes, the muted colors by default. */
    readonly color?: Hue | undefined;
};

/** Render the badge of an item on the line, such as an icon or an avatar. */
export function ActivityBadge(properties: ActivityBadgeProperties): JSX.Element {
    const isCondensed = useContext(CondensedContext);
    const rest = omit(properties, "color");

    return renderPart("div", "activity-badge", rest, () => [
        styles.badge,
        isCondensed() && styles.condensedBadge,
        properties.color !== undefined &&
            styles.hue(swatch[`${properties.color}Tint`], swatch[`${properties.color}TintText`]),
    ]);
}

/** Render what happened, in words or as a card. */
export function ActivityBody(properties: ActivityElementProperties<HTMLDivElement>): JSX.Element {
    const isCondensed = useContext(CondensedContext);

    return renderPart("div", "activity-body", properties, () => [
        text.sm,
        styles.body,
        isCondensed() && styles.condensedRow,
    ]);
}

/** Render the actions of an item at its end. */
export function ActivityAddon(properties: ActivityElementProperties<HTMLDivElement>): JSX.Element {
    const isCondensed = useContext(CondensedContext);

    return renderPart("div", "activity-addon", properties, () => [
        styles.actions,
        isCondensed() && styles.condensedRow,
    ]);
}

/** Render a break in the line, such as before the box to comment in. */
export function ActivityBreak(properties: ActivityElementProperties<HTMLLIElement>): JSX.Element {
    return renderPart("li", "activity-break", properties, styles.break, {
        role: "presentation" as const,
    });
}
