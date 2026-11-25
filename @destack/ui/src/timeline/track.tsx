import { t } from "@destack/locale";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { text } from "@destack/theme/text";
import { color, radius, space, stroke, swatch } from "@destack/theme/tokens.stylex";
import {
    createMemo,
    createSignal,
    For,
    type JSX,
    omit,
    Show,
    untrack,
    useEnvironment,
    useLocale,
} from "@destack/view";
import { Move, Sweep } from "../interaction/index.ts";
import { mergeProperties } from "../part/index.ts";
import { TimelineBar } from "./bar.tsx";
import {
    type TimelineBarControl,
    TimelineBarContext,
    TimelineTrackContext,
    type TimelinePartProperties,
    useTimeline,
    useTimelineBar,
    useTimelineRow,
} from "./control.ts";
import type { TimelineBarPart } from "./drag.ts";
import { TimelineMarker, TimelineRange } from "./marker.tsx";
import { rowMarker } from "./marker.stylex.ts";
import { places } from "./places.ts";
import { TimelineItem } from "./tree.ts";

/** The styles of a timeline's tracks and what stands on them by default. */
const styles = style.create({
    // leave the pointer to what stands in the lane, as lanes cover one another
    lane: {
        pointerEvents: "none",
        position: "absolute",
        insetBlockStart: 0,
        insetInline: 0,
    },
    // bring a selected bar over the ones that cover it
    raised: {
        zIndex: 1,
    },
    track: {
        position: "relative",
        overflow: "hidden",
        touchAction: "pan-y",
    },
    undated: {
        cursor: "crosshair",
    },
    hint: {
        position: "absolute",
        insetBlock: 0,
        insetInlineStart: space[2],
        display: "flex",
        alignItems: "center",
        color: color.mutedForeground,
        pointerEvents: "none",
        opacity: {
            default: 0,
            [style.when.ancestor(":hover", rowMarker)]: 1,
            [style.when.ancestor(":focus-within", rowMarker)]: 1,
        },
    },
    sweep: {
        position: "absolute",
        insetBlock: space[1],
        display: "flex",
        alignItems: "center",
        overflow: "hidden",
        paddingInline: space[2],
        borderRadius: radius[2],
        backgroundColor: `color-mix(in oklab, ${swatch.accentMark} 20%, transparent)`,
        color: color.foreground,
        whiteSpace: "nowrap",
        fontVariantNumeric: "tabular-nums",
        pointerEvents: "none",
    },
    rollup: {
        position: "absolute",
        insetBlockEnd: space[1],
        height: stroke.ring,
        minWidth: stroke.ring,
        borderRadius: radius.full,
        pointerEvents: "none",
    },
    baseline: {
        insetBlockStart: "auto",
        borderInlineWidth: 0,
        position: "absolute",
        insetBlockEnd: stroke.border,
        height: stroke.ring,
        minWidth: stroke.ring,
        borderRadius: radius.full,
        backgroundColor: { default: color.mutedForeground, [media.forcedColors]: "CanvasText" },
        opacity: 0.5,
        pointerEvents: "none",
    },
    slack: {
        insetBlockEnd: "auto",
        borderInlineWidth: 0,
        position: "absolute",
        top: `calc(50% - ${stroke.border} / 2)`,
        height: stroke.border,
        backgroundColor: color.mutedForeground,
        opacity: 0.6,
        pointerEvents: "none",
    },
    deadline: {
        position: "absolute",
        insetBlock: space[1],
        width: stroke.ring,
        backgroundColor: { default: swatch.destructiveMark, [media.forcedColors]: "CanvasText" },
        pointerEvents: "none",
    },
    inlineMilestone: {
        insetBlock: "auto",
        pointerEvents: "auto",
        borderWidth: { default: null, [media.forcedColors]: "2px" },
        borderStyle: { default: null, [media.forcedColors]: "solid" },
        position: "absolute",
        top: `calc(50% - ${space[2]} / 2)`,
        width: space[2],
        height: space[2],
        marginInlineStart: `calc(${space[2]} / -2)`,
        transform: "rotate(45deg)",
        zIndex: 2,
    },
});

/** Render a row's track: its items' bars, or the room to sweep an undated item's dates out on. */
export function TimelineTrack(properties: TimelinePartProperties): JSX.Element {
    // sweep dates out on the track of an item without any
    const control = useTimeline();
    const { node } = useTimelineRow();
    const isEmpty = (): boolean =>
        !("children" in properties) &&
        control.tree.lanes(node().item.key).length === 0 &&
        control.tree.rollup(node().item.key).length === 0;
    const rest = omit(properties, "children", "xstyle", "style");

    return (
        <TimelineTrackContext value={true}>
            <Show when={!isEmpty()} fallback={<UndatedTrack xstyle={properties.xstyle} />}>
                <div
                    role="gridcell"
                    data-slot="timeline-track"
                    {...rest}
                    {...style.attributes([styles.track, properties.xstyle], properties.style)}
                >
                    {"children" in properties ? properties.children : <TimelineBarList />}
                </div>
            </Show>
        </TimelineTrackContext>
    );
}

/** The properties of the bars of a track. */
export interface TimelineBarListProperties {
    /** Render what stands on the track for an item, its bar and its marks by default. */
    readonly children?: (item: () => TimelineItem) => JSX.Element;
}

/** Render the bars of a row's items, those that overlap in lanes or over one another. */
export function TimelineBarList(properties: TimelineBarListProperties): JSX.Element {
    // place each item in its lane
    const control = useTimeline();
    const { node } = useTimelineRow();
    const bars = createMemo(() =>
        control.tree
            .lanes(node().item.key)
            .flatMap((lane, index) => lane.map((item) => ({ key: item.key, lane: index }))),
    );

    return (
        <>
            <For each={bars()} keyed={(bar) => bar.key}>
                {(bar) => (
                    <TrackItem item={untrack(() => bar().key)} lane={bar().lane}>
                        {properties.children}
                    </TrackItem>
                )}
            </For>

            {/* The items under a folded item, small along its row's foot */}
            <For each={control.tree.rollup(node().item.key)}>
                {(item) => (
                    <span
                        data-slot="timeline-rollup"
                        aria-hidden="true"
                        {...style.attrs(
                            styles.rollup,
                            places.span(
                                control.window.share(control.tree.drawn(item).start),
                                control.window.share(control.tree.drawn(item).end) -
                                    control.window.share(control.tree.drawn(item).start),
                            ),
                            places.fill(swatch[`${TimelineItem.hue(item)}Mark`], color.background),
                        )}
                    />
                )}
            </For>
        </>
    );
}

/** Render an item's place on the track, above and below the bars it overlaps. */
function TrackItem(properties: {
    /** The key of the item. */
    readonly item: string;
    /** Its place among the bars that overlap, from the top. */
    readonly lane: number;
    /** Render what stands on the track for the item, its bar and marks when absent. */
    readonly children: ((item: () => TimelineItem) => JSX.Element) | undefined;
}): JSX.Element {
    // place the bar along the window
    const control = useTimeline();
    const { node } = useTimelineRow();
    const height = (): number => control.rowHeight(node());
    const locale = useLocale();
    const key = untrack(() => properties.item);
    const item = () => control.drag.preview(key);
    const from = () => control.window.share(control.tree.drawn(item()).start);
    const to = () => control.window.share(control.tree.drawn(item()).end);

    // drag the bar, its edges and its links
    const elements = new Map<TimelineBarPart, HTMLElement>();
    const isDraggable = untrack(
        () =>
            control.options.onItemsChange !== undefined ||
            control.options.onLinkCreate !== undefined,
    );
    const drags = isDraggable
        ? control.drag.draggables(key, (part) => elements.get(part))
        : undefined;

    // drag the done share at its grip, previewed until let go
    const [moving, setMoving] = createSignal<number>();
    const commitProgress = (done: number): void =>
        control.tree.commit([{ ...item(), progress: Math.round(done * 100) / 100 }]);
    let element: HTMLElement | undefined;
    const doneAt = (clientX: number, lane: HTMLElement): number => {
        // mirror the pointer's place right to left
        const bounds = lane.getBoundingClientRect();
        const along = (clientX - bounds.left) / bounds.width;
        const at = locale.direction === "rtl" ? 1 - along : along;

        return Math.min(1, Math.max(0, (at - from()) / (to() - from())));
    };
    const grip = new Move(
        {
            keys: [],
            onMove: (event) => {
                if (element !== undefined) {
                    setMoving(doneAt(event.clientX, element));
                }
            },
            onMoveEnd: () => {
                const done = moving();
                setMoving(undefined);
                if (done !== undefined) {
                    commitProgress(done);
                }
            },
        },
        useEnvironment().clock,
    );

    // share the bar's place and drags with its parts
    const bar: TimelineBarControl = {
        key,
        item,
        lane: () => properties.lane,
        from,
        to,
        hue: () => TimelineItem.hue(item()),
        variant: () => item().variant ?? "tint",
        isMilestone: () => item().start !== undefined && item().start === item().end,
        isMoving: () =>
            control.drag.manager.isDragging(key) &&
            control.drag.manager.operation()?.kind !== "link",
        open: () =>
            item().start === undefined ? "start" : item().end === undefined ? "end" : undefined,
        progress: () => moving() ?? item().progress,
        commitProgress,
        drags,
        progressDrag: grip.attributes(),
        addCard: () => undefined,
        register: (part) => (created) => elements.set(part, created),
    };

    return (
        <TimelineBarContext value={bar}>
            <div
                data-slot="timeline-lane"
                data-lane={properties.lane}
                ref={(created) => (element = created)}
                {...style.attrs(
                    styles.lane,
                    control.selection.isSelected(key) && styles.raised,
                    places.row(properties.lane * height(), height()),
                )}
            >
                {properties.children === undefined ? <TrackParts /> : properties.children(item)}
            </div>
        </TimelineBarContext>
    );
}

/** Render an item's bar and marks, which stand on a track by default. */
function TrackParts(): JSX.Element {
    // mark the times the item has
    const control = useTimeline();
    const { item, hue } = useTimelineBar();
    const end = (): number => control.tree.drawn(item()).end;

    return (
        <>
            <TimelineBar />
            {/* The planned dates beneath the bar */}
            <Show when={item().baseline}>
                {(planned) => (
                    <TimelineRange
                        data-slot="timeline-bar-baseline"
                        aria-hidden="true"
                        start={planned().start}
                        end={planned().end}
                        xstyle={styles.baseline}
                    />
                )}
            </Show>

            {/* The time the item can slip without moving the plan's end */}
            <Show when={item().slack}>
                {(slack) => (
                    <TimelineRange
                        data-slot="timeline-bar-slack"
                        aria-hidden="true"
                        start={end()}
                        end={end() + slack()}
                        xstyle={styles.slack}
                    />
                )}
            </Show>

            {/* The time the item is due */}
            <Show when={item().deadline}>
                {(deadline) => (
                    <TimelineMarker
                        data-slot="timeline-bar-deadline"
                        aria-hidden="true"
                        at={deadline()}
                        xstyle={styles.deadline}
                    />
                )}
            </Show>

            {/* The milestones along the item */}
            <For each={item().milestones ?? []}>
                {(milestone) => (
                    <TimelineMarker
                        data-slot="timeline-bar-milestone"
                        role="img"
                        aria-label={milestone.label}
                        title={milestone.label}
                        at={milestone.at}
                        xstyle={[
                            styles.inlineMilestone,
                            places.fill(swatch[`${hue()}Solid`], swatch[`${hue()}Label`]),
                        ]}
                    />
                )}
            </For>
        </>
    );
}

/** Render the track of an item without dates, on which a sweep sets them. */
function UndatedTrack(properties: {
    /** The StyleX styles applied after the track's styles. */
    readonly xstyle: style.Styles | undefined;
}): JSX.Element {
    // sweep a span out from the pressed time, its start and end snapped as edges
    const control = useTimeline();
    const locale = useLocale();
    const node = useTimelineRow().node;
    let element: HTMLElement | undefined;

    // read the time under the pointer, mirrored right to left
    const atPointer = (x: number): number => {
        if (element === undefined) {
            throw new TypeError("timeline sweep read before its track mounted");
        }
        const bounds = element.getBoundingClientRect();
        const along = (x - bounds.left) / bounds.width;
        const share = locale.direction === "rtl" ? 1 - along : along;

        return control.window.instant(share);
    };

    // hand on the swept dates, a step long at the least
    const sweep = new Sweep(
        {
            at: (x) => atPointer(x),
            span: (anchor, at) => ({
                start: control.window.snap(Math.min(anchor, at), "start"),
                end: control.window.snap(Math.max(anchor, at), "end"),
            }),
            canStart: () => control.options.onItemsChange !== undefined,
            onSweep: (swept) => {
                const end = Math.max(swept.end, control.window.step(swept.start, 1, "end"));
                control.tree.commit([{ ...node().item, start: swept.start, end }]);
            },
        },
        useEnvironment().clock,
    );

    return (
        <div
            role="gridcell"
            data-slot="timeline-track"
            data-undated=""
            {...mergeProperties(
                { ref: (created: HTMLDivElement) => (element = created) },
                sweep.attributes(),
            )}
            {...style.attrs(styles.track, styles.undated, properties.xstyle)}
        >
            {/* The hint on a hovered or focused row, before any sweep */}
            <Show when={sweep.span() === undefined && control.options.onItemsChange !== undefined}>
                <span data-slot="timeline-undated-hint" {...style.attrs(text.xs, styles.hint)}>
                    {locale.render(t`Drag to set dates`)}
                </span>
            </Show>

            {/* The swept span and its dates */}
            <Show when={sweep.span()}>
                {(swept) => (
                    <span
                        data-slot="timeline-bar-sweep"
                        {...style.attrs(
                            text.xs,
                            styles.sweep,
                            places.span(
                                control.window.share(swept().start),
                                control.window.share(swept().end) -
                                    control.window.share(swept().start),
                            ),
                        )}
                    >
                        {control.window.rangeLabel({ ...node().item, ...swept() })}
                    </span>
                )}
            </Show>
        </div>
    );
}
