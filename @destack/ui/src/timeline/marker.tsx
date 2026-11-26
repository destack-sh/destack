import type { Hue } from "@destack/color";
import { t } from "@destack/locale";
import type { TimeScale } from "@destack/schema";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, radius, space, stroke, swatch } from "@destack/theme/tokens.stylex";
import { state } from "@destack/theme/state";
import { text } from "@destack/theme/text";
import {
    For,
    type JSX,
    merge,
    omit,
    Show,
    untrack,
    useContext,
    useEnvironment,
    useLocale,
} from "@destack/view";
import { Move } from "../interaction/index.ts";
import { mergeProperties, renderPart } from "../part/index.ts";
import {
    leave,
    TimelineTrackContext,
    useTimeline,
    type TimelinePartProperties,
} from "./control.ts";
import { places } from "./places.ts";

/** The styles of a timeline's grid and markers. */
const styles = style.create({
    line: {
        position: "absolute",
        insetBlock: 0,
        width: stroke.border,
        backgroundColor: color.border,
        opacity: 0.5,
        pointerEvents: "none",
    },
    marker: {
        forcedColorAdjust: "none",
        position: "absolute",
        insetBlock: 0,
        width: stroke.border,
        backgroundColor: { default: color.foreground, [media.forcedColors]: "CanvasText" },
        color: color.background,
        pointerEvents: "none",
    },
    now: {
        width: stroke.ring,
        backgroundColor: { default: swatch.accentSolid, [media.forcedColors]: "Highlight" },
        color: swatch.accentLabel,
    },
    markerLabel: {
        position: "absolute",
        // write the label over the bars, which cover the line
        zIndex: 2,
        insetBlockStart: 0,
        insetInlineStart: "100%",
        paddingInline: space[1],
        borderEndEndRadius: radius[1],
        backgroundColor: "inherit",
        whiteSpace: "nowrap",
    },
    grid: {
        position: "absolute",
        inset: 0,
        pointerEvents: "none",
    },
    shade: {
        position: "absolute",
        insetBlock: 0,
        backgroundColor: color.muted,
        pointerEvents: "none",
    },
    range: {
        borderInlineWidth: { default: null, [media.forcedColors]: "1px" },
        borderInlineStyle: { default: null, [media.forcedColors]: "dashed" },
        position: "absolute",
        insetBlock: 0,
        opacity: 0.6,
        pointerEvents: "none",
    },
    rangeLabel: {
        position: "absolute",
        insetBlockStart: 0,
        insetInlineStart: 0,
        maxWidth: "100%",
        overflow: "hidden",
        paddingInline: space[1],
        textOverflow: "ellipsis",
        unicodeBidi: "plaintext",
        whiteSpace: "nowrap",
    },
    tint: (background: string, label: string) => ({ backgroundColor: background, color: label }),
});

/** Render a line down the track at each of the axis's marks, the non-working days shaded. */
export function TimelineGrid(properties: TimelinePartProperties): JSX.Element {
    // shade the days off and line the marks, on a layer over the rows
    const control = useTimeline();
    const share = (at: number) => control.window.share(at);
    const rest = omit(properties, "children");

    return renderPart("div", "timeline-grid", rest, styles.grid, {
        "aria-hidden": "true" as const,
        get children() {
            return (
                <>
                    <For each={control.window.nonWorking()} keyed={(span) => span.start}>
                        {(span) => (
                            <span
                                data-slot="timeline-non-working"
                                aria-hidden="true"
                                {...style.attrs(
                                    styles.shade,
                                    places.markSpan(
                                        control.labelArea(),
                                        share(span().start),
                                        share(span().end) - share(span().start),
                                    ),
                                )}
                            />
                        )}
                    </For>
                    <For each={control.window.ticks()} keyed={(tick) => tick.at}>
                        {(tick) => (
                            <span
                                data-slot="timeline-grid-line"
                                aria-hidden="true"
                                {...style.attrs(
                                    styles.line,
                                    places.mark(control.labelArea(), share(tick().at)),
                                )}
                            />
                        )}
                    </For>
                </>
            );
        },
    });
}

/** How far past its line a marker or an edge takes the pointer, more for a finger. */
const REACH = {
    default: `calc(-1 * ${space[1]})`,
    [media.pointerCoarse]: `calc(-1 * ${space[4]})`,
};

/** The styles of what a person drags along the track. */
const drags = style.create({
    movable: {
        pointerEvents: "auto",
        cursor: "ew-resize",
        touchAction: "none",
        zIndex: 3,
        "::before": { content: "''", position: "absolute", insetBlock: 0, insetInline: REACH },
    },
    // stand whole inside the window at both of its ends
    within: (forward: string, backward: string) => ({
        translate: { default: forward, ":dir(rtl)": backward },
    }),
    body: {
        pointerEvents: "auto",
        cursor: { default: "grab", ":active": "grabbing" },
        touchAction: "none",
    },
    edge: {
        position: "absolute",
        insetBlock: 0,
        width: stroke.ring,
        pointerEvents: "auto",
        cursor: "ew-resize",
        touchAction: "none",
        backgroundColor: "currentColor",
        "::before": { content: "''", position: "absolute", insetBlock: 0, insetInline: REACH },
    },
    start: { insetInlineStart: 0 },
    end: { insetInlineEnd: 0 },
});

/** A slide of places along the track: what moves them, and whether they move now. */
interface Slide {
    /** The attributes of the element that moves them. */
    readonly attributes: ReturnType<Move["attributes"]> & {
        readonly onPointerDown: (event: PointerEvent) => void;
        readonly onKeyDown: (event: KeyboardEvent) => void;
    };
    /** Whether the pointer moves them. */
    readonly isMoving: () => boolean;
}

/** Move places along the track by the pointer or the keys, from where they stood. */
function useSlide(
    stands: () => readonly number[],
    onSlide: (moved: readonly number[]) => void,
): Slide {
    // follow the pointer by its share of the track, and step by the keys
    const control = useTimeline();
    const locale = useLocale();
    let from: readonly number[] = [];
    const mirror = (): 1 | -1 => (locale.direction === "rtl" ? -1 : 1);
    const slideTo = (current: readonly number[], at: number): void => {
        // move the first place there, and the others along with it
        const [first = 0] = current;
        onSlide(current.map((place) => place + at - first));
    };
    const move = new Move(
        {
            distance: 0,
            orientation: "horizontal",
            onMoveStart: () => {
                from = untrack(stands);
            },
            onMove: (event) => {
                // step the first place by a key
                const current = untrack(stands);
                if (event.pointerType === "keyboard") {
                    const direction = event.deltaX * mirror() > 0 ? 1 : -1;
                    slideTo(current, control.window.step(current[0] ?? 0, direction, "start"));
                }
                // move from where they stood, landing the first where things land
                else {
                    const travel = (event.offsetX * mirror()) / control.window.measured();
                    const share = control.window.share(from[0] ?? 0);
                    const moved = control.window.instant(share + travel);
                    slideTo(from, control.window.snap(moved, "start"));
                }
            },
        },
        useEnvironment().clock,
    );
    const attributes = move.attributes();

    return {
        isMoving: () => move.isMoving(),
        attributes: merge(attributes, {
            onPointerDown: (event: PointerEvent) => {
                // keep the press from the track under it
                leave(event);
                attributes.onPointerDown(event);
            },
            onKeyDown: (event: KeyboardEvent) => {
                // go to either end of the window, leaving other keys to the move
                const { start, end } = untrack(() => control.window.range());
                if (event.target === event.currentTarget && event.key === "Home") {
                    event.preventDefault();
                    slideTo(untrack(stands), start);
                } else if (event.target === event.currentTarget && event.key === "End") {
                    event.preventDefault();
                    slideTo(untrack(stands), end);
                } else {
                    attributes.onKeyDown(event);
                }
            },
        }),
    };
}

/** The properties of a marker on a timeline. */
export interface TimelineMarkerProperties extends Omit<
    JSX.HTMLAttributes<HTMLSpanElement>,
    "class"
> {
    /** Where the marker stands. */
    readonly at: number;
    /** Handle the person moving it, which makes it movable, such as a playhead. */
    readonly onAtChange?: (at: number) => void;
    /** The StyleX styles applied after the marker's. */
    readonly xstyle?: style.Styles;
}

/** Render a line at a time, down every row or on the track it is in, while the window shows it. */
export function TimelineMarker(properties: TimelineMarkerProperties): JSX.Element {
    // place the marker by its share of the window
    const control = useTimeline();
    const isInTrack = useContext(TimelineTrackContext);
    const share = () => control.window.share(properties.at);
    const isShown = (): boolean => share() >= 0 && share() <= 1;
    const rest = omit(properties, "at", "onAtChange", "children");

    // slide it where the caller takes its place
    const slide = untrack(() => properties.onAtChange !== undefined)
        ? useSlide(
              () => [properties.at],
              ([at = 0]) => properties.onAtChange?.(at),
          )
        : undefined;

    return (
        <Show when={isShown() || slide?.isMoving() === true}>
            {renderPart(
                "span",
                "timeline-marker",
                rest,
                () => [
                    styles.marker,
                    slide !== undefined && [
                        state.focus,
                        drags.movable,
                        drags.within(`${String(-share() * 100)}%`, `${String(share() * 100)}%`),
                    ],
                    isInTrack ? places.at(share()) : places.mark(control.labelArea(), share()),
                ],
                mergeProperties(
                    slide === undefined
                        ? {}
                        : mergeProperties(slide.attributes, {
                              role: "slider" as const,
                              tabindex: 0,
                              get "aria-valuemin"() {
                                  return control.window.range().start;
                              },
                              get "aria-valuemax"() {
                                  return control.window.range().end;
                              },
                              get "aria-valuenow"() {
                                  return properties.at;
                              },
                              get "aria-valuetext"() {
                                  return control.window.instantLabel(properties.at);
                              },
                          }),
                    {
                        get children() {
                            return (
                                <Show when={properties.children}>
                                    <span {...style.attrs(text.xs, styles.markerLabel)}>
                                        {properties.children}
                                    </span>
                                </Show>
                            );
                        },
                    },
                ),
            )}
        </Show>
    );
}

/** Render a marker at the current time, in the accent. */
export function TimelineNow(
    properties: Omit<TimelineMarkerProperties, "at" | "onAtChange">,
): JSX.Element {
    const control = useTimeline();
    const rest = omit(properties, "xstyle");

    return (
        <Show when={control.window.now()}>
            {(now) => (
                <TimelineMarker
                    data-slot="timeline-now"
                    at={now()}
                    {...rest}
                    xstyle={[styles.now, properties.xstyle]}
                />
            )}
        </Show>
    );
}

/** The properties of a range on a timeline. */
export interface TimelineRangeProperties extends Omit<
    JSX.HTMLAttributes<HTMLSpanElement>,
    "class"
> {
    /** Where the range starts. */
    readonly start: number;
    /** Where the range ends, after its start. */
    readonly end: number;
    /** Handle the person moving it or an edge, which makes it movable, such as a loop. */
    readonly onRangeChange?: (range: TimeScale) => void;
    /** The swatch it is tinted in, the muted colors by default. */
    readonly color?: Hue;
    /** The StyleX styles applied after the range's. */
    readonly xstyle?: style.Styles;
}

/** Render a span of time, down every row or along the track it is in, such as a cycle. */
export function TimelineRange(properties: TimelineRangeProperties): JSX.Element {
    // cut the span to the window and tint it, showing it while any of it is in view
    const control = useTimeline();
    const isInTrack = useContext(TimelineTrackContext);
    const share = (at: number) => control.window.share(at);
    const from = () => Math.max(0, share(properties.start));
    const to = () => Math.min(1, share(properties.end));
    const isShown = (at: number): boolean => share(at) >= 0 && share(at) <= 1;
    const rest = omit(properties, "start", "end", "onRangeChange", "color", "children");

    // move it whole where the caller takes its range
    const isMovable = untrack(() => properties.onRangeChange !== undefined);
    const slide = isMovable
        ? useSlide(
              () => [properties.start, properties.end],
              ([start = 0, end = 0]) => properties.onRangeChange?.({ start, end }),
          )
        : undefined;

    return (
        <Show when={to() > from() || slide?.isMoving() === true}>
            {renderPart(
                "span",
                "timeline-range",
                rest,
                () => [
                    styles.range,
                    isMovable && drags.body,
                    isInTrack
                        ? places.span(from(), to() - from())
                        : places.markSpan(control.labelArea(), from(), to() - from()),
                    properties.color === undefined
                        ? styles.tint(color.muted, color.mutedForeground)
                        : styles.tint(
                              swatch[`${properties.color}Tint`],
                              swatch[`${properties.color}TintText`],
                          ),
                ],
                mergeProperties(slide === undefined ? {} : slide.attributes, {
                    get "data-color"() {
                        return properties.color;
                    },
                    get children() {
                        return (
                            <>
                                <Show when={properties.children}>
                                    <span {...style.attrs(text.xs, styles.rangeLabel)}>
                                        {properties.children}
                                    </span>
                                </Show>
                                <Show when={isMovable && isShown(properties.start)}>
                                    <RangeEdge
                                        edge="start"
                                        at={properties.start}
                                        onSlide={(start) =>
                                            properties.onRangeChange?.({
                                                start: Math.min(start, properties.end - 1),
                                                end: properties.end,
                                            })
                                        }
                                    />
                                </Show>
                                <Show when={isMovable && isShown(properties.end)}>
                                    <RangeEdge
                                        edge="end"
                                        at={properties.end}
                                        onSlide={(end) =>
                                            properties.onRangeChange?.({
                                                start: properties.start,
                                                end: Math.max(end, properties.start + 1),
                                            })
                                        }
                                    />
                                </Show>
                            </>
                        );
                    },
                }),
            )}
        </Show>
    );
}

/** Render an edge of a range, which moves alone. */
function RangeEdge(properties: {
    /** The edge. */
    readonly edge: "start" | "end";
    /** Where it stands. */
    readonly at: number;
    /** Handle the person moving it. */
    readonly onSlide: (at: number) => void;
}): JSX.Element {
    // slide the edge, keeping its presses from the range
    const control = useTimeline();
    const locale = useLocale();
    const slide = useSlide(
        () => [properties.at],
        ([at = 0]) => properties.onSlide(at),
    );

    return (
        <span
            role="slider"
            tabindex={0}
            data-slot="timeline-range-edge"
            data-edge={properties.edge}
            aria-label={locale.render(properties.edge === "start" ? t`Start` : t`End`)}
            aria-valuemin={control.window.range().start}
            aria-valuemax={control.window.range().end}
            aria-valuenow={properties.at}
            aria-valuetext={control.window.instantLabel(properties.at)}
            {...slide.attributes}
            {...style.attrs(
                state.focus,
                drags.edge,
                properties.edge === "start" ? drags.start : drags.end,
            )}
        />
    );
}
