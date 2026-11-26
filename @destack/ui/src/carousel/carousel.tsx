import { field } from "@destack/object";
import { state } from "@destack/theme/state";
import { type Axis, ORIENTATION } from "../position/index.ts";
import { Icon } from "@destack/ui/icon";
import arrowDown from "@destack/icon/phosphor/arrow-down";
import arrowLeft from "@destack/icon/phosphor/arrow-left";
import arrowRight from "@destack/icon/phosphor/arrow-right";
import arrowUp from "@destack/icon/phosphor/arrow-up";
import pause from "@destack/icon/phosphor/pause";
import play from "@destack/icon/phosphor/play";
import { t, type Direction } from "@destack/locale";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, radius, space, swatch } from "@destack/theme/tokens.stylex";
import {
    type Accessor,
    createContext,
    createControllableSignal,
    createEffect,
    createSignal,
    For,
    isServer,
    type JSX,
    omit,
    onCleanup,
    useContext,
    useEnvironment,
    useLocale,
} from "@destack/view";
import { Hover } from "../interaction/index.ts";
import { Button, type ButtonProperties } from "../button/index.ts";
import {
    type ElementPartProperties,
    renderPart,
    mergeProperties,
    type Render,
} from "../part/index.ts";
import { CollectionBuilder } from "../collection/index.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The carousel of the nearest carousel, null outside one. */
const CarouselContext = createContext<CarouselControl | null>(null);

/** The styles of a carousel and its elements. */
const styles = style.create({
    carousel: {
        position: "relative",
    },
    dots: {
        display: "flex",
        justifyContent: "center",
        paddingBlock: space[1],
    },
    dot: {
        // draw a dot in the middle of a larger target
        width: space[4],
        height: space[4],
        padding: space[1],
        borderRadius: radius.full,
        backgroundColor: { default: color.border, [media.forcedColors]: "CanvasText" },
        backgroundClip: "content-box",
        cursor: "pointer",
    },
    current: {
        backgroundColor: { default: swatch.accentMark, [media.forcedColors]: "Highlight" },
    },
    viewport: {
        position: "relative",
        display: "flex",
        overflow: "auto",
        overscrollBehavior: "contain",
        scrollbarWidth: "none",
        "::-webkit-scrollbar": { display: "none" },
    },
    item: {
        flexShrink: 0,
        flexGrow: 0,
        flexBasis: "100%",
        minWidth: 0,
        scrollSnapAlign: "start",
    },
    previous: {
        position: "absolute",
    },
    next: {
        position: "absolute",
    },
});

/** The scroll axis of a carousel's viewport in each orientation. */
const viewports = style.create({
    horizontal: { flexDirection: "row", scrollSnapType: "x mandatory" },
    vertical: { flexDirection: "column", scrollSnapType: "y mandatory" },
});

/** The place of the previous button in each orientation. */
const previouses = style.create({
    horizontal: {
        top: "50%",
        insetInlineStart: `calc(-1 * ${space[7]})`,
        transform: "translateY(-50%)",
    },
    vertical: { top: `calc(-1 * ${space[7]})`, left: "50%", transform: "translateX(-50%)" },
});

/** The place of the next button in each orientation. */
const nexts = style.create({
    horizontal: {
        top: "50%",
        insetInlineEnd: `calc(-1 * ${space[7]})`,
        transform: "translateY(-50%)",
    },
    vertical: { bottom: `calc(-1 * ${space[7]})`, left: "50%", transform: "translateX(-50%)" },
});

/** The commands and state a carousel hands its owner. */
export interface CarouselApi {
    /** The index of the slide shown. */
    readonly index: Accessor<number>;
    /** The number of slides. */
    readonly count: Accessor<number>;
    /** Move to the previous slide. */
    readonly scrollPrevious: () => void;
    /** Move to the next slide. */
    readonly scrollNext: () => void;
    /** Move to a slide by index. */
    readonly scrollTo: (index: number) => void;
}

/** The slide shown and the slides of a carousel, which its viewport, slides and buttons share. */
export class CarouselControl {
    /** The direction the slides line up in. */
    readonly orientation: () => Axis;
    /** The number of slides. */
    readonly count: Accessor<number>;
    /** The properties of the carousel, read for its controlled index and change handler. */
    readonly #properties: RenderProperties<CarouselAttributes, typeof CAROUSEL_PROPERTIES>;
    /** The index of the slide shown, controlled or the carousel's own. */
    readonly index: Accessor<number>;
    /** Whether autoplay moves the slides. */
    readonly isPlaying: Accessor<boolean>;
    /** Replace the index of the slide shown and tell the change handler. */
    readonly #setIndex: (index: number) => void;
    /** Start or stop autoplay. */
    readonly #setPlaying: (isPlaying: boolean) => void;
    /** Whether the pointer or the focus rests in the carousel, which pauses autoplay. */
    #isResting: boolean;
    /** The slides in document order, by their elements. */
    readonly #slides: CollectionBuilder<Accessor<Element | undefined>>;
    /** The scrolling viewport. */
    #viewport: HTMLElement | undefined;
    /** Whether the index follows a person's scrolling, which needs no scroll back. */
    #isFollowing: boolean;

    /** Create a carousel without slides on its first or controlled slide. */
    constructor(
        properties: RenderProperties<CarouselAttributes, typeof CAROUSEL_PROPERTIES>,
        orientation: () => Axis,
    ) {
        // start on the default slide and play when autoplay is set
        const [index, setIndex] = createControllableSignal({
            isControlled: () => properties.index !== undefined,
            value: () => properties.index ?? 0,
            defaultValue: properties.defaultIndex ?? 0,
            onChange: (next) => properties.onIndexChange?.(next),
        });
        const [isPlaying, setPlaying] = createSignal(properties.autoplay !== undefined, {
            ownedWrite: true,
        });
        this.orientation = orientation;
        this.count = () => this.#slides.size();
        this.index = index;
        this.isPlaying = isPlaying;
        this.#properties = properties;
        this.#setIndex = setIndex;
        this.#setPlaying = setPlaying;
        this.#isResting = false;
        this.#slides = new CollectionBuilder((element) => element());
        this.#viewport = undefined;
        this.#isFollowing = false;
    }

    /** Hand the owner the carousel's commands and state. */
    api(): CarouselApi {
        return {
            index: () => this.index(),
            count: this.count,
            scrollPrevious: () => this.go(this.index() - 1),
            scrollNext: () => this.go(this.index() + 1),
            scrollTo: (index) => this.go(index),
        };
    }

    /** Add a slide until it unmounts, returning its place among the slides. */
    addSlide(element: Accessor<Element | undefined>): Accessor<number> {
        return this.#slides.add(element);
    }

    /** Set the scrolling viewport. */
    setViewport(element: HTMLElement): void {
        this.#viewport = element;
    }

    /** Report whether the carousel wraps from its last slide to its first. */
    isLooping(): boolean {
        return this.#properties.loop;
    }

    /** Move to a slide, and tell the change handler. */
    go(index: number): void {
        const count = this.count();
        const target = this.isLooping()
            ? ((index % count) + count) % count
            : Math.min(Math.max(index, 0), count - 1);
        if (count > 0 && target !== this.index()) {
            this.#setIndex(target);
        }
    }

    /** Start or stop autoplay. */
    play(isPlaying: boolean): void {
        this.#setPlaying(isPlaying);
    }

    /** Pause autoplay while the pointer or the focus rests in the carousel. */
    rest(isResting: boolean): void {
        this.#isResting = isResting;
    }

    /** Move to the next slide on an autoplay tick, wrapping at the end. */
    tick(): void {
        if (this.isPlaying() && !this.#isResting) {
            const count = this.count();
            this.#setIndex(count > 0 ? (this.index() + 1) % count : 0);
        }
    }

    /** Scroll the viewport to the shown slide's start. */
    scroll(index: number): void {
        // skip the slide a person scrolled to
        const slide = this.#viewport?.children[index];
        if (this.#isFollowing || this.#viewport === undefined || !(slide instanceof HTMLElement)) {
            this.#isFollowing = false;
            return;
        }

        // scroll along the carousel's axis
        const isHorizontal = this.orientation() === "horizontal";
        this.#viewport.scrollTo({
            [isHorizontal ? "left" : "top"]: isHorizontal ? slide.offsetLeft : slide.offsetTop,
            behavior: "smooth",
        });
    }

    /** Follow a person scrolling the viewport to the slide nearest its start. */
    follow(viewport: HTMLElement): void {
        // read the scroll position against one slide's extent
        const isHorizontal = this.orientation() === "horizontal";
        const position = isHorizontal ? viewport.scrollLeft : viewport.scrollTop;
        const extent = isHorizontal ? viewport.clientWidth : viewport.clientHeight;
        const index = extent > 0 ? Math.round(Math.abs(position) / extent) : this.index();
        if (index !== this.index()) {
            this.#isFollowing = true;
            this.go(index);
        }
    }
}

/** The properties people change in a carousel's controls and inspector. */
const CAROUSEL_PROPERTIES = {
    orientation: ORIENTATION.described(t`Whether the slides line up across or down.`).default(
        "horizontal",
    ),
    loop: field
        .boolean()
        .labeled(t.context("label")`Loop`)
        .described(t`Wraps from its last slide to its first and back.`)
        .default(false),
};

/** The properties of a carousel besides its declared ones: its slide, autoplay and handlers. */
export interface CarouselAttributes extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> {
    /** The index of the slide shown, which makes it controlled. */
    readonly index?: number;
    /** The index of the slide shown at first while uncontrolled, the first by default. */
    readonly defaultIndex?: number;
    /** The time each slide shows before the next while autoplay runs, in milliseconds. */
    readonly autoplay?: number;
    /** Handle another slide being shown, by a button, a key or a person's scrolling. */
    readonly onIndexChange?: (index: number) => void;
    /** Receive the carousel's commands and state once it mounts. */
    readonly setApi?: (api: CarouselApi) => void;
    /** The StyleX styles applied after the carousel's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with its attributes, its own by default. */
    readonly render?: Render;
}

/** The properties of an element of a carousel, the native element's attributes included. */
export type CarouselElementProperties<Attributes> = Omit<Attributes, "class"> &
    ElementPartProperties;

/** Read the carousel around an element, refusing elements outside one. */
export function useCarousel(): CarouselControl {
    const control = useContext(CarouselContext);
    if (control === null) {
        throw new TypeError("carousel elements need a carousel around them");
    }

    return control;
}

/** Render a region of slides that scroll and snap. */
export const Carousel = defineComponent({
    name: "carousel",
    properties: CAROUSEL_PROPERTIES,
    render(
        properties: RenderProperties<CarouselAttributes, typeof CAROUSEL_PROPERTIES>,
    ): JSX.Element {
        // share one carousel with its slides and buttons
        const control = new CarouselControl(properties, () => properties.orientation);
        const locale = useLocale();
        const rest = omit(
            properties,
            "orientation",
            "index",
            "defaultIndex",
            "onIndexChange",
            "loop",
            "autoplay",
            "setApi",
        );

        // move to the next slide on each autoplay tick of the page's clock, which a server render has none of
        const delay = properties.autoplay;
        if (delay !== undefined && !isServer) {
            const { clock } = useEnvironment();
            const timer = clock.setInterval(() => control.tick(), delay);
            onCleanup(() => clock.clearInterval(timer));
        }
        createEffect(
            () => control.index(),
            (index) => control.scroll(index),
        );
        properties.setApi?.(control.api());
        const hover = new Hover({
            onHoverStart: () => control.rest(true),
            onHoverEnd: () => control.rest(false),
        });

        return (
            <CarouselContext value={control}>
                {renderPart(
                    "div",
                    "carousel",
                    rest,
                    styles.carousel,
                    mergeProperties(hover.attributes(), {
                        role: "region" as const,
                        get "aria-roledescription"() {
                            return locale.render(t.context("title")`carousel`);
                        },
                        get "data-orientation"() {
                            return properties.orientation;
                        },
                        onFocusIn: () => control.rest(true),
                        onFocusOut: () => control.rest(false),
                        onKeyDown: (
                            event: KeyboardEvent & { readonly currentTarget: HTMLElement },
                        ) => {
                            // move to the previous or next slide along the reading direction
                            const step = stepOf(
                                event.key,
                                properties.orientation,
                                locale.direction,
                            );
                            if (step !== 0) {
                                event.preventDefault();
                                control.go(control.index() + step);
                            }
                        },
                    }),
                )}
            </CarouselContext>
        );
    },
});

/** The properties of a carousel, its native attributes included. */
export type CarouselProperties = ComponentProperties<typeof Carousel>;

/** Render the viewport the slides scroll in. */
export function CarouselContent(
    properties: CarouselElementProperties<
        Omit<JSX.HTMLAttributes<HTMLDivElement>, "ref" | "onScroll">
    >,
): JSX.Element {
    const control = useCarousel();

    return renderPart(
        "div",
        "carousel-content",
        properties,
        () => [state.focusInset, styles.viewport, viewports[control.orientation()]],
        {
            tabindex: 0,
            get "aria-live"() {
                return control.isPlaying() ? "off" : "polite";
            },
            ref: (element) => control.setViewport(element),
            onScroll: (event) => control.follow(event.currentTarget),
        },
    );
}

/** Render a slide, named by its place among the slides. */
export function CarouselItem(
    properties: CarouselElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    // read the carousel, the locale and the reading direction
    const control = useCarousel();
    const locale = useLocale();
    let element: HTMLElement | undefined;
    const index = control.addSlide(() => element);

    return renderPart("div", "carousel-item", properties, styles.item, {
        role: "group",
        ref: (created: HTMLElement) => (element = created),
        get "aria-roledescription"() {
            return locale.render(t.context("title")`slide`);
        },
        get "aria-label"() {
            return locale.render(t`${index() + 1} of ${control.count()}`);
        },
    });
}

/** Render the button that moves to the previous slide, disabled on the first. */
export function CarouselPrevious(
    properties: Omit<ButtonProperties, "onClick" | "children">,
): JSX.Element {
    // read the carousel, the locale and the reading direction
    const control = useCarousel();
    const locale = useLocale();
    const rest = omit(properties, "xstyle", "style");
    const isVertical = (): boolean => control.orientation() === "vertical";
    const isRightToLeft = (): boolean => locale.direction === "rtl";

    return (
        <Button
            isIconOnly
            variant="outline"

            aria-label={locale.render(t.context("action")`Previous slide`)}
            disabled={!control.isLooping() && control.index() === 0}
            data-slot="carousel-previous"
            {...rest}
            onClick={() => control.go(control.index() - 1)}
            xstyle={[styles.previous, previouses[control.orientation()], properties.xstyle]}
        >
            <Icon icon={isVertical() ? arrowUp : isRightToLeft() ? arrowRight : arrowLeft} />
        </Button>
    );
}

/** Render the button that moves to the next slide, disabled on the last. */
export function CarouselNext(
    properties: Omit<ButtonProperties, "onClick" | "children">,
): JSX.Element {
    // read the carousel, the locale and the reading direction
    const control = useCarousel();
    const locale = useLocale();
    const rest = omit(properties, "xstyle", "style");
    const isVertical = (): boolean => control.orientation() === "vertical";
    const isRightToLeft = (): boolean => locale.direction === "rtl";

    return (
        <Button
            isIconOnly
            variant="outline"

            aria-label={locale.render(t.context("action")`Next slide`)}
            disabled={!control.isLooping() && control.index() >= control.count() - 1}
            data-slot="carousel-next"
            {...rest}
            onClick={() => control.go(control.index() + 1)}
            xstyle={[styles.next, nexts[control.orientation()], properties.xstyle]}
        >
            <Icon icon={isVertical() ? arrowDown : isRightToLeft() ? arrowLeft : arrowRight} />
        </Button>
    );
}

/** Render the button that stops and starts autoplay, which a rotating carousel must offer. */
export function CarouselPlay(
    properties: Omit<ButtonProperties, "onClick" | "children">,
): JSX.Element {
    const control = useCarousel();
    const locale = useLocale();

    return (
        <Button
            isIconOnly
            variant="outline"

            aria-label={locale.render(
                control.isPlaying() ? t`Stop slide rotation` : t`Start slide rotation`,
            )}
            data-slot="carousel-play"
            data-playing={control.isPlaying() ? "" : undefined}
            data-paused={control.isPlaying() ? undefined : ""}
            {...properties}
            onClick={() => control.play(!control.isPlaying())}
        >
            <Icon icon={control.isPlaying() ? pause : play} />
        </Button>
    );
}

/** Render a dot per slide that moves to it, marking the slide shown. */
export function CarouselDots(
    properties: CarouselElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    // read the slides and the one shown
    const control = useCarousel();
    const locale = useLocale();

    return renderPart("div", "carousel-dots", properties, styles.dots, {
        role: "group",
        get "aria-label"() {
            return locale.render(t.context("title")`Slides`);
        },
        get children() {
            return (
                <>
                    <For each={Array.from({ length: control.count() }, (_, index) => index)}>
                        {(index) => (
                            <button
                                type="button"
                                aria-label={locale.render(t`Go to slide ${index + 1}`)}
                                aria-current={control.index() === index ? "true" : undefined}
                                data-slot="carousel-dot"
                                onClick={() => control.go(index)}
                                {...style.attrs(
                                    state.touch,
                                    styles.dot,
                                    control.index() === index && styles.current,
                                )}
                            />
                        )}
                    </For>
                </>
            );
        },
    });
}

/** Read the slides a key moves by, mirroring left and right in right-to-left text. */
function stepOf(key: string, orientation: Axis, direction: Direction): number {
    const forward =
        orientation === "vertical" ? "ArrowDown" : direction === "rtl" ? "ArrowLeft" : "ArrowRight";
    const backward =
        orientation === "vertical" ? "ArrowUp" : direction === "rtl" ? "ArrowRight" : "ArrowLeft";

    return key === forward ? 1 : key === backward ? -1 : 0;
}
