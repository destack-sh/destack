import { isServer } from "@solidjs/web";
import {
    type Accessor,
    createEffect,
    createMemo,
    createSignal,
    onCleanup,
    sharedConfig,
} from "solid-js";
import { access, TRANSPARENT, type FalsyValue, onMount } from "./utils.ts";
import { makeEventListener } from "./event-listener.ts";

/** An element's rectangle on screen. */
export type Bounds = {
    /** The top edge. */
    readonly top: number;
    /** The left edge. */
    readonly left: number;
    /** The bottom edge. */
    readonly bottom: number;
    /** The right edge. */
    readonly right: number;
    /** The width. */
    readonly width: number;
    /** The height. */
    readonly height: number;
};

/** An element's rectangle, all null while there is no element. */
export type NullableBounds = { readonly [Key in keyof Bounds]: number | null };

/** Wrap a bounds update, such as to throttle it. */
export type UpdateGuard = <Args extends unknown[]>(
    update: (...args: Args) => void,
) => (...args: Args) => void;

/** What moves bounds: scrolling, DOM mutations and resizes, each tracked by default or through a guard. */
export type Options = {
    /** Whether, or through which guard, scrolling updates the bounds. */
    readonly trackScroll?: boolean | UpdateGuard;
    /** Whether, or through which guard, DOM mutations update the bounds. */
    readonly trackMutation?: boolean | UpdateGuard;
    /** Whether, or through which guard, the element's resizes update the bounds. */
    readonly trackResize?: boolean | UpdateGuard;
};

/** The bounds of no element. */
const NULLED_BOUNDS: NullableBounds = {
    top: null,
    left: null,
    bottom: null,
    right: null,
    width: null,
    height: null,
};

/** Read an element's rectangle on screen. */
export function getElementBounds(element: Element): Bounds;
/** Read an element's rectangle, all null without an element. */
export function getElementBounds(element: Element | FalsyValue): NullableBounds;
/**
 * Measure an element's rectangle.
 *
 * @construct an element always measures to numbers, so only an absent one reads null
 */
export function getElementBounds(element: Element | FalsyValue): NullableBounds {
    // measure nothing on the server or without an element
    if (
        isServer ||
        element === false ||
        element === 0 ||
        element === "" ||
        element === null ||
        element === undefined
    ) {
        return { ...NULLED_BOUNDS };
    }
    const { top, left, bottom, right, width, height } = element.getBoundingClientRect();

    return { top, left, bottom, right, width, height };
}

/** Follow an element's rectangle as the page scrolls, the DOM mutates and the element resizes. */
export function createElementBounds(
    target: Accessor<Element | FalsyValue> | Element,
    { trackMutation = true, trackResize = true, trackScroll = true }: Options = {},
): Readonly<NullableBounds> {
    // measure nothing on the server
    const isAccessor = typeof target === "function";
    if (isServer) {
        return NULLED_BOUNDS;
    }

    // measure from the server's nulls while hydrating, and remeasure an accessor's ref once settled
    const [track, trigger] = createSignal(undefined, { equals: false, ownedWrite: true });
    const update = (): void => trigger(undefined);
    let measure = (): NullableBounds => getElementBounds(access(target));
    if (sharedConfig.hydrating) {
        measure = () => NULLED_BOUNDS;
        onMount(() => {
            measure = () => getElementBounds(access(target));
            update();
        });
    } else if (isAccessor) {
        onMount(update);
    }

    // read each edge through its getter
    const current = createMemo(() => {
        track();

        return measure();
    }, TRANSPARENT);
    const bounds = { ...NULLED_BOUNDS };
    for (const key of Object.keys(NULLED_BOUNDS)) {
        Object.defineProperty(bounds, key, {
            enumerable: true,
            get: (): unknown => Reflect.get(current(), key),
        });
    }

    // remeasure when the element resizes
    if (trackResize !== false) {
        const observer = new ResizeObserver(
            typeof trackResize === "function" ? trackResize(update) : update,
        );
        if (isAccessor) {
            createEffect(
                target,
                (element) => {
                    if (!(element instanceof Element)) {
                        return undefined;
                    }
                    observer.observe(element);

                    return () => observer.unobserve(element);
                },
                TRANSPARENT,
            );
        } else {
            observer.observe(target);
        }
        onCleanup(() => observer.disconnect());
    }

    // remeasure when something holding the element scrolls
    if (trackScroll !== false) {
        const scrolled = (event: Event): void => {
            const element = access(target);
            if (
                element instanceof Element &&
                event.target instanceof Node &&
                event.target.contains(element)
            ) {
                update();
            }
        };
        makeEventListener(
            window,
            "scroll",
            typeof trackScroll === "function" ? trackScroll(scrolled) : scrolled,
            {
                capture: true,
            },
        );
    }

    // remeasure when the document's structure, styles or classes change
    if (trackMutation !== false) {
        const observer = new MutationObserver(
            typeof trackMutation === "function" ? trackMutation(update) : update,
        );
        observer.observe(document.body, {
            attributeFilter: ["style", "class"],
            subtree: true,
            childList: true,
        });
        onCleanup(() => observer.disconnect());
    }

    return bounds;
}
