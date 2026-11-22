import { Shortcut, t } from "@destack/locale";
import * as style from "@destack/style";
import { color, radius, stroke } from "@destack/theme/tokens.stylex";
import { type JSX, omit, Show, untrack, useEnvironment, useLocale, useNow } from "@destack/view";
import { DragAnnouncer, useDragManager } from "../drag/index.ts";
import { useCommandPlatform } from "../kbd/index.ts";
import { Move, Zoom } from "../interaction/index.ts";
import { mergeProperties } from "../part/index.ts";
import {
    type TimelineOptions,
    TimelineControl,
    TimelineContext,
    useTimeline,
    type TimelinePartProperties,
} from "./control.ts";
import { places } from "./places.ts";
import { TimelineItem } from "./tree.ts";

/** The styles of a timeline and its body. */
const styles = style.create({
    timeline: {
        display: "flex",
        flexDirection: "column",
        minHeight: 0,
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
        borderRadius: radius[4],
        overflow: "hidden",
        outlineStyle: "none",
    },
    body: {
        flexGrow: 1,
        userSelect: "none",
        minHeight: 0,
        overflowY: "auto",
        overflowX: "hidden",
    },
    rows: {
        position: "relative",
    },
    probe: {
        display: "grid",
        height: 0,
    },
    probeTrack: {
        gridColumn: "-2 / -1",
    },
});

/** The rows, which take the timeline's keys. */
const ROW_SLOT = "[data-slot=timeline-row]";

/** The share of the window Alt and an arrow pan it by. */
const PAN_STEP = 0.25;

/** The factor a key or a button zooms by. */
const ZOOM_STEP = 1.5;

/** How often the now marker moves: each minute, as the calendar's now line does. */
const NOW_INTERVAL = { minutes: 1 } as const;

/** The elements a press on pans the window from: a dated item's track outside its bar. */
const EMPTY_TRACK = "[data-slot=timeline-track]:not([data-undated])";

/** The properties of a timeline, the native element's attributes included. */
export interface TimelineProperties
    extends TimelineOptions, Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> {
    /** The parts, such as a header, a body and a minimap. */
    readonly children?: JSX.Element;
    /** The StyleX styles applied after the timeline's. */
    readonly xstyle?: style.Styles;
}

/** Render items on a time axis as a tree of bars, after Linear's timeline and Jaeger's traces. */
export function Timeline(properties: TimelineProperties): JSX.Element {
    // share the window, folds and drags with the parts
    const locale = useLocale();
    const drags = useDragManager();
    const environment = useEnvironment();
    const control = new TimelineControl(
        properties,
        drags.manager,
        locale,
        environment,
        useNow(NOW_INTERVAL),
    );
    const rest = omit(
        properties,
        "items",
        "links",
        "axis",
        "window",
        "defaultWindow",
        "onWindowChange",
        "onItemsChange",
        "propagate",
        "onAction",
        "onRowMove",
        "selection",
        "canLink",
        "onLinkCreate",
        "onLinkDelete",
        "onLinkAction",
        "timeZone",
        "snap",
        "hideNonWorking",
        "workWeek",
        "rowHeight",
        "labelWidth",
        "onLabelWidthChange",
        "children",
        "xstyle",
        "style",
    );

    // handle the timeline's keys on its rows
    const isChangeable = untrack(() => properties.onItemsChange !== undefined);
    const isLinkable = untrack(() => properties.onLinkCreate !== undefined);
    const isCommand = useCommandPlatform();
    const keys = {
        onKeyDown: (event: KeyboardEvent) => {
            // take keys on the rows alone, leaving the toolbar's and the links' keys to them
            const focused = control.focus.current();
            const isOnRow =
                event.target instanceof Element && event.target.closest(ROW_SLOT) !== null;
            if (event.defaultPrevented || !isOnRow || control.drag.manager.key(event)) {
                return;
            }

            // open the focused row's item with Enter
            if (focused !== undefined && event.key === "Enter") {
                event.preventDefault();
                control.open(focused);
            }
            // move the focused row a place up or down with Mod+Shift and the arrows, as Notion does
            else if (
                focused !== undefined &&
                control.options.onRowMove !== undefined &&
                (Shortcut.matches("mod+shift+up", event, isCommand()) ||
                    Shortcut.matches("mod+shift+down", event, isCommand()))
            ) {
                event.preventDefault();
                control.stepRow(focused, event.key === "ArrowUp" ? -1 : 1);
            }
            // unfold the focused row's siblings with an asterisk, as a tree does
            else if (focused !== undefined && event.key === "*") {
                event.preventDefault();
                control.tree.expandSiblings(focused);
            }
            // toggle the focused row's selection with X
            else if (focused !== undefined && (event.key === "x" || event.key === "X")) {
                event.preventDefault();
                control.selection.toggle(focused);
            }
            // select every row with Mod+A, and clear the selection with Escape
            else if (Shortcut.matches("mod+a", event, isCommand())) {
                event.preventDefault();
                control.selection.selectAll(control.tree.collection);
            } else if (event.key === "Escape" && control.selection.values().length > 0) {
                event.preventDefault();
                control.selection.clear();
            }
            // lift the focused row's bar with Space or place an undated item there
            else if (focused !== undefined && isChangeable && event.key === " ") {
                event.preventDefault();
                if (TimelineItem.isDated(control.tree.item(focused))) {
                    control.drag.lift(focused, event.shiftKey ? "end" : "body");
                } else {
                    control.schedule(focused);
                }
            }
            // lift a link from the focused row's end with L
            else if (
                focused !== undefined &&
                isLinkable &&
                (event.key === "l" || event.key === "L")
            ) {
                event.preventDefault();
                control.drag.lift(focused, "link-end");
            } else if (event.key === "+" || event.key === "=") {
                event.preventDefault();
                control.window.zoom(ZOOM_STEP, 0.5);
            } else if (event.key === "-") {
                event.preventDefault();
                control.window.zoom(1 / ZOOM_STEP, 0.5);
            } else if (event.key === "0") {
                event.preventDefault();
                control.window.fit();
            }
            // pan a quarter of the window with Alt and the inline arrows, mirrored right to left
            else if (event.altKey && (event.key === "ArrowLeft" || event.key === "ArrowRight")) {
                event.preventDefault();
                const isLater = (event.key === "ArrowRight") === (locale.direction === "ltr");
                control.window.pan(isLater ? PAN_STEP : -PAN_STEP);
            }
            // fold or unfold the focused row on the inline arrows
            else if (control.fold(event, locale.direction)) {
                event.preventDefault();
            }
            // move the focus, extending the selection with Shift
            else {
                const target = control.focus.move(event, locale.direction);
                if (target !== undefined && focused !== undefined && event.shiftKey) {
                    control.selection.extendFrom(focused, target, control.tree.collection);
                }
            }
        },
        onFocusOut: (event: FocusEvent & { readonly currentTarget: Element }) =>
            control.focus.focusOut(event),
    };

    return (
        <TimelineContext value={control}>
            <div
                data-slot="timeline"
                data-linking={control.drag.manager.operation()?.kind === "link" ? "" : undefined}
                {...mergeProperties(keys, rest)}
                {...style.attributes([styles.timeline, properties.xstyle], properties.style)}
            >
                {properties.children}
            </div>
            <Show when={!drags.isShared}>
                <DragAnnouncer manager={control.drag.manager} />
            </Show>
        </TimelineContext>
    );
}

/** Render the scroller of the rows, which pans and zooms. */
export function TimelineBody(properties: TimelinePartProperties): JSX.Element {
    // zoom by pinches and trackpads around the pointer
    const control = useTimeline();
    const locale = useLocale();
    const mirror = (share: number) => (locale.direction === "rtl" ? -share : share);
    const rest = omit(properties, "children", "xstyle", "style");
    let track: HTMLElement | undefined;
    const zoom = new Zoom({
        onZoom: (event) => {
            const bounds = track?.getBoundingClientRect();
            if (bounds !== undefined && bounds.width > 0) {
                const along = (event.clientX - bounds.left) / bounds.width;
                const around = locale.direction === "rtl" ? 1 - along : along;
                control.window.zoom(event.scale, around, mirror(-event.panX / bounds.width));
            }
        },
    });

    // pan on a horizontal wheel or a shifted one, leaving vertical scrolling to the rows
    const pan = {
        onWheel: (event: WheelEvent) => {
            // read the wheel's travel along the axis, mirrored right to left
            const delta = event.shiftKey ? event.deltaY : mirror(event.deltaX);
            const width = control.window.width();
            const isPanning =
                !event.ctrlKey &&
                delta !== 0 &&
                width !== undefined &&
                Math.abs(event.deltaY) <= Math.abs(delta);
            if (isPanning) {
                event.preventDefault();
                control.window.pan(delta / width);
            }
        },
    };

    // pan by dragging empty track, the window following the pointer as in Linear
    const drag = new Move(
        {
            keys: [],
            canStart: (event) =>
                event.target instanceof Element && event.target.matches(EMPTY_TRACK),
            onMove: (event) => {
                const width = control.window.width();
                const travel = locale.direction === "rtl" ? event.deltaX : -event.deltaX;
                if (width !== undefined && width > 0) {
                    control.window.pan(travel / width);
                }
            },
        },
        useEnvironment().clock,
    );

    return (
        <div
            data-slot="timeline-body"
            role="treegrid"
            aria-label={locale.render(t`Timeline`)}
            aria-rowcount={control.tree.rows().length}
            aria-multiselectable="true"
            {...mergeProperties(
                mergeProperties(
                    mergeProperties(mergeProperties(zoom.attributes(), pan), drag.attributes()),
                    rest,
                ),
                { ref: (created: HTMLDivElement) => control.attachScroller(created) },
            )}
            {...style.attributes([styles.body, properties.xstyle], properties.style)}
        >
            <div {...style.attrs(styles.rows, places.height(control.virtualizer.totalSize()))}>
                {/* Track width probe */}
                <div
                    aria-hidden="true"
                    {...style.attrs(styles.probe, places.columns(control.template()))}
                >
                    <div
                        ref={(created) => {
                            track = created;
                            control.window.attachTrack(created);
                        }}
                        {...style.attrs(styles.probeTrack)}
                    />
                </div>

                {/* Parts */}
                {properties.children}
            </div>
        </div>
    );
}
