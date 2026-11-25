import { t } from "@destack/locale";
import type { Action } from "@destack/object";
import { present } from "@destack/schema";
import * as style from "@destack/style";
import { color, radius, stroke } from "@destack/theme/tokens.stylex";
import {
    handle,
    type JSX,
    omit,
    Show,
    untrack,
    useEnvironment,
    useLocale,
    useNow,
} from "@destack/view";
import { DragAnnouncer, useDragManager } from "../drag/index.ts";
import {
    clearSelection,
    openItem,
    selectAll,
    toggleSelection,
    zoomIn,
    zoomOut,
    zoomToFit,
} from "../action/index.ts";
import { Move, Zoom } from "../interaction/index.ts";
import { contentOr, mergeProperties, renderPart } from "../part/index.ts";
import {
    type TimelineOptions,
    TimelineControl,
    TimelineContext,
    useTimeline,
    type TimelinePartProperties,
} from "./control.ts";
import { linkItem, moveRowDown, moveRowUp, panLeft, panRight } from "./actions.ts";
import { TimelineHeader } from "./header.tsx";
import { TimelineLinkLayer } from "./links.tsx";
import { TimelineGrid } from "./marker.tsx";
import { places } from "./places.ts";
import { TimelineRowList } from "./row.tsx";
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

/** Render items on a time axis as a tree of bars, after Jaeger's traces. */
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
    );

    // handle the timeline's keys on its rows
    const isChangeable = untrack(() => properties.onItemsChange !== undefined);
    const isLinkable = untrack(() => properties.onLinkCreate !== undefined);
    const pan = (sign: 1 | -1): void => {
        // pan the way the arrow points, mirrored in right-to-left text
        control.window.pan((locale.direction === "rtl" ? -sign : sign) * PAN_STEP);
    };
    const rowAt = (element: Element | null): string | undefined => {
        // find the row the person is on, none while a drag takes the keys
        const row = element?.closest<HTMLElement>(ROW_SLOT);

        return control.drag.manager.operation() === undefined ? row?.dataset["key"] : undefined;
    };
    const onRow = (action: Action, isEnabled: () => boolean, run: (key: string) => void) =>
        handle(action, {
            isEnabled: (element) => rowAt(element) !== undefined && isEnabled(),
            run: ({ element }) => run(present(rowAt(element), "the row an action runs on")),
        });
    const isMovable = (): boolean => control.options.onRowMove !== undefined;
    const handlers = [
        // select rows
        onRow(
            toggleSelection,
            () => true,
            (key) => control.selection.toggle(key),
        ),
        handle(selectAll, { run: () => control.selection.selectAll(control.tree.collection) }),
        handle(clearSelection, {
            isEnabled: () =>
                control.drag.manager.operation() === undefined &&
                control.selection.values().length > 0,
            run: () => control.selection.clear(),
        }),

        // open, move and link a row
        onRow(
            openItem,
            () => control.options.onAction !== undefined,
            (key) => control.open(key),
        ),
        onRow(moveRowUp, isMovable, (key) => control.stepRow(key, -1)),
        onRow(moveRowDown, isMovable, (key) => control.stepRow(key, 1)),
        onRow(
            linkItem,
            () => isLinkable,
            (key) => control.drag.lift(key, "link-end"),
        ),

        // zoom and pan the window
        handle(zoomIn, { run: () => control.window.zoom(ZOOM_STEP, 0.5) }),
        handle(zoomOut, { run: () => control.window.zoom(1 / ZOOM_STEP, 0.5) }),
        handle(zoomToFit, { run: () => control.window.fit() }),
        handle(panLeft, { run: () => pan(-1) }),
        handle(panRight, { run: () => pan(1) }),
    ];
    const keys = {
        ref: handlers,
        onKeyDown: (event: KeyboardEvent) => {
            // take keys on the rows alone, leaving the toolbar's and the links' keys to them
            const focused = control.focus.current();
            const isOnRow =
                event.target instanceof Element && event.target.closest(ROW_SLOT) !== null;
            if (event.defaultPrevented || !isOnRow || control.drag.manager.key(event)) {
                return;
            }

            // leave arrows with a modifier to the actions
            const isModified = event.altKey || event.ctrlKey || event.metaKey;
            if (isModified && event.key.startsWith("Arrow")) {
                return;
            }

            // unfold the focused row's siblings with an asterisk, as a tree does
            if (focused !== undefined && event.key === "*") {
                event.preventDefault();
                control.tree.expandSiblings(focused);
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
            {renderPart(
                "div",
                "timeline",
                rest,
                styles.timeline,
                mergeProperties(
                    {
                        get "data-linking"() {
                            return control.drag.manager.operation()?.kind === "link"
                                ? ""
                                : undefined;
                        },
                        get children() {
                            return (
                                <>
                                    {contentOr(properties, () => (
                                        <>
                                            <TimelineHeader />
                                            <TimelineBody />
                                        </>
                                    ))}
                                    {control.rowLength.render()}
                                </>
                            );
                        },
                    },
                    keys,
                ),
            )}
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
    const rest = omit(properties, "children");
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

    // pan by dragging empty track, the window following the pointer
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

    return renderPart(
        "div",
        "timeline-body",
        rest,
        styles.body,
        mergeProperties(
            mergeProperties(
                mergeProperties(mergeProperties(zoom.attributes(), pan), drag.attributes()),
                { ref: (created: HTMLDivElement) => control.attachScroller(created) },
            ),
            {
                role: "treegrid" as const,
                get "aria-label"() {
                    return locale.render(t`Timeline`);
                },
                get "aria-rowcount"() {
                    return control.tree.rows().length;
                },
                "aria-multiselectable": "true" as const,
                get children() {
                    return (
                        <div
                            {...style.attrs(
                                styles.rows,
                                places.height(control.virtualizer.totalSize()),
                            )}
                        >
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
                            {contentOr(properties, () => (
                                <>
                                    <TimelineGrid />
                                    <TimelineLinkLayer />
                                    <TimelineRowList />
                                </>
                            ))}
                        </div>
                    );
                },
            },
        ),
    );
}
