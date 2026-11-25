import { TimeScale } from "@destack/schema";
import * as style from "@destack/style";
import { color, radius, size, space, stroke, swatch } from "@destack/theme/tokens.stylex";
import { For, type JSX, omit, Show, useEnvironment, useLocale } from "@destack/view";
import { Move } from "../interaction/index.ts";
import { useTimeline, type TimelinePartProperties } from "./control.ts";
import { TimelineItem } from "./tree.ts";
import { places } from "./places.ts";

/** The styles of a timeline's minimap. */
const styles = style.create({
    minimap: {
        position: "relative",
        height: size[2],
        borderBlockStartWidth: stroke.border,
        borderBlockStartStyle: "solid",
        borderBlockStartColor: color.border,
        backgroundColor: color.muted,
        touchAction: "none",
    },
    trace: {
        position: "absolute",
        height: space[1],
        marginBlockStart: `calc(${space[1]} / -2)`,
        forcedColorAdjust: "none",
        borderRadius: radius.full,
    },
    lens: {
        position: "absolute",
        insetBlock: space[1],
        borderWidth: stroke.ring,
        borderStyle: "solid",
        borderColor: swatch.accentMark,
        borderRadius: radius[2],
        backgroundColor: color.selected,
        cursor: { default: "grab", ":active": "grabbing" },
    },
    lensGrip: {
        position: "absolute",
        insetBlock: 0,
        width: space[2],
        cursor: "ew-resize",
        "::after": {
            content: "''",
            position: "absolute",
            insetBlock: "25%",
            insetInlineStart: `calc(50% - ${stroke.ring} / 2)`,
            width: stroke.ring,
            borderRadius: radius.full,
            backgroundColor: swatch.accentMark,
        },
    },
    lensGripStart: {
        insetInlineStart: `calc(${space[2]} / -2)`,
    },
    lensGripEnd: {
        insetInlineEnd: `calc(${space[2]} / -2)`,
    },
});

/** The share of the span a window shows from which the minimap draws no lens. */
const WHOLE_SPAN = 0.999;

/** The least share of the span the minimap's grips leave the window. */
const MIN_LENS_SHARE = 0.02;

/** The minimap's grips, which resize the window rather than pan it. */
const MINIMAP_GRIP = "[data-slot=timeline-minimap-grip]";

/** Render every item over the whole span, and the window as a lens the pointer drags to pan. */
export function TimelineMinimap(properties: TimelinePartProperties): JSX.Element {
    // place each item as a line, and the window as a lens over the span
    const control = useTimeline();
    const locale = useLocale();
    const rest = omit(properties, "xstyle", "style");
    let element: HTMLElement | undefined;
    const span = () => control.tree.span();
    const dated = () => control.options.items.filter((item) => TimelineItem.isDated(item));
    const lens = () => ({
        left: TimeScale.at(span(), control.window.range().start),
        width:
            (control.window.range().end - control.window.range().start) /
            (span().end - span().start),
    });
    const { clock } = useEnvironment();

    // read the time a pointer's travel covers along the minimap, mirrored right to left
    const timeOf = (deltaX: number): number => {
        if (element === undefined) {
            throw new TypeError("timeline minimap moved before it mounted");
        }
        const travel = locale.direction === "rtl" ? -deltaX : deltaX;

        return (travel / element.getBoundingClientRect().width) * (span().end - span().start);
    };

    // pan by dragging the lens, a press on its grips resizing it instead
    const move = new Move(
        {
            distance: 0,
            orientation: "horizontal",
            canStart: (event) =>
                !(event.target instanceof Element && event.target.matches(MINIMAP_GRIP)),
            onMove: (event) => {
                const window = control.window.range();
                control.window.pan(timeOf(event.deltaX) / (window.end - window.start));
            },
        },
        clock,
    );

    // resize the window by a grip at either edge of the lens, a share of the span at the least
    const grip = (edge: "start" | "end") =>
        new Move(
            {
                distance: 0,
                orientation: "horizontal",
                onMove: (event) => {
                    // move the edge by the travel, the other edge a least share away
                    const window = control.window.range();
                    const least = (span().end - span().start) * MIN_LENS_SHARE;
                    const moved = window[edge] + timeOf(event.deltaX);
                    control.window.show(
                        edge === "start"
                            ? { ...window, start: Math.min(moved, window.end - least) }
                            : { ...window, end: Math.max(moved, window.start + least) },
                    );
                },
            },
            clock,
        ).attributes();

    return (
        <div
            data-slot="timeline-minimap"
            aria-hidden="true"
            {...rest}
            ref={(created) => (element = created)}
            {...style.attributes([styles.minimap, properties.xstyle], properties.style)}
        >
            <For each={dated()}>
                {(item, index) => (
                    <span
                        {...style.attrs(
                            styles.trace,
                            places.trace(
                                TimeScale.at(span(), control.tree.drawn(item).start),
                                (control.tree.drawn(item).end - control.tree.drawn(item).start) /
                                    (span().end - span().start),
                                (index() + 0.5) / dated().length,
                                control.tree.isSummary(item)
                                    ? color.mutedForeground
                                    : swatch[`${TimelineItem.hue(item)}Mark`],
                            ),
                        )}
                    />
                )}
            </For>

            {/* Lens, once the window shows less than every item */}
            <Show when={lens().width < WHOLE_SPAN}>
                <span
                    data-slot="timeline-minimap-lens"
                    {...move.attributes()}
                    {...style.attrs(styles.lens, places.span(lens().left, lens().width))}
                >
                    {/* The grips that resize the window */}
                    <span
                        data-slot="timeline-minimap-grip"
                        data-edge="start"
                        {...grip("start")}
                        {...style.attrs(styles.lensGrip, styles.lensGripStart)}
                    />
                    <span
                        data-slot="timeline-minimap-grip"
                        data-edge="end"
                        {...grip("end")}
                        {...style.attrs(styles.lensGrip, styles.lensGripEnd)}
                    />
                </span>
            </Show>
        </div>
    );
}
