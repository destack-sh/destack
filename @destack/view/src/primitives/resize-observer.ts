import { isServer } from "@solidjs/web";
import { type Accessor, createEffect, getOwner, onCleanup, sharedConfig } from "solid-js";
import {
    access,
    asArray,
    handleDiffArray,
    type Many,
    type MaybeAccessor,
    TRANSPARENT,
} from "./utils.ts";
import { makeEventListener } from "./event-listener.ts";
import { createHydratableSingletonRoot } from "./rootless.ts";
import { createHydratableStaticStore, createStaticStore } from "./static-store.ts";

/** A resize observer's entry for an element of a known type. */
export type ResizeObserverEntryGeneric<Target extends Element> = ResizeObserverEntry & {
    /** The resized element. */
    readonly target: Target;
};

/** Handle an element's new content rectangle. */
export type ResizeHandler<Target extends Element = Element> = (
    rectangle: DOMRectReadOnly,
    element: Target,
    entry: ResizeObserverEntryGeneric<Target>,
) => void;

/** A width and height in CSS pixels. */
export type Size = {
    /** The width. */
    readonly width: number;
    /** The height. */
    readonly height: number;
};

/** An element's border-box size, and its padding-box size, which transforms leave unchanged. */
export type SizeWithClient = Size & {
    /** The padding-box width. */
    readonly clientWidth: number;
    /** The padding-box height. */
    readonly clientHeight: number;
};

/** An element's sizes, all null while there is no element to measure. */
export type NullableSize =
    | SizeWithClient
    | {
          /** No width. */
          readonly width: null;
          /** No height. */
          readonly height: null;
          /** No padding-box width. */
          readonly clientWidth: null;
          /** No padding-box height. */
          readonly clientHeight: null;
      };

/** The window size on the server. */
const WINDOW_SIZE_FALLBACK: Size = { width: 0, height: 0 };

/** The sizes of no element. */
const ELEMENT_SIZE_FALLBACK: NullableSize = {
    width: null,
    height: null,
    clientWidth: null,
    clientHeight: null,
};

/** Make a resize observer disconnected on cleanup. */
export function makeResizeObserver<Target extends Element>(
    callback: (entries: ResizeObserverEntryGeneric<Target>[], observer: ResizeObserver) => void,
    options?: ResizeObserverOptions,
): { observe: (element: Target) => void; unobserve: (element: Target) => void };
/**
 * Observe the elements the caller adds.
 *
 * @construct every entry's target is an element the caller observed, so of the caller's type
 */
export function makeResizeObserver(
    callback: ResizeObserverCallback,
    options?: ResizeObserverOptions,
): { observe: (element: Element) => void; unobserve: (element: Element) => void } {
    // observe nothing on the server
    if (isServer) {
        return { observe: () => {}, unobserve: () => {} };
    }
    const observer = new ResizeObserver(callback);
    if (getOwner() !== null) {
        onCleanup(() => observer.disconnect());
    }

    return {
        observe: (element) => observer.observe(element, options),
        unobserve: (element) => observer.unobserve(element),
    };
}

/** Call back as one or several elements resize, following the targets as they change. */
export function createResizeObserver<Target extends Element>(
    targets: MaybeAccessor<Many<Target | undefined | null>>,
    onResize: ResizeHandler<Target>,
    options?: ResizeObserverOptions,
): void {
    // observe nothing on the server
    if (isServer) {
        return;
    }

    // call back when an element's rounded size changes
    const previousSizes = new WeakMap<Target, Size>();
    const { observe, unobserve } = makeResizeObserver<Target>((entries) => {
        for (const entry of entries) {
            const width = Math.round(entry.contentRect.width);
            const height = Math.round(entry.contentRect.height);
            const previous = previousSizes.get(entry.target);
            if (previous?.width !== width || previous.height !== height) {
                onResize(entry.contentRect, entry.target, entry);
                previousSizes.set(entry.target, { width, height });
            }
        }
    }, options);

    // observe the targets added and stop observing the ones removed
    let previous: Target[] = [];
    createEffect(
        () => asArray(access(targets)).filter((target) => target !== null && target !== undefined),
        (current: Target[]) => {
            handleDiffArray(current, previous, observe, unobserve);
            previous = current;
        },
        TRANSPARENT,
    );
}

/** Read the window's inner size. */
export function getWindowSize(): Size {
    return isServer
        ? { ...WINDOW_SIZE_FALLBACK }
        : { width: window.innerWidth, height: window.innerHeight };
}

/** Follow the window's inner size. */
export function createWindowSize(): Readonly<Size> {
    // keep the fallback on the server
    if (isServer) {
        return WINDOW_SIZE_FALLBACK;
    }
    const [size, setSize] = createHydratableStaticStore(WINDOW_SIZE_FALLBACK, getWindowSize);
    makeEventListener(window, "resize", () => setSize(getWindowSize()));

    return size;
}

/** Follow the window's inner size through one listener shared by every user. */
export const useWindowSize: () => Readonly<Size> = createHydratableSingletonRoot(createWindowSize);

/** Read an element's border-box and padding-box sizes. */
export function getElementSize(target: Element): SizeWithClient;
/** Read an element's sizes, all null without an element. */
export function getElementSize(target: Element | false | undefined | null): NullableSize;
/**
 * Measure an element.
 *
 * @construct an element always measures to numbers, so only an absent one reads null
 */
export function getElementSize(target: Element | false | undefined | null): NullableSize {
    // measure nothing on the server or without an element
    if (isServer || target === false || target === undefined || target === null) {
        return { ...ELEMENT_SIZE_FALLBACK };
    }
    const { width, height } = target.getBoundingClientRect();

    return { width, height, clientWidth: target.clientWidth, clientHeight: target.clientHeight };
}

/** Follow an element's border-box and padding-box sizes. */
export function createElementSize(target: Element): Readonly<SizeWithClient>;
/** Follow the sizes of the element an accessor gives, all null while it gives none. */
export function createElementSize(
    target: Accessor<Element | false | undefined | null>,
): Readonly<NullableSize>;
/**
 * Follow an element's sizes in a static store.
 *
 * @construct a fixed element always measures to numbers, so only an accessor's absent element reads null
 */
export function createElementSize(
    target: Accessor<Element | false | undefined | null> | Element,
): Readonly<NullableSize> {
    // measure nothing on the server
    if (isServer) {
        return ELEMENT_SIZE_FALLBACK;
    }

    // measure a fixed element right away, outside hydration
    const isAccessor = typeof target === "function";
    const [size, setSize] = createStaticStore<NullableSize>(
        sharedConfig.hydrating || isAccessor ? ELEMENT_SIZE_FALLBACK : getElementSize(target),
    );
    const observer = new ResizeObserver((entries) => {
        for (const entry of entries) {
            setSize(getElementSize(entry.target));
        }
    });
    onCleanup(() => observer.disconnect());

    // follow the element an accessor gives, or observe the fixed one
    if (isAccessor) {
        createEffect(
            target,
            (element) => {
                // read no size without an element
                if (element === false || element === undefined || element === null) {
                    setSize(ELEMENT_SIZE_FALLBACK);

                    return undefined;
                }
                setSize(getElementSize(element));
                observer.observe(element);

                return () => observer.unobserve(element);
            },
            TRANSPARENT,
        );
    } else {
        observer.observe(target);
    }

    return size;
}
