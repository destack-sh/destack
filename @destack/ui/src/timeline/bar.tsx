import caretLeft from "@destack/icon/phosphor/caret-left";
import caretRight from "@destack/icon/phosphor/caret-right";
import type { Hue } from "@destack/color";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { state } from "@destack/theme/state";
import { text } from "@destack/theme/text";
import { color, radius, shadow, space, stroke, swatch, weight } from "@destack/theme/tokens.stylex";
import {
    createEffect,
    createSignal,
    type JSX,
    omit,
    Show,
    untrack,
    useLocale,
} from "@destack/view";
import { CollectionBuilder } from "../collection/index.ts";
import { Icon } from "../icon/index.ts";
import { Press } from "../interaction/index.ts";
import { mergeProperties, renderPart } from "../part/index.ts";
import {
    Tooltip,
    TooltipContent,
    type TooltipContentProperties,
    TooltipTrigger,
} from "../tooltip/index.ts";
import {
    type TimelineBarControl,
    TimelineBarContext,
    type TimelineElementProperties,
    useTimeline,
    useTimelineBar,
    useTimelineRow,
} from "./control.ts";
import { TimelineBarHandle } from "./handle.tsx";
import { rowMarker } from "./marker.stylex.ts";
import { places } from "./places.ts";
import { type TimelineBarVariant, TimelineItem } from "./tree.ts";

/** The side of a milestone's diamond in pixels, between a dot and a bar's height. */
export const MILESTONE_PIXELS = 12;

/** The pixels a link runs out of its bar's end before it turns down to the next. */
export const LINK_BEND = 8;

/** The side of a milestone's diamond as a length. */
const MILESTONE_SIZE = `${String(MILESTONE_PIXELS)}px`;

/** The share of the track past which a short bar's label is written before it. */
const LABEL_FLIP = 0.75;

/** The styles of a bar and its parts. */
const styles = style.create({
    bar: {
        pointerEvents: "auto",
        position: "absolute",
        insetBlock: space[1],
        boxSizing: "border-box",
        minWidth: stroke.ring,
        borderRadius: radius[2],
        borderWidth: { default: null, [media.forcedColors]: "1px" },
        borderStyle: { default: null, [media.forcedColors]: "solid" },
        textAlign: "start",
        cursor: "default",
        zIndex: 1,
    },
    // draw the fill on its own, so what fades or turns leaves the parts of the bar alone
    hollow: {
        borderStyle: "none",
        backgroundColor: "transparent",
    },
    fill: {
        position: "absolute",
        inset: 0,
        borderRadius: "inherit",
        borderWidth: { default: null, [media.forcedColors]: "1px" },
        borderStyle: { default: null, [media.forcedColors]: "solid" },
    },
    summary: {
        // cover the grid, unlike the translucent borders
        backgroundColor: `color-mix(in oklab, ${color.foreground} 12%, ${color.background})`,
        color: color.foreground,
    },
    // run along the middle of the row, whose label says what it is
    line: {
        insetBlock: `calc(50% - ${space[1]})`,
        paddingInline: 0,
        borderRadius: radius.full,
        forcedColorAdjust: "none",
    },
    lineSummary: {
        backgroundColor: {
            default: `color-mix(in oklab, ${color.foreground} 28%, ${color.background})`,
            [media.forcedColors]: "CanvasText",
        },
    },
    movable: {
        cursor: { default: "grab", ":active": "grabbing" },
    },
    clipped: {
        color: "transparent",
    },
    label: {
        position: "absolute",
        inset: 0,
        paddingInline: space[2],
        display: "block",
        whiteSpace: "nowrap",
        overflow: "hidden",
        textOverflow: "ellipsis",
        alignContent: "center",
        pointerEvents: "none",
    },
    beside: {
        position: "absolute",
        insetBlock: 0,
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
    besideEnd: {
        insetInlineStart: "100%",
    },
    besideStart: {
        insetInlineEnd: "100%",
    },
    besideMilestone: {
        marginInline: `calc(${String(LINK_BEND * 2)}px)`,
    },
    progress: {
        borderInlineEndWidth: { default: null, [media.forcedColors]: "2px" },
        borderInlineEndStyle: { default: null, [media.forcedColors]: "solid" },
        position: "absolute",
        insetBlock: 0,
        insetInlineStart: 0,
        borderStartStartRadius: "inherit",
        borderEndStartRadius: "inherit",
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
    milestone: {
        width: MILESTONE_SIZE,
        minWidth: MILESTONE_SIZE,
        height: MILESTONE_SIZE,
        insetBlock: "auto",
        top: `calc(50% - ${MILESTONE_SIZE} / 2)`,
        marginInlineStart: `calc(${MILESTONE_SIZE} / -2)`,
    },
    diamond: {
        borderRadius: radius[1],
        transform: "rotate(45deg)",
    },
    lifted: {
        boxShadow: shadow.overlay,
        zIndex: 3,
    },
    linked: {
        outlineWidth: stroke.ring,
        outlineStyle: "solid",
        outlineColor: swatch.accentMark,
        outlineOffset: stroke.border,
    },
    late: {
        outlineWidth: stroke.ring,
        outlineStyle: "solid",
        outlineColor: swatch.destructiveMark,
        outlineOffset: `calc(-1 * ${stroke.ring})`,
    },
    addon: {
        position: "absolute",
        insetBlock: 0,
        display: "flex",
        alignItems: "center",
        gap: space[1],
        marginInline: space[1],
        whiteSpace: "nowrap",
    },
    addonEnd: {
        insetInlineStart: "100%",
    },
    addonStart: {
        insetInlineEnd: "100%",
    },
    dates: {
        position: "absolute",
        insetBlockStart: "50%",
        insetInlineStart: "100%",
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
    outside: {
        pointerEvents: "auto",
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
});

/** The properties of a bar on a timeline's track. */
export type TimelineBarProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> & {
    /** The bar's color, its item's by default. */
    readonly color?: Hue;
    /** How the bar is drawn, as its item says by default. */
    readonly variant?: TimelineBarVariant;
    /** The StyleX styles applied after the bar's. */
    readonly xstyle?: style.Styles;
};

/** Render the bar of a track's item, or a milestone's diamond. */
export function TimelineBar(properties: TimelineBarProperties): JSX.Element {
    // color the bar by its item, a parent's as a summary unless the caller colors it
    const control = useTimeline();
    const locale = useLocale();
    const { node } = useTimelineRow();
    const outer = useTimelineBar();
    const { key, item, from, to, isMilestone, isMoving, open } = outer;
    const cards = new CollectionBuilder<symbol>(() => undefined);
    const bar: TimelineBarControl = {
        ...outer,
        addCard: () => void cards.add(Symbol("card")),
        hue: () => properties.color ?? outer.hue(),
        variant: () => properties.variant ?? outer.variant(),
    };
    const isLine = (): boolean => bar.variant() === "line" && !isMilestone();
    const isSummary = (): boolean =>
        node().item.key === key && properties.color === undefined && control.tree.isSummary(item());
    const isMovable = untrack(() => control.options.onItemsChange !== undefined);
    const isLate = (): boolean => TimelineItem.isLate(item());
    const rest = omit(properties, "color", "variant", "children", "xstyle", "style");

    // fill the bar itself, or a layer of it that turns or fades
    const isHollow = (): boolean => isMilestone() || open() !== undefined;
    const fill = (): style.Styles => [
        isLine()
            ? places.fill(swatch[`${bar.hue()}Mark`], color.foreground)
            : bar.variant() === "solid" || isMilestone()
              ? places.fill(swatch[`${bar.hue()}Solid`], swatch[`${bar.hue()}Label`])
              : places.fill(swatch[`${bar.hue()}Tint`], swatch[`${bar.hue()}TintText`]),
        isSummary() && styles.summary,
        isLine() && isSummary() && styles.lineSummary,
    ];
    const fade = (): style.Styles =>
        open() !== undefined &&
        places.fade((open() === "end") === (locale.direction === "ltr") ? "to right" : "to left");

    // open the item on a press
    const press = new Press({
        onPress: (event) => control.press(key, event),
        keys: ["Enter"],
        shouldCancelOnMove: true,
    });

    return (
        <TimelineBarContext value={bar}>
            <Tooltip>
                <TooltipTrigger
                    render={(trigger) => (
                        <div
                            role="group"
                            aria-label={item().label}
                            {...mergeProperties(
                                trigger,
                                mergeProperties(
                                    mergeProperties(
                                        mergeProperties(
                                            { ref: bar.register("body") },
                                            press.attributes(),
                                        ),
                                        isMovable && bar.drags !== undefined ? bar.drags.body : {},
                                    ),
                                    rest,
                                ),
                            )}
                            tabindex={-1}
                            aria-describedby={
                                cards.items().length > 0 ? trigger["aria-describedby"] : undefined
                            }
                            data-slot="timeline-bar"
                            data-variant={bar.variant()}
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
                                    fill(),
                                    isHollow() && styles.hollow,
                                    isLine() && styles.line,
                                    isMovable && styles.movable,
                                    isMoving() && styles.lifted,
                                    control.isLinked(key) && styles.linked,
                                    isLate() && styles.late,
                                    properties.xstyle,
                                ],
                                properties.style,
                            )}
                        >
                            <Show when={isHollow()}>
                                <span
                                    aria-hidden="true"
                                    {...style.attrs(
                                        styles.fill,
                                        fill(),
                                        isMilestone() && styles.diamond,
                                        fade(),
                                    )}
                                />
                            </Show>
                            {"children" in properties ? properties.children : <BarParts />}
                        </div>
                    )}
                />
                <Outside />
            </Tooltip>
        </TimelineBarContext>
    );
}

/** Render the parts of a bar by default: its progress, label, card and handles. */
function BarParts(): JSX.Element {
    const { variant, isMilestone } = useTimelineBar();

    return (
        <>
            <TimelineBarProgress />
            <Show when={variant() !== "line" || isMilestone()}>
                <TimelineBarLabel />
            </Show>
            <TimelineBarCard />
            <TimelineBarHandle edge="progress" />
            <TimelineBarHandle edge="start" />
            <TimelineBarHandle edge="end" />
            <TimelineBarHandle edge="link-start" />
            <TimelineBarHandle edge="link-end" />
        </>
    );
}

/** Render the label of a bar over how much is done, or beside a bar too short for it. */
export function TimelineBarLabel(properties: TimelineElementProperties): JSX.Element {
    // measure whether the label fits
    const control = useTimeline();
    const { item, isMilestone, to } = useTimelineBar();
    const [isClipped, setClipped] = createSignal(false, { ownedWrite: true });
    const [element, setElement] = createSignal<HTMLElement | undefined>(undefined, {
        ownedWrite: true,
    });
    control.observeClip(element, setClipped);
    const measured = untrack(() => item().label);
    createEffect(
        () => item().label,
        (label) => {
            // measure again when the label changes, the observer measuring the first one
            const current = untrack(element);
            if (current !== undefined && label !== measured) {
                setClipped(current.scrollWidth > current.clientWidth + 1);
            }
        },
    );
    const isBeside = (): boolean => isClipped() || isMilestone();

    return (
        <>
            {renderPart(
                "span",
                "timeline-bar-words",
                properties,
                () => [styles.label, isBeside() && styles.clipped],
                {
                    ref: (created: HTMLElement) => void setElement(created),
                    get children() {
                        return item().label;
                    },
                },
            )}
            <Show when={isBeside()}>
                <span
                    data-slot="timeline-bar-label"
                    aria-hidden="true"
                    {...style.attrs(
                        text.xs,
                        styles.beside,
                        to() > LABEL_FLIP ? styles.besideStart : styles.besideEnd,
                        isMilestone() && styles.besideMilestone,
                    )}
                >
                    {properties.children ?? item().label}
                </span>
            </Show>
        </>
    );
}

/** Render how much of a bar is done, as a fill. */
export function TimelineBarProgress(properties: TimelineElementProperties): JSX.Element {
    const { isMilestone, progress, hue } = useTimelineBar();

    return (
        <Show when={isMilestone() ? undefined : progress()}>
            {(done) =>
                renderPart(
                    "span",
                    "timeline-bar-progress",
                    properties,
                    () => [styles.progress, places.progress(done(), swatch[`${hue()}Mark`])],
                    { "aria-hidden": "true" as const },
                )
            }
        </Show>
    );
}

/** Render the card a bar shows on hover, and the dates it lands on while it moves. */
export function TimelineBarCard(properties: TooltipContentProperties): JSX.Element {
    // show the dates it lands on while it moves
    const control = useTimeline();
    const { item, isMoving, addCard } = useTimelineBar();
    addCard();

    return (
        <Show
            when={!isMoving()}
            fallback={
                <span
                    data-slot="timeline-bar-dates"
                    aria-hidden="true"
                    {...style.attrs(text.xs, styles.dates)}
                >
                    {control.window.rangeLabel(item())}
                </span>
            }
        >
            <TooltipContent
                side="top"
                data-slot="timeline-bar-card"
                onPointerDown={(event: PointerEvent) => event.stopPropagation()}
                {...properties}
            >
                {"children" in properties ? properties.children : <BarDates />}
            </TooltipContent>
        </Show>
    );
}

/** Render the label, dates and duration of a bar's item, and when it was planned and is due. */
function BarDates(): JSX.Element {
    // write the dates at the precision of the axis, and how long they are
    const control = useTimeline();
    const locale = useLocale();
    const { item, isMilestone, open } = useTimelineBar();
    const length = (): string | undefined => {
        const { start, end } = control.tree.drawn(item());

        return isMilestone() || open() !== undefined
            ? undefined
            : control.window.durationLabel(end - start);
    };

    return (
        <>
            <span {...style.attrs(styles.cardLabel)}>{item().label}</span>
            <span {...style.attrs(styles.cardDetail)}>
                {control.window.rangeLabel(item())}
                <Show when={length()}>{(written) => <>{` · ${written()}`}</>}</Show>
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
        </>
    );
}

/** The properties of what stands beside a bar. */
export type TimelineBarAddonProperties = TimelineElementProperties & {
    /** The side of the bar it stands at, the end by default. */
    readonly align?: "start" | "end";
};

/** Render what stands beside a bar, such as a status before it or an assignee after it. */
export function TimelineBarAddon(properties: TimelineBarAddonProperties): JSX.Element {
    // stand past the bar's edge
    const rest = omit(properties, "align");
    const isStart = (): boolean => properties.align === "start";

    return renderPart(
        "span",
        "timeline-bar-addon",
        rest,
        () => [text.xs, styles.addon, isStart() ? styles.addonStart : styles.addonEnd],
        {
            get "data-align"() {
                return isStart() ? "start" : "end";
            },
        },
    );
}

/** Render a chip at the edge a bar lies past, which brings it into view. */
function Outside(): JSX.Element {
    // read the bars that overlap this one, which stand in start order
    const control = useTimeline();
    const locale = useLocale();
    const { node } = useTimelineRow();
    const { key, item, lane, from, to } = useTimelineBar();
    const outside = (): "start" | "end" | undefined => {
        // find the bars beside it
        const bars = control.tree.lanes(node().item.key)[lane()] ?? [];
        const index = bars.findIndex((each) => each.key === key);
        const later = bars[index + 1];
        const earlier = bars[index - 1];

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
    const reveal = (): void => {
        const drawn = control.tree.drawn(item());
        control.window.reveal((drawn.start + drawn.end) / 2);
    };

    return (
        <Show when={outside()}>
            {(side) => (
                <button
                    type="button"
                    tabindex={-1}
                    data-slot="timeline-bar-outside"
                    data-side={side()}
                    aria-label={locale.render(t`Show ${item().label}`)}
                    onClick={reveal}
                    {...style.attrs(
                        text.xs,
                        state.hover,
                        styles.outside,
                        side() === "start" ? styles.outsideStart : styles.outsideEnd,
                    )}
                >
                    <OutsideLabel side={side()} label={item().label} />
                </button>
            )}
        </Show>
    );
}

/** Render the label of a chip, with the arrow to the side its bar lies past. */
function OutsideLabel(properties: {
    /** The side the bar lies past. */
    readonly side: "start" | "end";
    /** The label of the bar's item. */
    readonly label: string;
}): JSX.Element {
    // point along the reading direction
    const locale = useLocale();
    const isRightToLeft = (): boolean => locale.direction === "rtl";

    return (
        <>
            <Show when={properties.side === "start"}>
                <Icon icon={isRightToLeft() ? caretRight : caretLeft} />
            </Show>
            <span {...style.attrs(styles.outsideLabel)}>{properties.label}</span>
            <Show when={properties.side === "end"}>
                <Icon icon={isRightToLeft() ? caretLeft : caretRight} />
            </Show>
        </>
    );
}
