import { Icon } from "@destack/icon";
import arrowDown from "@destack/icon/phosphor/arrow-down";
import arrowLeft from "@destack/icon/phosphor/arrow-left";
import arrowRight from "@destack/icon/phosphor/arrow-right";
import arrowUp from "@destack/icon/phosphor/arrow-up";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { useLocale } from "@destack/locale/solid";
import type { JSX } from "@solidjs/web";
import {
    createContext,
    createEffect,
    createSignal,
    merge,
    omit,
    onCleanup,
    untrack,
    useContext,
    type Accessor,
    type Setter,
} from "solid-js";
import { Button, type ButtonProperties } from "../button/index.ts";

/** The orientation of a carousel that sets none. */
const DEFAULTS: Required<Pick<CarouselProperties, "orientation">> = { orientation: "horizontal" };

/** The carousel of the nearest carousel, null outside one. */
const CarouselContext = createContext<CarouselControl | null>(null);

/** The styles of a carousel and its elements. */
const styles = style.create({
    carousel: {
        position: "relative",
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
        insetInlineStart: `calc(-1 * ${space[6]})`,
        transform: "translateY(-50%)",
    },
    vertical: { top: `calc(-1 * ${space[6]})`, left: "50%", transform: "translateX(-50%)" },
});

/** The place of the next button in each orientation. */
const nexts = style.create({
    horizontal: {
        top: "50%",
        insetInlineEnd: `calc(-1 * ${space[6]})`,
        transform: "translateY(-50%)",
    },
    vertical: { bottom: `calc(-1 * ${space[6]})`, left: "50%", transform: "translateX(-50%)" },
});

/** The commands and state a carousel hands its owner, after Embla's API. */
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
    readonly orientation: () => "horizontal" | "vertical";
    /** The number of slides. */
    readonly count: Accessor<number>;
    /** The properties of the carousel, read for its controlled index and change handler. */
    readonly #properties: CarouselProperties;
    /** The index of the slide shown when uncontrolled. */
    readonly #ownIndex: Accessor<number>;
    /** Replace the index of the slide shown when uncontrolled. */
    readonly #setIndex: Setter<number>;
    /** Replace the number of slides. */
    readonly #setCount: Setter<number>;
    /** The scrolling viewport. */
    #viewport: HTMLElement | undefined;
    /** Whether the index follows a person's scrolling, which needs no scroll back. */
    #isFollowing: boolean;

    /** Create a carousel without slides on its first or controlled slide. */
    constructor(properties: CarouselProperties, orientation: () => "horizontal" | "vertical") {
        // start on the first slide without slides
        const [index, setIndex] = createSignal(0);
        const [count, setCount] = createSignal(0, { ownedWrite: true });
        this.orientation = orientation;
        this.count = count;
        this.#properties = properties;
        this.#ownIndex = index;
        this.#setIndex = setIndex;
        this.#setCount = setCount;
        this.#viewport = undefined;
        this.#isFollowing = false;
    }

    /** Read the index of the slide shown, controlled or the carousel's own. */
    index(): number {
        return this.#properties.index ?? this.#ownIndex();
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

    /** Count a slide until it unmounts, returning its index. */
    register(): number {
        // take the next index and count the slide until it unmounts
        const index = untrack(this.count);
        this.#setCount((count) => count + 1);
        onCleanup(() => this.#setCount((count) => count - 1));

        return index;
    }

    /** Set the scrolling viewport. */
    setViewport(element: HTMLElement): void {
        this.#viewport = element;
    }

    /** Move to a slide within the carousel, staying on the first and last, and tell the change handler. */
    go(index: number): void {
        const target = Math.min(Math.max(index, 0), this.count() - 1);
        if (target !== this.index()) {
            this.#setIndex(target);
            this.#properties.onIndexChange?.(target);
        }
    }

    /** Scroll the viewport to the shown slide's start, unless the person's scrolling put it there. */
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

/** The properties of a carousel, the native element's attributes included. */
export interface CarouselProperties extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "style" | "onKeyDown"
> {
    /** The direction the slides line up in, horizontal by default. */
    readonly orientation?: "horizontal" | "vertical";
    /** The index of the slide shown, which makes it controlled. */
    readonly index?: number;
    /** Handle another slide being shown, by a button, a key or a person's scrolling. */
    readonly onIndexChange?: (index: number) => void;
    /** Receive the carousel's commands and state once it mounts. */
    readonly api?: (api: CarouselApi) => void;
    /** The StyleX styles applied after the carousel's styles. */
    readonly style?: style.Styles;
}

/** The properties of an element of a carousel, the native element's attributes included. */
export type CarouselElementProperties<Attributes> = Omit<Attributes, "class" | "style"> & {
    /** The StyleX styles applied after the element's styles. */
    readonly style?: style.Styles;
};

/** Read the carousel around an element, refusing elements outside one. */
function useCarousel(): CarouselControl {
    const control = useContext(CarouselContext);
    if (control === null) {
        throw new TypeError("carousel elements need a carousel around them");
    }

    return control;
}

/** Render a region of slides that scroll and snap, which the arrow keys and its buttons move through. */
export function Carousel(properties: CarouselProperties): JSX.Element {
    // share one carousel with its slides and buttons
    const carousel = merge(DEFAULTS, properties);
    const control = new CarouselControl(properties, () => carousel.orientation);
    const locale = useLocale();
    const rest = omit(carousel, "orientation", "index", "onIndexChange", "api", "style");
    createEffect(
        () => control.index(),
        (index) => control.scroll(index),
    );
    properties.api?.(control.api());

    return (
        <CarouselContext value={control}>
            <div
                role="region"
                aria-roledescription={locale.render(t`carousel`)}
                data-slot="carousel"
                data-orientation={carousel.orientation}
                {...rest}
                onKeyDown={(event) => {
                    // move to the previous or next slide along the reading direction
                    const step = stepOf(event.key, carousel.orientation, locale.direction);
                    if (step !== 0) {
                        event.preventDefault();
                        control.go(control.index() + step);
                    }
                }}
                {...style.attrs(styles.carousel, carousel.style)}
            />
        </CarouselContext>
    );
}

/** Render the scrolling viewport that holds the slides. */
export function CarouselContent(
    properties: CarouselElementProperties<
        Omit<JSX.HTMLAttributes<HTMLDivElement>, "ref" | "onScroll">
    >,
): JSX.Element {
    const control = useCarousel();
    const rest = omit(properties, "style");

    return (
        <div
            data-slot="carousel-content"
            {...rest}
            ref={(element) => control.setViewport(element)}
            onScroll={(event) => control.follow(event.currentTarget)}
            {...style.attrs(styles.viewport, viewports[control.orientation()], properties.style)}
        />
    );
}

/** Render a slide, named by its place among the slides. */
export function CarouselItem(
    properties: CarouselElementProperties<JSX.HTMLAttributes<HTMLDivElement>>,
): JSX.Element {
    // read the carousel, the locale and the reading direction
    const control = useCarousel();
    const locale = useLocale();
    const index = control.register();
    const rest = omit(properties, "style");

    return (
        <div
            role="group"
            aria-roledescription={locale.render(t`slide`)}
            aria-label={locale.render(t`${index + 1} of ${control.count()}`)}
            data-slot="carousel-item"
            {...rest}
            {...style.attrs(styles.item, properties.style)}
        />
    );
}

/** Render the button that moves to the previous slide, disabled on the first. */
export function CarouselPrevious(
    properties: Omit<ButtonProperties, "onClick" | "children">,
): JSX.Element {
    // read the carousel, the locale and the reading direction
    const control = useCarousel();
    const locale = useLocale();
    const rest = omit(properties, "style");
    const isVertical = (): boolean => control.orientation() === "vertical";
    const isRightToLeft = (): boolean => locale.direction === "rtl";

    return (
        <Button
            variant="outline"
            size="icon-sm"
            aria-label={locale.render(t`Previous slide`)}
            disabled={control.index() === 0}
            data-slot="carousel-previous"
            {...rest}
            onClick={() => control.go(control.index() - 1)}
            style={[styles.previous, previouses[control.orientation()], properties.style]}
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
    const rest = omit(properties, "style");
    const isVertical = (): boolean => control.orientation() === "vertical";
    const isRightToLeft = (): boolean => locale.direction === "rtl";

    return (
        <Button
            variant="outline"
            size="icon-sm"
            aria-label={locale.render(t`Next slide`)}
            disabled={control.index() >= control.count() - 1}
            data-slot="carousel-next"
            {...rest}
            onClick={() => control.go(control.index() + 1)}
            style={[styles.next, nexts[control.orientation()], properties.style]}
        >
            <Icon icon={isVertical() ? arrowDown : isRightToLeft() ? arrowLeft : arrowRight} />
        </Button>
    );
}

/** Read the slides a key moves by, mirroring left and right in right-to-left text. */
function stepOf(
    key: string,
    orientation: "horizontal" | "vertical",
    direction: "ltr" | "rtl",
): number {
    const forward =
        orientation === "vertical" ? "ArrowDown" : direction === "rtl" ? "ArrowLeft" : "ArrowRight";
    const backward =
        orientation === "vertical" ? "ArrowUp" : direction === "rtl" ? "ArrowRight" : "ArrowLeft";

    return key === forward ? 1 : key === backward ? -1 : 0;
}
