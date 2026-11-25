import { type Direction, t } from "@destack/locale";
import * as style from "@destack/style";
import { state } from "@destack/theme/state";
import { color, radius, space, stroke, swatch } from "@destack/theme/tokens.stylex";
import { type JSX, omit, Show, untrack, useLocale } from "@destack/view";
import { mergeProperties, renderPart } from "../part/index.ts";
import { leave, type TimelineElementProperties, useTimeline, useTimelineBar } from "./control.ts";
import type { TimelineBarDrags } from "./drag.ts";
import { rowMarker } from "./marker.stylex.ts";
import { places } from "./places.ts";

/** The share of an item an arrow key on its progress handle steps by. */
const PROGRESS_STEP = 0.1;

/** The styles of a bar's handles. */
const styles = style.create({
    handle: {
        position: "absolute",
        insetBlock: 0,
        width: space[2],
        cursor: "ew-resize",
        zIndex: 2,
    },
    // grab an edge from both its sides, more from the outside, as bars often meet
    start: {
        insetInlineStart: `calc(-1 * ${space[1]})`,
    },
    end: {
        insetInlineEnd: `calc(-1 * ${space[1]})`,
    },
    beforeBar: {
        insetInlineEnd: "100%",
    },
    afterBar: {
        insetInlineStart: "100%",
    },
    grip: {
        position: "absolute",
        insetBlockEnd: `calc(-1 * ${space[1]})`,
        width: space[2],
        height: space[2],
        marginInlineStart: `calc(${space[2]} / -2)`,
        borderWidth: stroke.ring,
        borderStyle: "solid",
        borderColor: swatch.accentMark,
        borderRadius: radius.full,
        backgroundColor: color.background,
        cursor: "ew-resize",
        touchAction: "none",
        zIndex: 3,
        opacity: {
            default: 0,
            [style.when.ancestor(":hover", rowMarker)]: 1,
            [style.when.ancestor(":focus-within", rowMarker)]: 1,
        },
    },
    connector: {
        position: "absolute",
        top: `calc(50% - ${space[2]} / 2)`,
        width: space[2],
        height: space[2],
        borderWidth: stroke.ring,
        borderStyle: "solid",
        borderColor: swatch.accentMark,
        borderRadius: radius.full,
        backgroundColor: color.background,
        cursor: "crosshair",
        zIndex: 3,
        opacity: {
            default: 0,
            [style.when.ancestor(":hover", rowMarker)]: 1,
            [style.when.ancestor(":focus-within", rowMarker)]: 1,
            [style.when.ancestor("[data-linking]")]: 1,
        },
    },
    connectorStart: {
        marginInlineEnd: space[3],
    },
    connectorEnd: {
        marginInlineStart: space[3],
    },
});

/** What a handle drags: an edge of the bar, how much of it is done, or a link from an edge. */
export type TimelineBarEdge = "start" | "end" | "progress" | "link-start" | "link-end";

/** The properties of a handle of a bar. */
export type TimelineBarHandleProperties = TimelineElementProperties & {
    /** What it drags. */
    readonly edge: TimelineBarEdge;
};

/** Render a handle of a bar, where its timeline takes what the handle changes. */
export function TimelineBarHandle(properties: TimelineBarHandleProperties): JSX.Element {
    // show a link's handle where links are drawn, and the others where a span changes
    const control = useTimeline();
    const bar = useTimelineBar();
    const edge = untrack(() => properties.edge);
    const isShown = (): boolean =>
        edge === "link-start" || edge === "link-end"
            ? control.options.onLinkCreate !== undefined
            : control.options.onItemsChange !== undefined && !bar.isMilestone();

    return (
        <Show when={isShown()}>
            {edge === "progress" ? (
                <ProgressHandle {...properties} />
            ) : (
                <EdgeHandle {...properties} edge={edge} />
            )}
        </Show>
    );
}

/** What each handle of an edge drags, what it says, and where it stands. */
const EDGES = {
    start: {
        slot: "timeline-bar-handle",
        edge: "start",
        drag: (drags: TimelineBarDrags) => drags.start,
        label: (item: string) => t`Change the start of ${item}`,
        xstyle: [styles.handle, styles.start],
    },
    end: {
        slot: "timeline-bar-handle",
        edge: "end",
        drag: (drags: TimelineBarDrags) => drags.end,
        label: (item: string) => t`Change the end of ${item}`,
        xstyle: [styles.handle, styles.end],
    },
    "link-start": {
        slot: "timeline-bar-connector",
        edge: "start",
        drag: (drags: TimelineBarDrags) => drags.linkStart,
        label: (item: string) => t`Link from the start of ${item}`,
        xstyle: [styles.connector, styles.connectorStart, styles.beforeBar],
    },
    "link-end": {
        slot: "timeline-bar-connector",
        edge: "end",
        drag: (drags: TimelineBarDrags) => drags.linkEnd,
        label: (item: string) => t`Link from the end of ${item}`,
        xstyle: [styles.connector, styles.connectorEnd, styles.afterBar],
    },
} as const;

/** Render the handle that drags an edge, or a link from it. */
function EdgeHandle(
    properties: TimelineElementProperties & {
        /** What it drags. */
        readonly edge: keyof typeof EDGES;
    },
): JSX.Element {
    // drag the edge where the bar's drags are
    const locale = useLocale();
    const bar = useTimelineBar();
    const key = untrack(() => properties.edge);
    const { slot, edge, drag, label, xstyle } = EDGES[key];
    const rest = omit(properties, "edge");
    if (bar.drags === undefined) {
        return null;
    }

    return renderPart(
        "span",
        slot,
        rest,
        xstyle,
        mergeProperties(mergeProperties({ ref: bar.register(key) }, drag(bar.drags)), {
            tabindex: -1,
            onPointerDown: leave,
            onClick: leave,
            "data-edge": edge,
            get "aria-label"() {
                return locale.render(label(bar.item().label));
            },
        }),
    );
}

/** Render the handle that sets how much is done, by a drag or the arrow keys. */
function ProgressHandle(properties: TimelineBarHandleProperties): JSX.Element {
    // step by a tenth along the text's direction, or go to either end
    const locale = useLocale();
    const bar = useTimelineBar();
    const rest = omit(properties, "edge");
    const onKeyDown = (event: KeyboardEvent): void => {
        const done = progressAfter(event.key, bar.progress() ?? 0, locale.direction);
        if (done !== undefined) {
            event.preventDefault();
            event.stopPropagation();
            bar.commitProgress(done);
        }
    };

    return renderPart(
        "span",
        "timeline-bar-progress-grip",
        rest,
        () => [state.touch, styles.grip, places.at(bar.progress() ?? 0)],
        mergeProperties(mergeProperties(bar.progressDrag, { onKeyDown }), {
            role: "slider" as const,
            tabindex: -1,
            onPointerDown: leave,
            onClick: leave,
            "aria-valuemin": 0,
            "aria-valuemax": 100,
            get "aria-label"() {
                return locale.render(t`How much of ${bar.item().label} is done`);
            },
            get "aria-valuenow"() {
                return percentOf(bar.progress());
            },
            get "aria-valuetext"() {
                const done = bar.progress();

                return done === undefined ? undefined : locale.number(done, { style: "percent" });
            },
        }),
    );
}

/** Read a done share as a whole percent, none while unknown. */
function percentOf(done: number | undefined): number | undefined {
    return done === undefined ? undefined : Math.round(done * 100);
}

/** Read the done share a key on the grip steps to: a tenth along the text, or an end. */
function progressAfter(key: string, done: number, direction: Direction): number | undefined {
    // step forward and backward along the text's direction
    const forward = direction === "rtl" ? "ArrowLeft" : "ArrowRight";
    const backward = direction === "rtl" ? "ArrowRight" : "ArrowLeft";
    if (key === forward) {
        return Math.min(1, done + PROGRESS_STEP);
    } else if (key === backward) {
        return Math.max(0, done - PROGRESS_STEP);
    }
    // go to either end
    else if (key === "Home") {
        return 0;
    } else if (key === "End") {
        return 1;
    }
    // leave other keys alone
    else {
        return undefined;
    }
}
