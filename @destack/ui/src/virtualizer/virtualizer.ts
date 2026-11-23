import { nth } from "@destack/schema";
import * as virtual from "@tanstack/virtual-core";
import {
    type Accessor,
    createEffect,
    createMemo,
    createSignal,
    onCleanup,
    untrack,
} from "@destack/view";
import type { Direction } from "@destack/locale";
import type { Axis } from "../position/index.ts";

/** The items a virtualizer renders beyond the visible ones on either side. */
const OVERSCAN = 8;

/** An item a virtualizer renders: its index, key, lane, and where it starts and ends in pixels. */
export type VirtualItem = virtual.VirtualItem;

/** The visible range a virtualizer extracts the rendered indexes from. */
export type VirtualRange = virtual.Range;

/** A width and a height in pixels. */
export type VirtualRect = virtual.Rect;

/** Where a scroll puts an item: at the start, the center, the end, or the nearest edge. */
export type ScrollAlignment = "start" | "center" | "end" | "auto";

/** How a scroll moves: at once or smoothly. */
export type ScrollBehavior = "auto" | "smooth" | "instant";

/** Where and how a scroll lands. */
export interface ScrollOptions {
    /** Where the scroll puts the item, the nearest edge by default. */
    readonly align?: ScrollAlignment;
    /** How the scroll moves, at once by default. */
    readonly behavior?: ScrollBehavior;
}

/** What a virtualizer measures and how it scrolls. */
export interface VirtualizerOptions {
    /** The number of items. */
    readonly count: Accessor<number>;
    /** Read how to estimate an item's size before it is measured, a new one measuring all again. */
    readonly estimateSize: Accessor<(index: number) => number>;
    /** Read the element that scrolls the items, the window when absent. */
    readonly scrollElement?: Accessor<HTMLElement | undefined>;
    /** The axis the items run along, vertical by default. */
    readonly orientation?: Axis;
    /** The number of lanes across the scroll, such as the columns of a grid, one by default. */
    readonly lanes?: Accessor<number>;
    /** Whether an item joins the shortest lane by its estimate or by its measured size. */
    readonly laneAssignment?: "estimate" | "measured";
    /** The room between items, in pixels. */
    readonly gap?: number | undefined;
    /** The items rendered beyond the visible ones on either side, eight by default. */
    readonly overscan?: number | undefined;
    /** Read how to key an item of the items as they stand, by its index by default. */
    readonly itemKey?: Accessor<(index: number) => string | number> | undefined;
    /** Choose the indexes to render for a visible range, such as sticky headers beside it. */
    readonly rangeExtractor?: (range: VirtualRange) => number[];
    /** The indexes rendered out of view too, such as a dragged or focused item's. */
    readonly persisted?: Accessor<readonly number[]>;
    /** The room before the first item, in pixels. */
    readonly paddingStart?: Accessor<number>;
    /** The room after the last item, in pixels. */
    readonly paddingEnd?: Accessor<number>;
    /** The room a scroll to an item leaves before it, in pixels. */
    readonly scrollPaddingStart?: number;
    /** The room a scroll to an item leaves after it, in pixels. */
    readonly scrollPaddingEnd?: number;
    /** The distance from the scrolling element's start to the first item, in pixels. */
    readonly scrollMargin?: Accessor<number>;
    /** The end the view stays at as items come and go, the start by default. */
    readonly anchorTo?: "start" | "end" | undefined;
    /** Whether the view follows items added at the end while it is at the end. */
    readonly followOnAppend?: boolean | ScrollBehavior | undefined;
    /** The distance from the end that still counts as at the end, in pixels. */
    readonly scrollEndThreshold?: number | undefined;
    /** The size the scrolling element is taken to have before it is measured. */
    readonly initialRect?: VirtualRect | undefined;
    /** The scroll position at first, in pixels. */
    readonly initialOffset?: number | undefined;
    /** The measurements of an earlier virtualizer, from its snapshot. */
    readonly initialMeasurements?: readonly VirtualItem[];
    /** Report whether the virtualizer measures and renders, true by default. */
    readonly isEnabled?: Accessor<boolean>;
    /** The direction a horizontal list's text runs in, left to right by default. */
    readonly direction?: Accessor<Direction>;
}

/** The items of a long list in view as it scrolls, after TanStack Virtual. */
export class Virtualizer {
    /** The axis the items run along. */
    readonly orientation: Axis;
    /** The number of lanes across the scroll. */
    readonly lanes: Accessor<number>;
    /** The engine. */
    readonly #core:
        | virtual.Virtualizer<HTMLElement, Element>
        | virtual.Virtualizer<Window, Element>;
    /** Count each change the engine reports. */
    readonly #revision: Accessor<number>;
    /** The engine's settings, given to it as they change. */
    readonly #settings: Accessor<ReturnType<typeof settingsOf>>;
    /** Replace the engine's settings. */
    readonly #update: (settings: ReturnType<typeof settingsOf>) => void;

    /** Follow which items are in view as their element or the window scrolls. */
    constructor(options: VirtualizerOptions) {
        // count each change the engine reports
        const [revision, setRevision] = createSignal(0, { ownedWrite: true });
        this.#revision = revision;
        this.orientation = options.orientation ?? "vertical";
        this.lanes = options.lanes ?? (() => 1);
        const shared = {
            ...untrack(() => settingsOf(options)),
            measureElement: virtual.measureElement,
            onChange: () => setRevision((count) => count + 1),
        };

        // scroll an element where one is given, the window otherwise
        const scrollElement = options.scrollElement;
        if (scrollElement === undefined) {
            const core = new virtual.Virtualizer<Window, Element>({
                ...shared,
                getScrollElement: () => (typeof window === "undefined" ? null : window),
                observeElementRect: virtual.observeWindowRect,
                observeElementOffset: virtual.observeWindowOffset,
                scrollToFn: virtual.windowScroll,
            });
            this.#core = core;
            this.#update = (settings) => core.setOptions({ ...core.options, ...settings });
        } else {
            const core = new virtual.Virtualizer<HTMLElement, Element>({
                ...shared,
                getScrollElement: () => untrack(scrollElement) ?? null,
                observeElementRect: virtual.observeElementRect,
                observeElementOffset: virtual.observeElementOffset,
                scrollToFn: virtual.elementScroll,
            });
            this.#core = core;
            this.#update = (settings) => core.setOptions({ ...core.options, ...settings });
        }

        // give the engine its settings as they change, measuring again under a new estimate
        this.#settings = createMemo((previous: ReturnType<typeof settingsOf> | undefined) => {
            const settings = settingsOf(options);
            untrack(() => {
                this.#update(settings);
                if (previous !== undefined && previous.estimateSize !== settings.estimateSize) {
                    this.#core.measure();
                }
            });

            return settings;
        }, undefined);

        // attach to the scrolling element and place the scroll once the items have rendered
        createEffect(
            () => ({ settings: this.#settings(), element: scrollElement?.() }),
            (next, previous) =>
                untrack(() => {
                    // lay the items out under a new estimate, then measure the rendered ones again
                    this.#core._willUpdate();
                    const estimate = previous?.settings.estimateSize;
                    if (estimate !== undefined && estimate !== next.settings.estimateSize) {
                        this.#core.getVirtualItems();
                        for (const element of this.#core.elementsCache.values()) {
                            if (element.isConnected) {
                                this.#core.measureElement(element);
                            }
                        }
                    }
                }),
        );
        onCleanup(this.#core._didMount());
    }

    /** Follow the engine's settings and each change it reports. */
    #read(): void {
        this.#settings();
        this.#revision();
    }

    /** Read the items in or near view. */
    items(): readonly VirtualItem[] {
        this.#read();

        return this.#core.getVirtualItems();
    }

    /** Read the size of every item together, in pixels. */
    totalSize(): number {
        this.#read();

        return this.#core.getTotalSize();
    }

    /** Read the room the run of items in view leaves before and after it, persisted ones aside. */
    spacers(): { readonly before: number; readonly after: number } {
        // read the run of items around the range, leaving every item's room after a view without one
        const range = this.range();
        const items = range === undefined ? this.items() : runOf(this.items(), range.startIndex);
        const first = items[0];
        const last = items.at(-1);
        if (first === undefined || last === undefined) {
            return { before: 0, after: this.totalSize() };
        }

        // measure from the run's first item, less the room the persisted items around it take in flow
        const margin = this.#settings().scrollMargin;
        const rendered = this.items();
        const outside = {
            before: sizeOf(rendered.filter((item) => item.index < first.index)),
            after: sizeOf(rendered.filter((item) => item.index > last.index)),
        };

        return {
            before: first.start - margin - outside.before,
            after: this.totalSize() - (last.end - margin) - outside.after,
        };
    }

    /** Read the first and last index in view, none before the first measurement. */
    range(): { readonly startIndex: number; readonly endIndex: number } | undefined {
        this.#read();

        return this.#core.range ?? undefined;
    }

    /** Report whether the list scrolls right now. */
    isScrolling(): boolean {
        this.#read();

        return this.#core.isScrolling;
    }

    /** Read the scroll position, in pixels, none before the first measurement. */
    scrollOffset(): number | undefined {
        this.#read();

        return this.#core.scrollOffset ?? undefined;
    }

    /** Read the way the list last scrolled, none before it scrolled. */
    scrollDirection(): "forward" | "backward" | undefined {
        this.#read();

        return this.#core.scrollDirection ?? undefined;
    }

    /** Read the distance from the view's end to the list's end, in pixels. */
    distanceFromEnd(): number {
        this.#read();

        return this.#core.getDistanceFromEnd();
    }

    /** Report whether the view is at the list's end, within a threshold in pixels. */
    isAtEnd(threshold?: number): boolean {
        this.#read();

        return this.#core.isAtEnd(threshold);
    }

    /** Read the item at a scroll position. */
    itemAt(offset: number): VirtualItem | undefined {
        return this.#core.getVirtualItemForOffset(offset);
    }

    /** Read the scroll position that shows an item, none for an index out of range. */
    offsetOf(index: number, align: ScrollAlignment = "auto"): number | undefined {
        return this.#core.getOffsetForIndex(index, align)?.[0];
    }

    /** Read the attributes a rendered item's element spreads, which measure it. */
    itemAttributes(item: VirtualItem) {
        return {
            "data-index": item.index,
            ref: (element: Element) => {
                // mark the element before measuring it, as its attributes land after its reference
                element.setAttribute("data-index", String(item.index));
                this.measure(element);
            },
        };
    }

    /** Measure a rendered item's element, which has its index in `data-index`. */
    measure(element: Element): void {
        this.#core.measureElement(element);
    }

    /** Set the size of an item, in pixels. */
    resize(index: number, size: number): void {
        this.#core.resizeItem(index, size);
    }

    /** Scroll an item into view. */
    scrollToIndex(index: number, options: ScrollOptions = {}): void {
        this.#core.scrollToIndex(index, { align: "auto", ...options });
    }

    /** Scroll to a position, in pixels. */
    scrollToOffset(offset: number, options: ScrollOptions = {}): void {
        this.#core.scrollToOffset(offset, options);
    }

    /** Scroll by a distance, in pixels. */
    scrollBy(delta: number, behavior?: ScrollBehavior): void {
        this.#core.scrollBy(delta, behavior === undefined ? {} : { behavior });
    }

    /** Scroll to the list's end. */
    scrollToEnd(behavior?: ScrollBehavior): void {
        this.#core.scrollToEnd(behavior === undefined ? {} : { behavior });
    }

    /** Stop a scroll under way. */
    cancelScroll(): void {
        this.#core.cancelScroll();
    }

    /** Read the measured items, which a later virtualizer takes as its initial measurements. */
    snapshot(): readonly VirtualItem[] {
        return this.#core.takeSnapshot();
    }
}

/** Add up the sizes of some items. */
function sizeOf(items: readonly VirtualItem[]): number {
    return items.reduce((total, item) => total + item.size, 0);
}

/** Read the run of items with consecutive indexes around the first one at or past an index. */
function runOf(items: readonly VirtualItem[], index: number): readonly VirtualItem[] {
    // find the item at the index, then the items that follow on from it either way
    const at = items.findIndex((item) => item.index >= index);
    if (at === -1) {
        return [];
    }
    let [start, end] = [at, at + 1];
    while (start > 0 && nth(items, start - 1).index === nth(items, start).index - 1) {
        start -= 1;
    }
    while (end < items.length && nth(items, end).index === nth(items, end - 1).index + 1) {
        end += 1;
    }

    return items.slice(start, end);
}

/** Build the indexes a virtualizer renders: its range's, then the persisted ones out of it. */
function rangeOf(options: VirtualizerOptions): (range: VirtualRange) => number[] {
    // read the persisted indexes as they stand
    const extract = options.rangeExtractor ?? virtual.defaultRangeExtractor;
    const persisted = options.persisted?.() ?? [];
    if (persisted.length === 0) {
        return extract;
    }

    // add each persisted index the range leaves out, in order
    return (range) =>
        [...new Set([...extract(range), ...persisted])].toSorted((left, right) => left - right);
}

/** Read the engine's settings from a virtualizer's options as they stand. */
function settingsOf(options: VirtualizerOptions) {
    return {
        count: options.count(),
        estimateSize: options.estimateSize(),
        horizontal: options.orientation === "horizontal",
        lanes: options.lanes?.() ?? 1,
        laneAssignmentMode: options.laneAssignment ?? "estimate",
        gap: options.gap ?? 0,
        overscan: options.overscan ?? OVERSCAN,
        getItemKey: options.itemKey?.() ?? virtual.defaultKeyExtractor,
        rangeExtractor: rangeOf(options),
        paddingStart: options.paddingStart?.() ?? 0,
        paddingEnd: options.paddingEnd?.() ?? 0,
        scrollPaddingStart: options.scrollPaddingStart ?? 0,
        scrollPaddingEnd: options.scrollPaddingEnd ?? 0,
        scrollMargin: options.scrollMargin?.() ?? 0,
        anchorTo: options.anchorTo ?? "start",
        followOnAppend: options.followOnAppend ?? false,
        enabled: options.isEnabled?.() ?? true,
        isRtl: options.direction?.() === "rtl",
        ...(options.scrollEndThreshold === undefined
            ? {}
            : { scrollEndThreshold: options.scrollEndThreshold }),
        ...(options.initialRect === undefined ? {} : { initialRect: options.initialRect }),
        ...(options.initialOffset === undefined ? {} : { initialOffset: options.initialOffset }),
        ...(options.initialMeasurements === undefined
            ? {}
            : { initialMeasurementsCache: [...options.initialMeasurements] }),
    };
}
