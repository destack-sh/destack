import { state } from "@destack/theme/state";
import { Icon } from "@destack/ui/icon";
import caretLeft from "@destack/icon/phosphor/caret-left";
import caretRight from "@destack/icon/phosphor/caret-right";
import type { Hue } from "@destack/color";
import { type Direction, t } from "@destack/locale";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, radius, space, shadow, stroke, swatch, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    createEffect,
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
import { Move, Press, Sweep } from "../interaction/index.ts";
import { mergeProperties } from "../part/index.ts";
import { Tooltip, TooltipContent, TooltipTrigger } from "../tooltip/index.ts";
import { type TimelineBarVariant, useTimeline, useTimelineRow } from "./control.ts";
import type { TimelineBarPart } from "./drag.ts";
import { places } from "./places.ts";
import { TimelineItem } from "./tree.ts";
import { rowMarker } from "./marker.stylex.ts";

/** The color a bar takes when neither it nor its item sets one. */
export const DEFAULT_COLOR: Hue = "blue";

/** The side of a milestone's diamond in pixels, between a dot and a bar's height. */
export const MILESTONE_PIXELS = 12;

/** The pixels a link runs out of its bar's end before it turns down to the next. */
export const LINK_BEND = 8;

/** The side of a milestone's diamond as a length. */
const MILESTONE_SIZE = `${String(MILESTONE_PIXELS)}px`;

/** The share of the track past which a short bar's label is written before it. */
const LABEL_FLIP = 0.75;

/** The share of an item an arrow key on its progress grip steps by. */
const PROGRESS_STEP = 0.1;

/** The styles of a timeline's bars. */
const styles = style.create({
    lane: {
        position: "absolute",
        insetBlockStart: 0,
        insetInline: 0,
    },
    track: {
        position: "relative",
        overflow: "hidden",
        touchAction: "pan-y",
    },
    summary: {
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
        backgroundColor: color.muted,
        color: color.mutedForeground,
    },
    bar: {
        position: "absolute",
        insetBlock: space[1],
        textAlign: "start",
        minWidth: stroke.ring,
        borderRadius: radius[2],
        whiteSpace: "nowrap",
        overflow: "hidden",
        textOverflow: "ellipsis",
        paddingInline: space[1],
        cursor: "default",
        zIndex: 1,
    },
    movable: {
        cursor: { default: "grab", ":active": "grabbing" },
    },
    clipped: {
        color: "transparent",
    },
    beside: {
        position: "absolute",
        insetBlock: space[1],
        zIndex: 1,
        display: "flex",
        alignItems: "center",
        marginInline: `${String(LINK_BEND * 2)}px`,
        paddingInline: space[1],
        borderRadius: radius[1],
        backgroundColor: color.background,
        backgroundImage: {
            default: null,
            [style.when.ancestor(":hover", rowMarker)]: {
                default: null,
                [media.hover]: `linear-gradient(${color.hover}, ${color.hover})`,
            },
        },
        whiteSpace: "nowrap",
        color: color.mutedForeground,
        pointerEvents: "none",
    },
    cardLabel: {
        display: "block",
        fontWeight: weight.medium,
    },
    cardDetail: {
        display: "block",
        opacity: 0.8,
        fontVariantNumeric: "tabular-nums",
    },
    outside: {
        position: "absolute",
        insetBlock: space[1],
        zIndex: 2,
        display: "flex",
        alignItems: "center",
        gap: space[1],
        maxWidth: "40%",
        paddingInline: space[2],
        borderRadius: radius.full,
        backgroundColor: color.muted,
        color: color.mutedForeground,
        cursor: "pointer",
    },
    outsideStart: {
        insetInlineStart: space[1],
    },
    outsideEnd: {
        insetInlineEnd: space[1],
    },
    outsideLabel: {
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
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
    lifted: {
        boxShadow: shadow.overlay,
        zIndex: 3,
    },
    dates: {
        position: "absolute",
        insetBlockStart: "50%",
        transform: "translateY(-50%)",
        marginInlineStart: space[3],
        zIndex: 4,
        paddingBlock: space[1],
        paddingInline: space[2],
        borderRadius: radius[1],
        backgroundColor: color.foreground,
        color: color.background,
        whiteSpace: "nowrap",
        pointerEvents: "none",
        fontVariantNumeric: "tabular-nums",
    },
    handle: {
        position: "absolute",
        insetBlock: space[1],
        width: space[2],
        cursor: "ew-resize",
        zIndex: 2,
    },
    barLabel: {
        position: "relative",
    },
    progressGrip: {
        position: "absolute",
        insetBlockEnd: 0,
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
    late: {
        outlineWidth: stroke.ring,
        outlineStyle: "solid",
        outlineColor: swatch.destructiveMark,
        outlineOffset: `calc(-1 * ${stroke.ring})`,
    },
    rollup: {
        position: "absolute",
        insetBlockEnd: space[1],
        height: stroke.ring,
        minWidth: stroke.ring,
        borderRadius: radius.full,
        pointerEvents: "none",
    },
    slack: {
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
        backgroundColor: swatch.destructiveMark,
        pointerEvents: "none",
    },
    inlineMilestone: {
        position: "absolute",
        top: `calc(50% - ${space[2]} / 2)`,
        width: space[2],
        height: space[2],
        marginInlineStart: `calc(${space[2]} / -2)`,
        transform: "rotate(45deg)",
        zIndex: 2,
    },
    adorn: {
        position: "absolute",
        insetBlock: 0,
        display: "flex",
        alignItems: "center",
        gap: space[1],
        marginInline: space[1],
        whiteSpace: "nowrap",
    },
    baseline: {
        position: "absolute",
        insetBlockEnd: stroke.border,
        height: stroke.ring,
        minWidth: stroke.ring,
        borderRadius: radius.full,
        backgroundColor: color.mutedForeground,
        opacity: 0.5,
        pointerEvents: "none",
    },
    progress: {
        position: "absolute",
        insetBlock: 0,
        insetInlineStart: 0,
        pointerEvents: "none",
    },
    milestone: {
        width: MILESTONE_SIZE,
        minWidth: MILESTONE_SIZE,
        height: MILESTONE_SIZE,
        insetBlock: "auto",
        top: `calc(50% - ${MILESTONE_SIZE} / 2)`,
        marginInlineStart: `calc(${MILESTONE_SIZE} / -2)`,
        padding: 0,
        borderRadius: radius[1],
        transform: "rotate(45deg)",
    },
    besideMilestone: {
        marginInline: `calc(${MILESTONE_SIZE} + ${String(LINK_BEND * 2)}px)`,
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
    linked: {
        outlineWidth: stroke.ring,
        outlineStyle: "solid",
        outlineColor: swatch.accentMark,
        outlineOffset: stroke.border,
    },
});

/** The properties of a bar on a timeline's track. */
export interface TimelineBarProperties extends Omit<
    JSX.ButtonHTMLAttributes<HTMLButtonElement>,
    "class"
> {
    /** The bar's color, its item's by default. */
    readonly color?: Hue;
    /** How the bar is filled, with its color's tint by default. */
    readonly variant?: TimelineBarVariant;
    /** What shows beside the bar's start, such as a status. */
    readonly before?: JSX.Element;
    /** What shows beside the bar's end, such as an assignee. */
    readonly after?: JSX.Element;
    /** The StyleX styles applied after the bar's. */
    readonly xstyle?: style.Styles;
}

/** Render a row's bar on the track, moved and resized when the timeline takes changes. */
export function TimelineBar(properties: TimelineBarProperties): JSX.Element {
    // draw the row's bars in their lanes, or a track to sweep an undated item's dates out on
    const control = useTimeline();
    const node = useTimelineRow().node;
    const bars = createMemo(() =>
        control.tree
            .lanes(node().item.key)
            .flatMap((lane, index) => lane.map((item) => ({ key: item.key, lane: index }))),
    );
    const rollup = () => control.tree.rollup(node().item.key);

    return (
        <Show
            when={bars().length > 0 || rollup().length > 0}
            fallback={<UndatedTrack xstyle={properties.xstyle} />}
        >
            <div role="gridcell" data-slot="timeline-track" {...style.attrs(styles.track)}>
                <For each={bars()} keyed={(bar) => bar.key}>
                    {(bar) => (
                        <Tooltip>
                            <BarTrack {...properties} item={bar().key} lane={bar().lane} />
                        </Tooltip>
                    )}
                </For>

                {/* The items under a folded item, small along its row's foot */}
                <For each={rollup()}>
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
                                places.fill(
                                    swatch[`${item.color ?? DEFAULT_COLOR}Mark`],
                                    color.background,
                                ),
                            )}
                        />
                    )}
                </For>
            </div>
        </Show>
    );
}

/** Render the track of an item without dates, on which a sweep sets them, after Linear. */
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

/** Render a bar's track: the bar, its labels, handles and connectors, and its hover card. */
function BarTrack(
    properties: TimelineBarProperties & {
        /** The key of the item the bar draws. */
        readonly item: string;
        /** The lane of the row the bar runs in, from the top. */
        readonly lane: number;
    },
): JSX.Element {
    // place the bar along the window in its lane, colored by its item
    const control = useTimeline();
    const locale = useLocale();
    const node = useTimelineRow().node;
    const key = untrack(() => properties.item);
    const item = () => control.drag.preview(key);
    const rest = omit(
        properties,
        "item",
        "lane",
        "color",
        "variant",
        "before",
        "after",
        "children",
        "xstyle",
        "style",
    );

    // draw a parent's bar as a neutral summary of its children, as Gantt charts draw summary tasks
    const isSummary = (): boolean =>
        node().item.key === key &&
        node().hasChildren &&
        properties.color === undefined &&
        item().color === undefined;
    const isMilestone = (): boolean => item().start !== undefined && item().start === item().end;
    const open = (): "start" | "end" | undefined =>
        item().start === undefined ? "start" : item().end === undefined ? "end" : undefined;
    const isMoving = (): boolean =>
        control.drag.manager.isDragging(key) && control.drag.manager.operation()?.kind !== "link";
    const from = () => control.window.share(control.tree.drawn(item()).start);
    const to = () => control.window.share(control.tree.drawn(item()).end);
    const hue = (): Hue => properties.color ?? item().color ?? DEFAULT_COLOR;
    const isMovable = untrack(() => control.options.onItemsChange !== undefined);
    const isLinkable = untrack(() => control.options.onLinkCreate !== undefined);

    // register the bar's parts, which drags lift and the label measures
    const elements = new Map<TimelineBarPart, HTMLElement>();
    const register = (part: TimelineBarPart) => (created: HTMLElement) =>
        elements.set(part, created);

    // write a clipped bar's or a milestone's label beside it
    const [isClipped, setClipped] = createSignal(false, { ownedWrite: true });
    const [body, setBody] = createSignal<HTMLElement | undefined>(undefined, { ownedWrite: true });
    control.observeClip(body, setClipped);
    const registerBody = (created: HTMLElement) => {
        elements.set("body", created);
        setBody(created);
    };
    const measuredLabel = untrack(() => item().label);
    createEffect(
        () => item().label,
        (label) => {
            // measure again when the label changes, the observer measuring the first one
            const element = untrack(body);
            if (element !== undefined && label !== measuredLabel) {
                setClipped(element.scrollWidth > element.clientWidth + 1);
            }
        },
    );
    const isBeside = () => (isClipped() || isMilestone()) && properties.children === undefined;

    // chip a bar past an edge of the window, the one nearest the window alone in its lane
    const outside = (): "start" | "end" | undefined => {
        // read the bars beside this one in its lane, which stand in start order
        const lane = control.tree.lanes(node().item.key)[properties.lane] ?? [];
        const index = lane.findIndex((each) => each.key === key);
        const later = lane[index + 1];
        const earlier = lane[index - 1];

        // take the side the bar lies past, where no neighbour lies past it too
        const isNearestBefore =
            later === undefined || control.window.share(control.tree.drawn(later).end) >= 0;
        const isNearestAfter =
            earlier === undefined || control.window.share(control.tree.drawn(earlier).start) <= 1;
        if (to() < 0 && isNearestBefore) {
            return "start";
        } else if (from() > 1 && isNearestAfter) {
            return "end";
        }
        // show the bar itself
        else {
            return undefined;
        }
    };

    // drag the bar, its edges and its links, and open it on a press
    const drags =
        isMovable || isLinkable
            ? control.drag.draggables(key, (part) => elements.get(part))
            : undefined;
    const press = new Press({
        onPress: (event) => control.press(key, event),
        keys: ["Enter"],
        shouldCancelOnMove: true,
    });

    // drag or step the done share at its grip, previewed until let go
    const [moving, setMoving] = createSignal<number>();
    const progress = () => moving() ?? item().progress;
    const commitProgress = (done: number) =>
        control.tree.commit([{ ...item(), progress: Math.round(done * 100) / 100 }]);

    // read the pointer's place along the bar as a done share, from 0 to 1
    const doneAt = (clientX: number, element: HTMLElement): number => {
        // mirror the pointer's place right to left
        const bounds = element.getBoundingClientRect();
        const along = (clientX - bounds.left) / bounds.width;
        const at = locale.direction === "rtl" ? 1 - along : along;

        return Math.min(1, Math.max(0, (at - from()) / (to() - from())));
    };

    // follow the pointer from the grip along the lane
    let lane: HTMLElement | undefined;
    const grip = new Move(
        {
            keys: [],
            onMove: (event) => {
                if (lane !== undefined) {
                    setMoving(doneAt(event.clientX, lane));
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

    // place the deadline, slack and milestones along the track
    const share = (at: number) => control.window.share(at);
    const isLate = (): boolean => TimelineItem.isLate(item());

    return (
        <div
            data-slot="timeline-lane"
            data-lane={properties.lane}
            ref={(created) => (lane = created)}
            {...style.attrs(
                styles.lane,
                places.row(properties.lane * control.rowHeight, control.rowHeight),
            )}
        >
            {/* The bar, or a milestone's diamond */}
            <TooltipTrigger
                render={(trigger) => (
                    <button
                        {...mergeProperties(
                            trigger,
                            mergeProperties(
                                mergeProperties(
                                    mergeProperties({ ref: registerBody }, press.attributes()),
                                    isMovable && drags !== undefined ? drags.body : {},
                                ),
                                rest,
                            ),
                        )}
                        tabindex={-1}
                        data-slot="timeline-bar"
                        data-variant={properties.variant ?? "tint"}
                        data-summary={isSummary() ? "" : undefined}
                        data-milestone={isMilestone() ? "" : undefined}
                        data-dragging={isMoving() ? "" : undefined}
                        data-linked={control.isLinked(key) ? "" : undefined}
                        data-late={isLate() ? "" : undefined}
                        {...style.attributes(
                            [
                                text.xs,
                                state.focus,
                                styles.bar,
                                isMilestone()
                                    ? [styles.milestone, places.at(from())]
                                    : places.span(from(), to() - from()),
                                properties.variant === "solid" || isMilestone()
                                    ? places.fill(swatch[`${hue()}Solid`], swatch[`${hue()}Label`])
                                    : places.fill(
                                          swatch[`${hue()}Tint`],
                                          swatch[`${hue()}TintText`],
                                      ),
                                isSummary() && styles.summary,
                                isMovable && styles.movable,
                                (isClipped() || isMilestone()) && styles.clipped,
                                isMoving() && styles.lifted,
                                control.isLinked(key) && styles.linked,
                                isLate() && styles.late,
                                open() !== undefined &&
                                    places.fade(
                                        (open() === "end") === (locale.direction === "ltr")
                                            ? "to right"
                                            : "to left",
                                    ),
                                properties.xstyle,
                            ],
                            properties.style,
                        )}
                    >
                        {/* How much is done, under the label */}
                        <Show when={isMilestone() ? undefined : progress()}>
                            {(done) => (
                                <span
                                    data-slot="timeline-bar-progress"
                                    aria-hidden="true"
                                    {...style.attrs(
                                        styles.progress,
                                        places.progress(done(), swatch[`${hue()}Mark`]),
                                    )}
                                />
                            )}
                        </Show>
                        <span {...style.attrs(styles.barLabel)}>
                            {properties.children ?? item().label}
                        </span>
                    </button>
                )}
            />

            {/* The planned dates beneath the bar */}
            <Show when={item().baseline}>
                {(planned) => (
                    <span
                        data-slot="timeline-bar-baseline"
                        aria-hidden="true"
                        {...style.attrs(
                            styles.baseline,
                            places.span(
                                control.window.share(planned().start),
                                control.window.share(planned().end) -
                                    control.window.share(planned().start),
                            ),
                        )}
                    />
                )}
            </Show>

            {/* The time the item can slip without moving the plan's end */}
            <Show when={item().slack}>
                {(slack) => (
                    <span
                        data-slot="timeline-bar-slack"
                        aria-hidden="true"
                        {...style.attrs(
                            styles.slack,
                            places.span(
                                to(),
                                share(control.tree.drawn(item()).end + slack()) - to(),
                            ),
                        )}
                    />
                )}
            </Show>

            {/* The time the item is due */}
            <Show when={item().deadline}>
                {(deadline) => (
                    <span
                        data-slot="timeline-bar-deadline"
                        aria-hidden="true"
                        {...style.attrs(styles.deadline, places.at(share(deadline())))}
                    />
                )}
            </Show>

            {/* The milestones along the item */}
            <For each={item().milestones ?? []}>
                {(milestone) => (
                    <span
                        data-slot="timeline-bar-milestone"
                        role="img"
                        aria-label={milestone.label}
                        title={milestone.label}
                        {...style.attrs(
                            styles.inlineMilestone,
                            places.fill(swatch[`${hue()}Solid`], swatch[`${hue()}Label`]),
                            places.at(share(milestone.at)),
                        )}
                    />
                )}
            </For>

            {/* What shows beside the bar's start and end */}
            <Show when={properties.before}>
                <span
                    data-slot="timeline-bar-before"
                    {...style.attrs(text.xs, styles.adorn, places.before(from()))}
                >
                    {properties.before}
                </span>
            </Show>
            <Show when={properties.after}>
                <span
                    data-slot="timeline-bar-after"
                    {...style.attrs(text.xs, styles.adorn, places.at(to()))}
                >
                    {properties.after}
                </span>
            </Show>

            {/* The label beside a bar too short for it, and beside a milestone */}
            <Show when={isBeside()}>
                <span
                    data-slot="timeline-bar-label"
                    aria-hidden="true"
                    {...style.attrs(
                        text.xs,
                        styles.beside,
                        isMilestone() && styles.besideMilestone,
                        to() > LABEL_FLIP ? places.before(from()) : places.at(to()),
                    )}
                >
                    {item().label}
                </span>
            </Show>

            {/* The hover card: the label, the dates and the duration, hidden while dragging */}
            <Show when={!isMoving()}>
                <TooltipContent side="top" data-slot="timeline-bar-card">
                    <span {...style.attrs(styles.cardLabel)}>{item().label}</span>
                    <span {...style.attrs(styles.cardDetail)}>
                        {control.window.rangeLabel(item())}
                        <Show when={!isMilestone() && open() === undefined}>
                            {" · "}
                            {control.window.durationLabel(
                                control.tree.drawn(item()).end - control.tree.drawn(item()).start,
                            )}
                        </Show>
                    </span>
                    <Show when={item().baseline}>
                        {(planned) => (
                            <span {...style.attrs(styles.cardDetail)}>
                                {locale.render(
                                    t`Planned ${control.window.rangeLabel({ ...item(), ...planned() })}`,
                                )}
                            </span>
                        )}
                    </Show>
                    <Show when={item().deadline}>
                        {(deadline) => (
                            <span {...style.attrs(styles.cardDetail)}>
                                {locale.render(
                                    t`Due ${control.window.rangeLabel({ ...item(), start: deadline(), end: deadline() })}`,
                                )}
                            </span>
                        )}
                    </Show>
                </TooltipContent>
            </Show>

            {/* A chip at the edge the bar lies past, which brings it into view */}
            <Show when={outside()}>
                {(side) => (
                    <button
                        type="button"
                        tabindex={-1}
                        data-slot="timeline-bar-outside"
                        data-side={side()}
                        aria-label={locale.render(t`Show ${item().label}`)}
                        onClick={() =>
                            control.window.reveal(
                                (control.tree.drawn(item()).start +
                                    control.tree.drawn(item()).end) /
                                    2,
                            )
                        }
                        {...style.attrs(
                            text.xs,
                            state.hover,
                            styles.outside,
                            side() === "start" ? styles.outsideStart : styles.outsideEnd,
                        )}
                    >
                        <Show when={side() === "start"}>
                            <Icon icon={locale.direction === "rtl" ? caretRight : caretLeft} />
                        </Show>
                        <span {...style.attrs(styles.outsideLabel)}>{item().label}</span>
                        <Show when={side() === "end"}>
                            <Icon icon={locale.direction === "rtl" ? caretLeft : caretRight} />
                        </Show>
                    </button>
                )}
            </Show>

            {/* The dates a moving bar lands on */}
            <Show when={isMoving()}>
                <span
                    data-slot="timeline-bar-dates"
                    aria-hidden="true"
                    {...style.attrs(text.xs, styles.dates, places.at(to()))}
                >
                    {control.window.rangeLabel(item())}
                </span>
            </Show>

            {/* The grip that sets how much is done, by a drag or the arrow keys */}
            <Show when={isMovable && !isMilestone()}>
                <span
                    role="slider"
                    tabindex={-1}
                    data-slot="timeline-bar-progress-grip"
                    aria-label={locale.render(t`How much of ${item().label} is done`)}
                    aria-valuemin={0}
                    aria-valuemax={100}
                    aria-valuenow={percentOf(progress())}
                    aria-valuetext={
                        progress() === undefined
                            ? undefined
                            : locale.number(progress() ?? 0, { style: "percent" })
                    }
                    {...mergeProperties(grip.attributes(), {
                        onKeyDown: (event: KeyboardEvent) => {
                            // step by a tenth along the text's direction, or go to either end
                            const done = progressAfter(
                                event.key,
                                progress() ?? 0,
                                locale.direction,
                            );
                            if (done !== undefined) {
                                event.preventDefault();
                                event.stopPropagation();
                                commitProgress(done);
                            }
                        },
                    })}
                    {...style.attrs(
                        styles.progressGrip,
                        places.at(from() + (to() - from()) * (progress() ?? 0)),
                    )}
                />
            </Show>

            {/* The handles that move one edge alone, none on a milestone */}
            <Show when={isMovable && !isMilestone() ? drags : undefined}>
                {(current) => (
                    <>
                        <span
                            tabindex={-1}
                            data-slot="timeline-bar-handle"
                            data-edge="start"
                            aria-label={locale.render(t`Change the start of ${item().label}`)}
                            {...mergeProperties({ ref: register("start") }, current().start)}
                            {...style.attrs(styles.handle, places.before(from()))}
                        />
                        <span
                            tabindex={-1}
                            data-slot="timeline-bar-handle"
                            data-edge="end"
                            aria-label={locale.render(t`Change the end of ${item().label}`)}
                            {...mergeProperties({ ref: register("end") }, current().end)}
                            {...style.attrs(styles.handle, places.at(to()))}
                        />
                    </>
                )}
            </Show>

            {/* The connectors a link is drawn from, beyond each edge */}
            <Show when={isLinkable ? drags : undefined}>
                {(current) => (
                    <>
                        <span
                            tabindex={-1}
                            data-slot="timeline-bar-connector"
                            data-edge="start"
                            aria-label={locale.render(t`Link from the start of ${item().label}`)}
                            {...mergeProperties(
                                { ref: register("link-start") },
                                current().linkStart,
                            )}
                            {...style.attrs(
                                styles.connector,
                                styles.connectorStart,
                                places.before(from()),
                            )}
                        />
                        <span
                            tabindex={-1}
                            data-slot="timeline-bar-connector"
                            data-edge="end"
                            aria-label={locale.render(t`Link from the end of ${item().label}`)}
                            {...mergeProperties({ ref: register("link-end") }, current().linkEnd)}
                            {...style.attrs(styles.connector, styles.connectorEnd, places.at(to()))}
                        />
                    </>
                )}
            </Show>
        </div>
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
