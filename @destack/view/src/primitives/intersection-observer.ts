import { isServer } from "@solidjs/web";
import {
    type Accessor,
    createEffect,
    createSignal,
    createStore,
    getOwner,
    NotReadyError,
    onCleanup,
    runWithOwner,
    untrack,
} from "solid-js";
import {
    access,
    TRANSPARENT,
    type FalsyValue,
    handleDiffArray,
    type MaybeAccessor,
    onMount,
} from "./utils.ts";

export { NotReadyError };

/** Start observing an element. */
export type AddIntersectionObserverEntry = (element: Element) => void;

/** Stop observing an element. */
export type RemoveIntersectionObserverEntry = (element: Element) => void;

/** Handle one element's intersection entry. */
export type EntryCallback = (
    entry: IntersectionObserverEntry,
    instance: IntersectionObserver,
) => void;

/** Observe an element with its own callback, or return a ref that does. */
export type AddViewportObserverEntry = {
    /** Observe an element with its callback. */
    (element: Element, callback: MaybeAccessor<EntryCallback>): void;
    /** Return a ref that observes its element with the callback. */
    (callback: MaybeAccessor<EntryCallback>): (element: Element) => void;
};

/** Stop observing an element and forget its callback. */
export type RemoveViewportObserverEntry = (element: Element) => void;

/** A viewport observer's `add`, with how to remove, start and stop it and its instance, absent on the server. */
export type CreateViewportObserverReturnValue = [
    AddViewportObserverEntry,
    {
        /** Stop observing an element. */
        remove: RemoveViewportObserverEntry;
        /** Observe the initial elements. */
        start: () => void;
        /** Stop observing every element. */
        stop: () => void;
        /** The observer, absent on the server. */
        instance: IntersectionObserver | undefined;
    },
];

/** Decide a visibility from an entry and the current visibility. */
export type VisibilitySetter<Context extends object = object> = (
    entry: IntersectionObserverEntry,
    context: Context & { visible: boolean },
) => boolean;

/** Where an element stands against the viewport, from its last two entries. */
export const Occurrence = {
    Entering: "Entering",
    Leaving: "Leaving",
    Inside: "Inside",
    Outside: "Outside",
} as const;
/** Where an element stands against the viewport. */
export type Occurrence = (typeof Occurrence)[keyof typeof Occurrence];

/** The horizontal direction an element moves in. */
export const DirectionX = { Left: "Left", Right: "Right", None: "None" } as const;
/** The horizontal direction an element moves in. */
export type DirectionX = (typeof DirectionX)[keyof typeof DirectionX];

/** The vertical direction an element moves in. */
export const DirectionY = { Top: "Top", Bottom: "Bottom", None: "None" } as const;
/** The vertical direction an element moves in. */
export type DirectionY = (typeof DirectionY)[keyof typeof DirectionY];

/** Make an intersection observer of some elements, disconnected on cleanup. */
export function makeIntersectionObserver(
    elements: Element[],
    onChange: IntersectionObserverCallback,
    options?: IntersectionObserverInit,
): {
    add: AddIntersectionObserverEntry;
    remove: RemoveIntersectionObserverEntry;
    start: () => void;
    reset: () => void;
    stop: () => void;
    instance: IntersectionObserver | undefined;
} {
    // observe nothing on the server, where there is no observer
    if (isServer) {
        return {
            add: () => {},
            remove: () => {},
            start: () => {},
            reset: () => {},
            stop: () => {},
            instance: undefined,
        };
    }

    // observe the elements right away, and disconnect on cleanup
    const instance = new IntersectionObserver(onChange, options);
    const add: AddIntersectionObserverEntry = (element) => observe(element, instance);
    const remove: RemoveIntersectionObserverEntry = (element) => instance.unobserve(element);
    const start = (): void => {
        for (const element of elements) {
            add(element);
        }
    };
    const reset = (): void => {
        for (const record of instance.takeRecords()) {
            remove(record.target);
        }
    };
    const stop = (): void => instance.disconnect();
    start();
    if (getOwner() !== null) {
        onCleanup(stop);
    }

    return { add, remove, start, stop, reset, instance };
}

/** Hold the latest entry of each observed element in a store, with a visibility reader pending until the first entry. */
export function createIntersectionObserver(
    elements: Accessor<Element[]>,
    options?: MaybeAccessor<IntersectionObserverInit>,
): readonly [
    entries: readonly IntersectionObserverEntry[],
    isVisible: (element: Element) => boolean,
] {
    // hold nothing on the server, reading every element as hidden
    if (isServer) {
        return [[], () => false];
    }

    // keep each element's latest entry in a slot of its own
    const [entries, setEntries] = createStore<IntersectionObserverEntry[]>([]);
    const slots = new WeakMap<Element, number>();
    let nextSlot = 0;
    const report: IntersectionObserverCallback = (reported) => {
        for (const entry of reported) {
            let slot = slots.get(entry.target);
            if (slot === undefined) {
                slot = nextSlot++;
                slots.set(entry.target, slot);
            }
            const index = slot;
            const frozen: IntersectionObserverEntry = Object.freeze({
                boundingClientRect: entry.boundingClientRect,
                intersectionRatio: entry.intersectionRatio,
                intersectionRect: entry.intersectionRect,
                isIntersecting: entry.isIntersecting,
                rootBounds: entry.rootBounds,
                target: entry.target,
                time: entry.time,
            });
            runWithOwner(null, () => {
                setEntries((draft) => {
                    draft[index] = frozen;
                });
            });
        }
    };

    // observe with the options, again whenever they change
    let observed: Element[] = [];
    let instance = new IntersectionObserver(
        report,
        untrack(() => access(options)),
    );
    onCleanup(() => instance.disconnect());
    if (typeof options === "function") {
        createEffect(
            options,
            (current) => {
                instance.disconnect();
                instance = new IntersectionObserver(report, current);
                for (const element of observed) {
                    observe(element, instance);
                }
            },
            TRANSPARENT,
        );
    }

    // observe the elements added and stop observing the ones removed
    createEffect(
        elements,
        (current, previous = []) => {
            handleDiffArray(
                current,
                previous,
                (element) => observe(element, instance),
                (element) => instance.unobserve(element),
            );
            observed = current;
        },
        TRANSPARENT,
    );

    // read an element's visibility, pending until its first entry
    const isVisible = (element: Element): boolean => {
        // read the element's latest entry
        const slot = slots.get(element);
        const entry = slot === undefined ? undefined : entries[slot];
        if (entry === undefined) {
            throw new NotReadyError("element has not yet been observed");
        }

        return entry.isIntersecting;
    };

    return [entries, isVisible];
}

/** Observe many elements through one observer, each with its own callback. */
export function createViewportObserver(
    elements: MaybeAccessor<Element[]>,
    callback: EntryCallback,
    options?: IntersectionObserverInit,
): CreateViewportObserverReturnValue;
/** Observe many elements through one observer, from pairs of an element and its callback. */
export function createViewportObserver(
    initial: MaybeAccessor<[Element, EntryCallback][]>,
    options?: IntersectionObserverInit,
): CreateViewportObserverReturnValue;
/** Observe elements added later through one observer. */
export function createViewportObserver(
    options?: IntersectionObserverInit,
): CreateViewportObserverReturnValue;
/** Observe elements through one observer, reading which form the arguments take. */
export function createViewportObserver(
    first?: MaybeAccessor<Element[] | [Element, EntryCallback][]> | IntersectionObserverInit,
    second?: EntryCallback | IntersectionObserverInit,
    third?: IntersectionObserverInit,
): CreateViewportObserverReturnValue {
    // observe nothing on the server
    if (isServer) {
        return [
            () => () => {},
            { remove: () => {}, start: () => {}, stop: () => {}, instance: undefined },
        ];
    }

    // read the initial pairs and the options from the arguments' form
    let initial: Accessor<[Element, EntryCallback][]> = noPairs;
    let options: IntersectionObserverInit | undefined;
    if (isElementSource(first)) {
        if (typeof second === "function") {
            initial = () =>
                elementsOf(access(first)).map((element): [Element, EntryCallback] => [
                    element,
                    second,
                ]);
            options = third;
        } else {
            initial = () => pairsOf(access(first));
            options = second;
        }
    } else {
        options = first;
    }

    // call each element's callback, or the callback its accessor gives
    const callbacks = new WeakMap<Element, MaybeAccessor<EntryCallback>>();
    const { add, remove, stop, instance } = makeIntersectionObserver(
        [],
        (entries, observer) => {
            for (const entry of entries) {
                const result: unknown = callbacks.get(entry.target)?.(entry, observer);
                if (isEntryCallback(result)) {
                    result(entry, observer);
                }
            }
        },
        options,
    );

    // observe an element with its callback, or return a ref that does
    function addEntry(element: Element, callback: MaybeAccessor<EntryCallback>): void;
    function addEntry(callback: MaybeAccessor<EntryCallback>): (element: Element) => void;
    /**
     * Observe an element now, or once a ref receives it.
     *
     * @construct a call with an element observes it now, and a call with a callback returns the ref
     */
    function addEntry(
        target: Element | MaybeAccessor<EntryCallback>,
        callback?: MaybeAccessor<EntryCallback>,
    ): ((element: Element) => void) | undefined {
        if (target instanceof Element) {
            if (callback !== undefined) {
                add(target);
                callbacks.set(target, callback);
            }

            return undefined;
        }

        return (element) => {
            add(element);
            callbacks.set(element, target);
        };
    }
    const removeEntry: RemoveViewportObserverEntry = (element) => {
        callbacks.delete(element);
        remove(element);
    };
    const start = (): void => {
        for (const [element, callback] of initial()) {
            addEntry(element, callback);
        }
    };
    onMount(start);

    return [addEntry, { remove: removeEntry, start, stop, instance }];
}

/** Follow whether one element is in view, pending until the first entry unless given an initial value. */
export function createVisibilityObserver(
    element: Accessor<Element | FalsyValue> | Element,
    options?: IntersectionObserverInit & { initialValue?: boolean },
    setter?: MaybeAccessor<VisibilitySetter>,
): Accessor<boolean> {
    // answer the initial value on the server
    if (isServer) {
        return () => options?.initialValue ?? false;
    }

    // hold the visibility, unknown until the first entry unless given
    const [visibility, setVisibility] = createSignal<boolean | undefined>(options?.initialValue, {
        ownedWrite: true,
    });
    const visible = (): boolean => {
        const current = visibility();
        if (current === undefined) {
            throw new NotReadyError("visibility not yet observed");
        }

        return current;
    };

    // decide the visibility from each entry, through the setter when given
    const decide = setter === undefined ? undefined : access(setter);
    const instance = new IntersectionObserver((entries) => {
        for (const entry of entries) {
            const previous = untrack(visibility) ?? false;
            setVisibility(
                decide === undefined ? entry.isIntersecting : decide(entry, { visible: previous }),
            );
        }
    }, options);
    onCleanup(() => instance.disconnect());

    // observe the element, or each element the accessor gives
    if (element instanceof Element) {
        observe(element, instance);
    } else {
        createEffect(
            element,
            (current) => {
                if (
                    current === false ||
                    current === 0 ||
                    current === "" ||
                    current === null ||
                    current === undefined
                ) {
                    return undefined;
                }
                observe(current, instance);

                return () => instance.unobserve(current);
            },
            TRANSPARENT,
        );
    }

    return visible;
}

/** Read where an element stands against the viewport from whether it intersects now and before. */
export function getOccurrence(
    isIntersecting: boolean,
    wasIntersecting: boolean | undefined,
): Occurrence {
    if (isIntersecting) {
        return wasIntersecting === true ? Occurrence.Inside : Occurrence.Entering;
    }

    return wasIntersecting === true ? Occurrence.Leaving : Occurrence.Outside;
}

/** Wrap a visibility setter to receive the element's occurrence. */
export function withOccurrence<Context extends object>(
    setter: MaybeAccessor<VisibilitySetter<Context & { occurrence: Occurrence }>>,
): () => VisibilitySetter<Context> {
    // decide nothing on the server
    if (isServer) {
        return () => () => false;
    }

    return () => {
        let wasIntersecting: boolean | undefined;
        const decide = access(setter);

        return (entry, context) => {
            const occurrence = getOccurrence(entry.isIntersecting, wasIntersecting);
            wasIntersecting = entry.isIntersecting;

            return decide(entry, { ...context, occurrence });
        };
    };
}

/** Read the direction an element moves in from its last two rectangles. */
export function getDirection(
    rectangle: DOMRectReadOnly,
    previous: DOMRectReadOnly | undefined,
    isIntersecting: boolean,
): { directionX: DirectionX; directionY: DirectionY } {
    // report no direction without a previous rectangle
    if (previous === undefined) {
        return { directionX: DirectionX.None, directionY: DirectionY.None };
    }

    // compare the edges, reading the direction against whether the element intersects
    let directionY: DirectionY = DirectionY.None;
    if (rectangle.top < previous.top) {
        directionY = isIntersecting ? DirectionY.Bottom : DirectionY.Top;
    } else if (rectangle.top > previous.top) {
        directionY = isIntersecting ? DirectionY.Top : DirectionY.Bottom;
    }
    let directionX: DirectionX = DirectionX.None;
    if (rectangle.left > previous.left) {
        directionX = isIntersecting ? DirectionX.Left : DirectionX.Right;
    } else if (rectangle.left < previous.left) {
        directionX = isIntersecting ? DirectionX.Right : DirectionX.Left;
    }

    return { directionX, directionY };
}

/** Wrap a visibility setter to receive the element's direction of movement. */
export function withDirection<Context extends object>(
    setter: MaybeAccessor<
        VisibilitySetter<Context & { directionX: DirectionX; directionY: DirectionY }>
    >,
): () => VisibilitySetter<Context> {
    // decide nothing on the server
    if (isServer) {
        return () => () => false;
    }

    return () => {
        let previous: DOMRectReadOnly | undefined;
        const decide = access(setter);

        return (entry, context) => {
            const direction = getDirection(
                entry.boundingClientRect,
                previous,
                entry.isIntersecting,
            );
            previous = entry.boundingClientRect;

            return decide(entry, { ...context, ...direction });
        };
    };
}

/** Observe an element, refusing one with `display: contents`, which has no box to intersect. */
function observe(element: Element, instance: IntersectionObserver): void {
    if (element instanceof HTMLElement && element.style.display === "contents") {
        throw new TypeError("an element with display: contents has no box to observe");
    }
    instance.observe(element);
}

/** Check whether the first argument of a viewport observer gives its elements. */
function isElementSource(
    value:
        | MaybeAccessor<Element[] | [Element, EntryCallback][]>
        | IntersectionObserverInit
        | undefined,
): value is MaybeAccessor<Element[] | [Element, EntryCallback][]> {
    return Array.isArray(value) || typeof value === "function";
}

/** Keep the elements of a list of elements. */
function elementsOf(list: Element[] | [Element, EntryCallback][]): Element[] {
    return list.filter((item) => item instanceof Element);
}

/** Keep the pairs of a list of pairs. */
function pairsOf(list: Element[] | [Element, EntryCallback][]): [Element, EntryCallback][] {
    return list.filter((item) => Array.isArray(item));
}

/** Check whether a callback's result is itself an entry callback, as an accessor's is. */
function isEntryCallback(value: unknown): value is EntryCallback {
    return typeof value === "function";
}

/** List no initial pairs. */
function noPairs(): [Element, EntryCallback][] {
    return [];
}
