import { type ScheduleEdge, ScheduleLink } from "@destack/schema";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { color, swatch } from "@destack/theme/tokens.stylex";
import { createMemo, createUniqueId, For, type JSX, Show, untrack, useLocale } from "@destack/view";
import { useTimeline } from "./control.ts";
import { TimelineItem } from "./tree.ts";
import { places } from "./places.ts";
import { LINK_BEND, MILESTONE_PIXELS } from "./bar.tsx";

/** The width of a dependency's line in pixels, between a hairline and a bar's edge. */
const LINK_WIDTH = 1.5;

/** The pixels of each dash and gap of a link being drawn, half a link's bend. */
const LINK_DASH = 4;

/** The width of a link's invisible hit line in pixels, a finger's tip around the drawn line. */
const LINK_HIT_WIDTH = 12;

/** The styles of a timeline's links. */
const styles = style.create({
    links: {
        position: "absolute",
        insetBlockStart: 0,
        pointerEvents: "none",
        overflow: "visible",
        zIndex: 0,
    },
    arrow: {
        fill: color.mutedForeground,
    },
    link: {
        fill: "none",
        stroke: color.mutedForeground,
        strokeWidth: LINK_WIDTH,
    },
    linkHighlighted: {
        stroke: swatch.accentMark,
        strokeWidth: (LINK_WIDTH * 4) / 3,
    },
    arrowHighlighted: {
        fill: swatch.accentMark,
    },
    linkHit: {
        fill: "none",
        stroke: "transparent",
        strokeWidth: LINK_HIT_WIDTH,
    },
    linkTarget: {
        pointerEvents: "stroke",
        cursor: "pointer",
        outlineStyle: "none",
    },
    linkDrawing: {
        stroke: swatch.accentMark,
        strokeDasharray: `${String(LINK_DASH)} ${String(LINK_DASH)}`,
    },
});

/** Render the dependencies between the visible rows as elbow arrows, each removable with Delete. */
export function TimelineLinkLayer(properties: {
    /** The StyleX styles applied after the links' styles. */
    readonly xstyle?: style.Styles;
}): JSX.Element {
    // place each link whose two items are visible rows, from its source edge to its target edge
    const control = useTimeline();
    const locale = useLocale();
    const arrow = createUniqueId();
    const indexOf = createMemo(
        () => new Map(control.tree.rows().map((row, index) => [row.item.key, index])),
    );
    const sign = () => (locale.direction === "rtl" ? -1 : 1);
    const pathOf = (
        from: { readonly key: string; readonly edge: ScheduleEdge },
        to: { readonly key: string; readonly edge: ScheduleEdge },
    ): string | undefined => {
        // find both bars' rows and lanes, leaving out items without dates and rows a fold hides
        const width = control.window.width();
        const isDrawn =
            TimelineItem.isDated(control.tree.item(from.key)) &&
            TimelineItem.isDated(control.tree.item(to.key));
        const [source, target] = isDrawn
            ? [control.tree.where(from.key), control.tree.where(to.key)]
            : [undefined, undefined];
        const fromIndex = source === undefined ? undefined : indexOf().get(source.row);
        const toIndex = target === undefined ? undefined : indexOf().get(target.row);
        if (
            source === undefined ||
            target === undefined ||
            fromIndex === undefined ||
            toIndex === undefined ||
            width === undefined
        ) {
            return undefined;
        }

        // read an edge's place along the track, mirrored right to left
        const xOf = (key: string, edge: ScheduleEdge) => {
            const drawn = control.tree.drawn(control.drag.preview(key));
            const x = control.window.share(edge === "start" ? drawn.start : drawn.end) * width;

            return sign() === 1 ? x : width - x;
        };

        // stop short of a milestone's diamond by half its diagonal
        const dropped = control.drag.preview(to.key);
        const isMilestone = dropped.start !== undefined && dropped.start === dropped.end;
        const short = isMilestone ? (MILESTONE_PIXELS / 2) * Math.SQRT2 : 0;
        const startX = xOf(from.key, from.edge);
        const endX = xOf(to.key, to.edge) + sign() * (to.edge === "start" ? -short : short);
        const targetTop = control.rowTop(toIndex) + target.lane * control.rowHeight;
        const startY = control.rowTop(fromIndex) + (source.lane + 0.5) * control.rowHeight;
        const endY = targetTop + control.rowHeight / 2;

        // run out of the source edge and into the target's edge from outside
        const out = startX + sign() * (from.edge === "end" ? LINK_BEND : -LINK_BEND);
        const into = endX + sign() * (to.edge === "start" ? -LINK_BEND : LINK_BEND);
        const isBehind = sign() * (to.edge === "start" ? out - into : into - out) > 0;

        // turn along the target lane's near edge when the way in lies behind the way out
        if (isBehind) {
            const edgeY = startY < endY ? targetTop : targetTop + control.rowHeight;

            return `M ${String(startX)} ${String(startY)} H ${String(out)} V ${String(edgeY)} H ${String(into)} V ${String(endY)} H ${String(endX)}`;
        }
        // run straight down at the way out, then along the target's lane
        else {
            return `M ${String(startX)} ${String(startY)} H ${String(out)} V ${String(endY)} H ${String(into)} H ${String(endX)}`;
        }
    };

    // draw each link whose items are drawn, keyed by its two items
    const links = createMemo(() =>
        (control.options.links ?? []).flatMap((link) => {
            const edges = ScheduleLink.edges(link.type);
            const path = pathOf(
                { key: link.from, edge: edges.from },
                { key: link.to, edge: edges.to },
            );

            return path === undefined ? [] : [{ link, path }];
        }),
    );

    // draw a link being drawn from its edge to the edge it aims at
    const drawing = () => {
        const operation = control.drag.manager.operation();
        const target = operation?.target;

        return operation?.kind === "link" &&
            operation.from.kind === "edge" &&
            target?.kind === "edge" &&
            target.key !== operation.from.key
            ? pathOf(operation.from, target)
            : undefined;
    };
    const isDeletable = untrack(() => control.options.onLinkDelete !== undefined);
    const isOpenable = untrack(() => control.options.onLinkAction !== undefined);
    const isFocusable = isDeletable || isOpenable;
    const shortcuts = [
        ...(isDeletable ? ["Delete", "Backspace"] : []),
        ...(isOpenable ? ["Enter"] : []),
    ];
    const highlighted = `${arrow}-highlighted`;
    const isHighlighted = (link: ScheduleLink): boolean => {
        const current = control.highlight();

        return current !== undefined && ScheduleLink.equals(current, link);
    };

    return (
        <svg
            data-slot="timeline-link-layer"
            height={control.rowTop(control.tree.rows().length)}
            {...style.attrs(styles.links, places.track(control.labelArea()), properties.xstyle)}
        >
            <defs>
                <marker
                    id={arrow}
                    viewBox="0 0 8 8"
                    refX="8"
                    refY="4"
                    markerWidth="6"
                    markerHeight="6"
                    orient="auto"
                >
                    <path d="M 0 0 L 8 4 L 0 8 z" {...style.attrs(styles.arrow)} />
                </marker>
                <marker
                    id={highlighted}
                    viewBox="0 0 8 8"
                    refX="8"
                    refY="4"
                    markerWidth="6"
                    markerHeight="6"
                    orient="auto"
                >
                    <path
                        d="M 0 0 L 8 4 L 0 8 z"
                        {...style.attrs(styles.arrow, styles.arrowHighlighted)}
                    />
                </marker>
            </defs>
            <For each={links()} keyed={(entry) => JSON.stringify([entry.link.from, entry.link.to])}>
                {(entry) => (
                    <g
                        data-slot="timeline-link"
                        role={isFocusable ? "button" : undefined}
                        aria-keyshortcuts={isFocusable ? shortcuts.join(" ") : undefined}
                        tabindex={isFocusable ? 0 : undefined}
                        aria-label={locale.render(
                            t`${control.tree.item(entry().link.from).label} to ${control.tree.item(entry().link.to).label}`,
                        )}
                        aria-hidden={isFocusable ? undefined : "true"}
                        data-highlighted={isHighlighted(entry().link) ? "" : undefined}
                        onKeyDown={(event) => {
                            // remove the link with Delete or Backspace, or open it with Enter
                            if (event.key === "Delete" || event.key === "Backspace") {
                                event.preventDefault();
                                control.options.onLinkDelete?.(entry().link);
                            } else if (event.key === "Enter") {
                                event.preventDefault();
                                control.options.onLinkAction?.(entry().link);
                            }
                        }}
                        onDblClick={() => control.options.onLinkAction?.(entry().link)}
                        onPointerEnter={() => control.setHighlight(entry().link)}
                        onPointerLeave={() => control.setHighlight(undefined)}
                        onFocus={() => control.setHighlight(entry().link)}
                        onBlur={() => control.setHighlight(undefined)}
                        {...style.attrs(isFocusable && styles.linkTarget)}
                    >
                        <path d={entry().path} {...style.attrs(styles.linkHit)} />
                        <path
                            d={entry().path}
                            marker-end={`url(#${isHighlighted(entry().link) ? highlighted : arrow})`}
                            {...style.attrs(
                                styles.link,
                                isHighlighted(entry().link) && styles.linkHighlighted,
                            )}
                        />
                    </g>
                )}
            </For>
            <Show when={drawing()}>
                {(path) => (
                    <path
                        data-slot="timeline-link-drawing"
                        d={path()}
                        marker-end={`url(#${arrow})`}
                        {...style.attrs(styles.link, styles.linkDrawing)}
                    />
                )}
            </Show>
        </svg>
    );
}
