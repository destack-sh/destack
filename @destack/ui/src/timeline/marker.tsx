import type { Hue } from "@destack/color";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, radius, space, stroke, swatch } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { For, type JSX, omit, Show } from "@destack/view";
import { useTimeline, type TimelinePartProperties } from "./control.ts";
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
        whiteSpace: "nowrap",
    },
    tint: (background: string, label: string) => ({ backgroundColor: background, color: label }),
});

/** Render a line down the track at each of the axis's marks, the non-working days shaded. */
export function TimelineGrid(properties: TimelinePartProperties): JSX.Element {
    // shade the days off and line the marks, on a layer over the rows
    const control = useTimeline();
    const share = (at: number) => control.window.share(at);
    const rest = omit(properties, "children", "xstyle", "style");

    return (
        <div
            data-slot="timeline-grid"
            aria-hidden="true"
            {...rest}
            {...style.attributes([styles.grid, properties.xstyle], properties.style)}
        >
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
        </div>
    );
}

/** The properties of a marker on a timeline. */
export interface TimelineMarkerProperties extends Omit<
    JSX.HTMLAttributes<HTMLSpanElement>,
    "class"
> {
    /** The time the marker stands at, in epoch milliseconds. */
    readonly at: number;
    /** The StyleX styles applied after the marker's. */
    readonly xstyle?: style.Styles;
}

/** Render a line down the track at a time, labelled by its children, while the window shows it. */
export function TimelineMarker(properties: TimelineMarkerProperties): JSX.Element {
    // place the marker by its time's share of the window
    const control = useTimeline();
    const share = () => control.window.share(properties.at);
    const rest = omit(properties, "at", "children", "xstyle", "style");

    return (
        <Show when={share() >= 0 && share() <= 1}>
            <span
                data-slot="timeline-marker"
                {...rest}
                {...style.attributes(
                    [styles.marker, places.mark(control.labelArea(), share()), properties.xstyle],
                    properties.style,
                )}
            >
                <Show when={properties.children}>
                    <span {...style.attrs(text.xs, styles.markerLabel)}>{properties.children}</span>
                </Show>
            </span>
        </Show>
    );
}

/** Render a marker at the current time, in the accent. */
export function TimelineNow(properties: Omit<TimelineMarkerProperties, "at">): JSX.Element {
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
    /** The start, in epoch milliseconds. */
    readonly start: number;
    /** The end, in epoch milliseconds, after the start. */
    readonly end: number;
    /** The swatch it is tinted in, the muted colors by default. */
    readonly color?: Hue;
    /** The StyleX styles applied after the range's. */
    readonly xstyle?: style.Styles;
}

/** Render a span of time shaded down the track, such as a cycle, labelled at its top. */
export function TimelineRange(properties: TimelineRangeProperties): JSX.Element {
    // cut the span to the window and tint it, showing it while any of it is in view
    const control = useTimeline();
    const share = (at: number) => control.window.share(at);
    const from = () => Math.max(0, share(properties.start));
    const to = () => Math.min(1, share(properties.end));
    const rest = omit(properties, "start", "end", "color", "children", "xstyle", "style");

    return (
        <Show when={to() > from()}>
            <span
                data-slot="timeline-range"
                data-color={properties.color}
                {...rest}
                {...style.attributes(
                    [
                        styles.range,
                        places.markSpan(control.labelArea(), from(), to() - from()),
                        properties.color === undefined
                            ? styles.tint(color.muted, color.mutedForeground)
                            : styles.tint(
                                  swatch[`${properties.color}Tint`],
                                  swatch[`${properties.color}TintText`],
                              ),
                        properties.xstyle,
                    ],
                    properties.style,
                )}
            >
                <Show when={properties.children}>
                    <span {...style.attrs(text.xs, styles.rangeLabel)}>{properties.children}</span>
                </Show>
            </span>
        </Show>
    );
}
