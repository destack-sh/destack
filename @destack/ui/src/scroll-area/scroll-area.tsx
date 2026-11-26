import { field } from "@destack/object";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { state } from "@destack/theme/state";
import { color, motion, radius, space } from "@destack/theme/tokens.stylex";
import {
    createContext,
    createEffect,
    createSignal,
    type JSX,
    omit,
    untrack,
    useContext,
    useEnvironment,
} from "@destack/view";
import { Timeout } from "@destack/view/primitives/timer";
import { Move } from "../interaction/index.ts";
import {
    contentOr,
    type ElementPartProperties,
    mergeProperties,
    renderPart,
    type Render,
} from "../part/index.ts";
import { type Axis, Scroll, scrollFade, AXES, ORIENTATION } from "../position/index.ts";
import { areaMarker } from "./scroll-area.stylex.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The least length of a thumb in pixels, which a pointer can aim at. */
const LEAST_THUMB = 24;

/** The milliseconds a bar stays after scrolling, as macOS keeps its own. */
const LINGER = 800;

/** The styles of a scroll area. */
const styles = style.create({
    area: {
        position: "relative",
        display: "flex",
        flexDirection: "column",
        minWidth: 0,
        minHeight: 0,
        overflow: "hidden",
    },
    viewport: {
        flexGrow: 1,
        minWidth: 0,
        minHeight: 0,
        borderRadius: "inherit",
        overscrollBehavior: "contain",
        // leave the bars to the parts, and to the device on touch
        scrollbarWidth: { default: "none", [media.pointerCoarse]: "auto" },
    },
    // lie over the content, in view while a person scrolls or points at it
    bar: {
        position: "absolute",
        zIndex: 1,
        display: { default: "block", [media.pointerCoarse]: "none" },
        touchAction: "none",
        opacity: {
            default: 0,
            ":hover": { default: null, [media.hover]: 1 },
            ":is([data-scrolling])": 1,
            ":is([data-moving])": 1,
            [style.when.ancestor(":hover", areaMarker)]: { default: null, [media.hover]: 1 },
            [media.forcedColors]: 1,
        },
        transitionProperty: "opacity",
        transitionDuration: motion.durationMedium,
        transitionTimingFunction: motion.easingStandard,
    },
    thumb: {
        position: "absolute",
        borderRadius: radius.full,
        backgroundColor: {
            default: `color-mix(in oklab, ${color.foreground} 25%, transparent)`,
            ":hover": {
                default: null,
                [media.hover]: `color-mix(in oklab, ${color.foreground} 40%, transparent)`,
            },
            ":active": `color-mix(in oklab, ${color.foreground} 50%, transparent)`,
            [media.forcedColors]: "CanvasText",
        },
        transitionProperty: "background-color",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
    },
});

/** The axes a scroll area scrolls along in each orientation. */
const orientations = style.create({
    vertical: { overflowX: "hidden", overflowY: "auto" },
    horizontal: { overflowX: "auto", overflowY: "hidden" },
    both: { overflow: "auto" },
});

/** The place of the bar along each axis. */
const bars = style.create({
    vertical: { insetBlock: 0, insetInlineEnd: 0, width: space[3] },
    horizontal: { insetInline: 0, insetBlockEnd: 0, height: space[3] },
});

/** The place of the thumb in the bar along each axis. */
const thumbs = style.create({
    vertical: { insetInline: `calc(${space[1]} * 0.75)` },
    horizontal: { insetBlock: `calc(${space[1]} * 0.75)` },
});

/** A scroll area as its bars read it. */
interface ScrollAreaControl {
    /** Read the element that scrolls. */
    readonly viewport: () => HTMLElement | undefined;
    /** Read whether a person scrolled it just now. */
    readonly isScrolling: () => boolean;
}

/** The scroll area around a bar, null outside one. */
const ScrollAreaContext = createContext<ScrollAreaControl | null>(null);

/** A bar as its thumb reads it. */
interface BarControl {
    /** The axis it scrolls along. */
    readonly axis: Axis;
    /** Take the thumb's element. */
    readonly setThumb: (element: HTMLElement) => void;
}

/** The bar around a thumb, null outside one. */
const BarContext = createContext<BarControl | null>(null);

/** The properties people change in a scroll area's controls and inspector. */
const SCROLL_AREA_PROPERTIES = {
    orientation: field
        .enum({ ...AXES, both: { title: t.context("option")`Both` } })
        .labeled(t.context("label")`Orientation`)
        .described(t`The directions it scrolls in.`)
        .default("vertical"),
    fade: field
        .boolean()
        .labeled(t.context("label")`Fade`)
        .described(t`Fades the edges where more content waits to scroll in.`)
        .default(false),
};

/** The properties of a scroll area besides its declared ones: its bars and attributes. */
export interface ScrollAreaAttributes extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> {
    /** The bars, one per axis by default and none for null. */
    readonly bars?: JSX.Element;
    /** The StyleX styles applied after the scroll area's styles, usually its size. */
    readonly xstyle?: style.Styles;
    /** Render another element with its attributes, its own by default. */
    readonly render?: Render;
}

/** Render a region that scrolls natively, its bars over its content. */
export const ScrollArea = defineComponent({
    name: "scrollArea",
    properties: SCROLL_AREA_PROPERTIES,
    render(
        properties: RenderProperties<ScrollAreaAttributes, typeof SCROLL_AREA_PROPERTIES>,
    ): JSX.Element {
        // follow the scrolling, and keep the bars a moment after
        const [viewport, setViewport] = createSignal<HTMLElement | undefined>(undefined, {
            ownedWrite: true,
        });
        const [isScrolling, setScrolling] = createSignal(false, { ownedWrite: true });
        const linger = new Timeout();
        const scroll = (): void => {
            setScrolling(true);
            linger.set(() => setScrolling(false), LINGER);
        };
        createEffect(viewport, (element) =>
            element === undefined ? undefined : Scroll.follow(element),
        );
        const control: ScrollAreaControl = { viewport, isScrolling };
        const rest = omit(properties, "orientation", "fade", "bars", "xstyle", "style", "children");

        return (
            <ScrollAreaContext value={control}>
                <div
                    data-slot="scroll-area"
                    data-orientation={properties.orientation}
                    {...style.attributes(
                        [areaMarker, styles.area, properties.xstyle],
                        properties.style,
                    )}
                >
                    {renderPart(
                        "div",
                        "scroll-area-viewport",
                        rest,
                        () => [
                            state.focusInset,
                            styles.viewport,
                            orientations[properties.orientation],
                            properties.fade && scrollFade.sides,
                        ],
                        {
                            tabindex: 0,
                            ref: (element) => void setViewport(element),
                            onScroll: scroll,
                            get children() {
                                return properties.children;
                            },
                        },
                    )}
                    {"bars" in properties ? (
                        properties.bars
                    ) : (
                        <StandardBars orientation={properties.orientation} />
                    )}
                </div>
            </ScrollAreaContext>
        );
    },
});

/** The properties of a scroll area, its native attributes included. */
export type ScrollAreaProperties = ComponentProperties<typeof ScrollArea>;

/** Render a bar for each axis an area scrolls along. */
function StandardBars(properties: { readonly orientation: Axis | "both" }): JSX.Element {
    const axes: readonly Axis[] = untrack(() =>
        properties.orientation === "both" ? ["vertical", "horizontal"] : [properties.orientation],
    );

    return (
        <>
            {axes.map((axis) => (
                <ScrollAreaBar orientation={axis} />
            ))}
        </>
    );
}

/** The properties people change in a scroll bar's controls and inspector. */
const SCROLL_AREA_BAR_PROPERTIES = {
    orientation: ORIENTATION.described(
        t`Whether the bar runs down the side or across the bottom.`,
    ).default("vertical"),
};

/** The properties of a scroll bar besides its declared ones: its attributes. */
export type ScrollAreaBarAttributes = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "style"> &
    ElementPartProperties & {};

/** Render a bar that scrolls its area along an axis, hidden where everything fits. */
export const ScrollAreaBar = defineComponent({
    name: "scrollAreaBar",
    properties: SCROLL_AREA_BAR_PROPERTIES,
    render(
        properties: RenderProperties<ScrollAreaBarAttributes, typeof SCROLL_AREA_BAR_PROPERTIES>,
    ): JSX.Element {
        // stand by the area around it
        const area = useContext(ScrollAreaContext);
        if (area === null) {
            throw new TypeError("a scroll area's bar needs a scroll area around it");
        }
        const axis = untrack(() => properties.orientation);
        const isVertical = axis === "vertical";
        let bar: HTMLElement | undefined;
        let thumbElement: HTMLElement | undefined;
        let thumbLength = 0;
        let start = 0;
        const draw = (): void => {
            // size and place the thumb by the share in view
            const view = area.viewport();
            if (view === undefined || bar === undefined) {
                return;
            }
            const { length, whole, most, offset } = measure(view, axis);
            bar.hidden = most <= 1;
            const track = isVertical ? bar.clientHeight : bar.clientWidth;
            const thumb = Math.max(LEAST_THUMB, (length / whole) * track);
            const place = most === 0 ? 0 : (offset / most) * (track - thumb);
            thumbLength = Math.min(thumb, track);
            if (thumbElement !== undefined) {
                thumbElement.style[isVertical ? "height" : "width"] = `${String(thumbLength)}px`;
                thumbElement.style[isVertical ? "top" : "insetInlineStart"] = `${String(place)}px`;
            }
        };
        const scrollTo = (view: HTMLElement, offset: number): void => {
            // scroll to a distance from the start of the text
            if (isVertical) {
                view.scrollTop = offset;
            } else {
                view.scrollLeft = getComputedStyle(view).direction === "rtl" ? -offset : offset;
            }
        };
        const trackLength = (): number => (isVertical ? bar?.clientHeight : bar?.clientWidth) ?? 0;

        // follow the scrolling, and the size of the area and of what it scrolls
        createEffect(area.viewport, (view) => {
            if (view === undefined) {
                return undefined;
            }
            untrack(draw);
            const resizes = new ResizeObserver(() => untrack(draw));
            [view, ...view.children].forEach((each) => resizes.observe(each));
            const changes = new MutationObserver(() => {
                [...view.children].forEach((each) => resizes.observe(each));
                untrack(draw);
            });
            changes.observe(view, { childList: true });
            const listening = new AbortController();
            view.addEventListener("scroll", () => untrack(draw), {
                passive: true,
                signal: listening.signal,
            });

            return () => {
                resizes.disconnect();
                changes.disconnect();
                listening.abort();
            };
        });

        // go to where a press beside the thumb lands
        const press = (event: PointerEvent): void => {
            // leave the thumb, and other buttons
            const view = untrack(area.viewport);
            const bounds = bar?.getBoundingClientRect();
            const isOnThumb =
                event.target instanceof Node && thumbElement?.contains(event.target) === true;
            const room = trackLength() - thumbLength;
            if (
                view === undefined ||
                bounds === undefined ||
                event.button !== 0 ||
                isOnThumb ||
                room <= 0
            ) {
                return;
            }

            // have the thumb's middle under the pointer
            const isBackward = getComputedStyle(view).direction === "rtl";
            const along = isVertical
                ? event.clientY - bounds.top
                : isBackward
                  ? bounds.right - event.clientX
                  : event.clientX - bounds.left;
            scrollTo(view, ((along - thumbLength / 2) / room) * measure(view, axis).most);
        };

        // drag the thumb by the share of the room it has
        const move = new Move(
            {
                distance: 0,
                keys: [],
                orientation: axis,
                onMoveStart: () => {
                    const view = untrack(area.viewport);
                    start = view === undefined ? 0 : measure(view, axis).offset;
                },
                onMove: (event) => {
                    const view = untrack(area.viewport);
                    const room = trackLength() - thumbLength;
                    if (view !== undefined && room > 0) {
                        const isBackward =
                            !isVertical && getComputedStyle(view).direction === "rtl";
                        const offset = isVertical
                            ? event.offsetY
                            : event.offsetX * (isBackward ? -1 : 1);
                        scrollTo(view, start + (offset / room) * measure(view, axis).most);
                    }
                },
            },
            useEnvironment().clock,
        );

        return (
            <BarContext value={{ axis, setThumb: (element) => (thumbElement = element) }}>
                {renderPart(
                    "div",
                    "scroll-area-bar",
                    omit(properties, "orientation", "children"),
                    [styles.bar, bars[axis]],
                    mergeProperties(move.attributes(), {
                        "aria-hidden": "true" as const,
                        tabindex: -1,
                        "data-orientation": axis,
                        hidden: true,
                        ref: (element: HTMLElement) => {
                            bar = element;
                        },
                        get "data-scrolling"() {
                            return area.isScrolling() ? "" : undefined;
                        },
                        get "data-moving"() {
                            return move.isMoving() ? "" : undefined;
                        },
                        onPointerDown: press,
                        onWheel: (event: WheelEvent) =>
                            area.viewport()?.scrollBy({ left: event.deltaX, top: event.deltaY }),
                        get children() {
                            return contentOr(properties, () => <ScrollAreaThumb />);
                        },
                    }),
                )}
            </BarContext>
        );
    },
});

/** The properties of a scroll bar, its native attributes included. */
export type ScrollAreaBarProperties = ComponentProperties<typeof ScrollAreaBar>;

/** Render the part of a bar that stands for what is in view. */
export function ScrollAreaThumb(
    properties: Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "style"> & ElementPartProperties,
): JSX.Element {
    const bar = useContext(BarContext);
    if (bar === null) {
        throw new TypeError("a scroll area's thumb needs a bar around it");
    }

    return renderPart("div", "scroll-area-thumb", properties, [styles.thumb, thumbs[bar.axis]], {
        ref: (element) => bar.setThumb(element),
    });
}

/** Read the length in view, the whole, how far it scrolls at most and how far it is scrolled. */
function measure(
    view: HTMLElement,
    axis: Axis,
): {
    readonly length: number;
    readonly whole: number;
    readonly most: number;
    readonly offset: number;
} {
    const [length, whole, offset] =
        axis === "vertical"
            ? [view.clientHeight, view.scrollHeight, view.scrollTop]
            : [view.clientWidth, view.scrollWidth, Math.abs(view.scrollLeft)];

    return { length, whole, most: whole - length, offset };
}
