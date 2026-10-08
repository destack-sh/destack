import { isServer } from "@solidjs/web";
import {
    type Accessor,
    createEffect,
    createRenderEffect,
    createSignal,
    sharedConfig,
} from "solid-js";
import { access, type MaybeAccessor, TRANSPARENT, onMount } from "./utils.ts";
import { createHydratableSingletonRoot } from "./rootless.ts";
import { createDerivedStaticStore } from "./static-store.ts";

/** A scroll offset. */
export type Position = {
    /** The horizontal offset. */
    readonly x: number;
    /** The vertical offset. */
    readonly y: number;
};

/** How scrolling outside an element is prevented. */
export type CreatePreventScrollProperties = {
    /** The element that may still scroll, none by default. */
    readonly element?: MaybeAccessor<HTMLElement | undefined>;
    /** Whether scrolling is prevented, true by default. */
    readonly enabled?: MaybeAccessor<boolean>;
    /** Whether the document's scrollbar hides, true by default. */
    readonly hideScrollbar?: MaybeAccessor<boolean>;
    /** Whether the document makes up for the hidden scrollbar's width, true by default. */
    readonly preventScrollbarShift?: MaybeAccessor<boolean>;
    /** Whether the width is made up with padding or margin, padding by default. */
    readonly preventScrollbarShiftMode?: MaybeAccessor<"padding" | "margin">;
    /** Whether the scroll position is restored once scrolling is allowed again, true by default. */
    readonly restoreScrollPosition?: MaybeAccessor<boolean>;
    /** Whether two-finger pinch zoom stays allowed, false by default. */
    readonly allowPinchZoom?: MaybeAccessor<boolean>;
};

/** An axis to scroll along. */
type Axis = "x" | "y";

/** A style applied to the document by every active instance, restored after the last. */
type ActiveStyle = {
    /** How many instances apply it. */
    count: number;
    /** The values the style replaced, by CSS property name. */
    readonly original: ReadonlyMap<string, string>;
};

/** The scroll position on the server. */
const FALLBACK_SCROLL_POSITION: Position = { x: 0, y: 0 };

/** The instances preventing scroll, the latest on top, whose events only the top one handles. */
const stack = createSignal<string[]>([], { ownedWrite: true });

/** The document styles active instances apply. */
const activeStyles = new Map<string, ActiveStyle>();

/** The next instance's identifier. */
let nextIdentifier = 0;

/** Find the nearest ancestor of a node that scrolls, else the document's scrolling element. */
export function getScrollParent(node: Element | undefined): Element {
    // find nothing on the server, which has no document
    if (isServer) {
        throw new TypeError("scroll parents exist only in a document");
    }

    // climb until an ancestor scrolls
    let current = node;
    while (current !== undefined && !isScrollable(current)) {
        current = current.parentElement ?? undefined;
    }

    return current ?? document.scrollingElement ?? document.documentElement;
}

/** Check whether an element's overflow lets it scroll. */
export function isScrollable(node: Element): boolean {
    // nothing scrolls on the server
    if (isServer) {
        return false;
    }
    const style = window.getComputedStyle(node);

    return /(?:auto|scroll)/u.test(style.overflow + style.overflowX + style.overflowY);
}

/** Read the scroll position of an element or the window. */
export function getScrollPosition(target: Element | Window | undefined): Position {
    // read zero on the server or without a target
    if (isServer || target === undefined) {
        return FALLBACK_SCROLL_POSITION;
    }

    return target instanceof Window
        ? { x: target.scrollX, y: target.scrollY }
        : { x: target.scrollLeft, y: target.scrollTop };
}

/** Follow the scroll position of an element or the window, the window by default. */
export function createScrollPosition(
    target?: Accessor<Element | Window | undefined> | Element | Window,
): Readonly<Position> {
    // read the window by default in the browser
    const source = isServer ? target : (target ?? window);
    const isHydrating = sharedConfig.hydrating;
    const read = (): Position => {
        if (isServer) {
            return { ...FALLBACK_SCROLL_POSITION };
        }

        return getScrollPosition(typeof source === "function" ? source() : source);
    };

    // reread on each scroll, keeping zero until hydration settles
    const [version, setVersion] = createSignal(0, { ownedWrite: true });
    const update = (): void => {
        setVersion((count) => count + 1);
    };
    const position = createDerivedStaticStore<Position>(() => {
        const current = version();

        return isHydrating && current === 0 ? { ...FALLBACK_SCROLL_POSITION } : read();
    });
    if (!isServer && (isHydrating || typeof source === "function")) {
        onMount(update);
    }

    // listen to the target's scroll events, through an effect on both sides to keep hydration keys aligned
    const listen = (element: Element | Window | undefined): (() => void) | undefined => {
        if (isServer || element === undefined) {
            return undefined;
        }
        element.addEventListener("scroll", update, { passive: true });

        return () => element.removeEventListener("scroll", update);
    };
    if (typeof source === "function") {
        createEffect(source, listen);
    } else {
        createRenderEffect(() => source, listen);
    }

    return position;
}

/** Follow the window's scroll position through one listener shared by every user. */
export const useWindowScrollPosition: () => Readonly<Position> = createHydratableSingletonRoot(() =>
    createScrollPosition(isServer ? () => undefined : window),
);

/** Prevent scrolling outside an element, hiding the document's scrollbar, while enabled. */
export function createPreventScroll(properties: CreatePreventScrollProperties = {}): void {
    // prevent nothing on the server
    if (isServer) {
        return;
    }
    const identifier = String(nextIdentifier++);
    const [, setStack] = stack;

    // join the stack of instances while enabled
    createEffect(
        () => access(properties.enabled) ?? true,
        (isEnabled) => {
            if (!isEnabled) {
                return undefined;
            }
            setStack((current) => [...current, identifier]);

            return () => setStack((current) => current.filter((entry) => entry !== identifier));
        },
        TRANSPARENT,
    );

    // hide the document's scrollbar, making up for its width
    createEffect(
        () => ({
            isEnabled: access(properties.enabled) ?? true,
            shouldHide: access(properties.hideScrollbar) ?? true,
            shouldPreventShift: access(properties.preventScrollbarShift) ?? true,
            shiftMode: access(properties.preventScrollbarShiftMode) ?? "padding",
            shouldRestore: access(properties.restoreScrollPosition) ?? true,
        }),
        ({ isEnabled, shouldHide, shouldPreventShift, shiftMode, shouldRestore }) => {
            if (!isEnabled || !shouldHide) {
                return undefined;
            }

            // measure the scrollbar and the scroll position before hiding it
            const root = document.documentElement;
            const scrollbarWidth = window.innerWidth - root.clientWidth;
            const offsetTop = window.scrollY;
            const offsetLeft = window.scrollX;
            const style = new Map<string, string>([["overflow", "hidden"]]);
            if (shouldPreventShift && scrollbarWidth > 0) {
                const side = shiftMode === "padding" ? "padding-right" : "margin-right";
                style.set(
                    side,
                    `calc(${window.getComputedStyle(root).getPropertyValue(side)} + ${scrollbarWidth}px)`,
                );
                style.set("--scrollbar-width", `${scrollbarWidth}px`);
            }
            const restore = applyDocumentStyle("prevent-scroll", root, style);

            return () => {
                restore();
                if (shouldRestore && scrollbarWidth > 0) {
                    window.scrollTo(offsetLeft, offsetTop);
                }
            };
        },
        TRANSPARENT,
    );

    // cancel wheel and touch scrolling outside the element while on top of the stack
    createEffect(
        () => ({
            isTop: stack[0]().at(-1) === identifier,
            isEnabled: access(properties.enabled) ?? true,
            wrapper: access(properties.element),
            allowsPinchZoom: access(properties.allowPinchZoom) ?? false,
        }),
        ({ isTop, isEnabled, wrapper, allowsPinchZoom }) => {
            if (!isTop || !isEnabled) {
                return undefined;
            }

            // cancel a wheel turn that would not scroll inside the element
            const preventWheel = (event: WheelEvent): void => {
                // read the target and the axis the wheel turns along
                const target = event.target;
                if (!(target instanceof HTMLElement)) {
                    return;
                }
                const axis: Axis = Math.abs(event.deltaX) > Math.abs(event.deltaY) ? "x" : "y";
                const delta = axis === "x" ? event.deltaX : event.deltaY;
                const isInside = wrapper?.contains(target) === true;
                if ((!isInside || !wouldScroll(target, axis, delta, wrapper)) && event.cancelable) {
                    event.preventDefault();
                }
            };

            // remember where a touch starts, and read its axis from its first move
            let touchStart: [number, number] = [0, 0];
            let touchAxis: Axis | undefined;
            let touchDelta: number | undefined;
            const startTouch = (event: TouchEvent): void => {
                // remember the touch's start
                const touch = event.changedTouches[0];
                touchStart = touch === undefined ? [0, 0] : [touch.clientX, touch.clientY];
                touchAxis = undefined;
                touchDelta = undefined;
            };

            // cancel a touch move that would not scroll inside the element, and pinches unless allowed
            const preventTouch = (event: TouchEvent): void => {
                // read the touched element
                const target = event.target;
                if (!(target instanceof HTMLElement)) {
                    return;
                }
                let shouldCancel: boolean;
                if (event.touches.length === 2) {
                    shouldCancel = !allowsPinchZoom;
                } else if (target instanceof HTMLInputElement && target.type === "range") {
                    shouldCancel = false;
                } else {
                    if (touchAxis === undefined || touchDelta === undefined) {
                        const touch = event.changedTouches[0];
                        const current: [number, number] =
                            touch === undefined ? [0, 0] : [touch.clientX, touch.clientY];
                        const deltaX = touchStart[0] - current[0];
                        const deltaY = touchStart[1] - current[1];
                        touchAxis = Math.abs(deltaX) > Math.abs(deltaY) ? "x" : "y";
                        touchDelta = touchAxis === "x" ? deltaX : deltaY;
                    }
                    const isInside = wrapper?.contains(target) === true;
                    shouldCancel =
                        !isInside || !wouldScroll(target, touchAxis, touchDelta, wrapper);
                }
                if (shouldCancel && event.cancelable) {
                    event.preventDefault();
                }
            };

            // listen to the document until stopped
            document.addEventListener("wheel", preventWheel, { passive: false });
            document.addEventListener("touchstart", startTouch, { passive: false });
            document.addEventListener("touchmove", preventTouch, { passive: false });

            return () => {
                document.removeEventListener("wheel", preventWheel);
                document.removeEventListener("touchstart", startTouch);
                document.removeEventListener("touchmove", preventTouch);
            };
        },
        TRANSPARENT,
    );
}

/** Apply a style to the document for one more instance, returning how to release it, restored after the last. */
function applyDocumentStyle(
    key: string,
    element: HTMLElement,
    style: ReadonlyMap<string, string>,
): () => void {
    // remember the replaced values on first use, and count every other
    const active = activeStyles.get(key);
    if (active === undefined) {
        const original = new Map(
            [...style.keys()].map((name) => [name, element.style.getPropertyValue(name)]),
        );
        activeStyles.set(key, { count: 1, original });
    } else {
        active.count++;
    }
    for (const [name, value] of style) {
        element.style.setProperty(name, value);
    }

    return () => {
        // restore the replaced values once the last instance releases the style
        const current = activeStyles.get(key);
        if (current === undefined) {
            return;
        }
        current.count--;
        if (current.count > 0) {
            return;
        }
        activeStyles.delete(key);
        for (const [name, value] of current.original) {
            if (value === "") {
                element.style.removeProperty(name);
            } else {
                element.style.setProperty(name, value);
            }
        }
        if (element.style.length === 0) {
            element.removeAttribute("style");
        }
    };
}

/** Check whether scrolling a target along an axis by a delta would move anything up to the wrapper. */
function wouldScroll(
    target: HTMLElement,
    axis: Axis,
    delta: number,
    wrapper: HTMLElement | undefined,
): boolean {
    // sum the scroll left in each container from the target up to the wrapper or the document
    const stopAt = wrapper?.contains(target) === true ? wrapper : document.documentElement;
    const direction = axis === "x" && window.getComputedStyle(target).direction === "rtl" ? -1 : 1;
    let available = 0;
    let availableBefore = 0;
    let current: HTMLElement | null = target;
    while (current !== null) {
        const clientSize = axis === "x" ? current.clientWidth : current.clientHeight;
        const offset = axis === "x" ? current.scrollLeft : current.scrollTop;
        const scrollSize = axis === "x" ? current.scrollWidth : current.scrollHeight;
        const remaining = scrollSize - clientSize - direction * offset;
        if ((offset !== 0 || remaining !== 0) && isScrollContainer(current, axis)) {
            available += remaining;
            availableBefore += offset;
        }
        current = current === stopAt ? null : current.parentElement;
    }

    // count a pixel of slack as no scroll, as some browsers report one where none is possible
    if (delta > 0) {
        return Math.abs(available) > 1;
    }

    return delta < 0 ? Math.abs(availableBefore) >= 1 : true;
}

/** Check whether an element scrolls along an axis, the root scrolling while its overflow is visible. */
function isScrollContainer(element: HTMLElement, axis: Axis): boolean {
    const style = getComputedStyle(element);
    const overflow = axis === "x" ? style.overflowX : style.overflowY;

    return (
        overflow === "auto" ||
        overflow === "scroll" ||
        (element.tagName === "HTML" && overflow === "visible")
    );
}
