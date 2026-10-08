import * as virtual from "@tanstack/virtual-core";
import { type Accessor, createEffect, createSignal, untrack } from "@destack/view";

/** The rows a virtualizer renders beyond the visible ones on either side. */
const OVERSCAN = 8;

/** What a virtualizer measures: how many items, how tall each is at first, and the element that scrolls them. */
export interface VirtualizerOptions {
    /** The number of items. */
    readonly count: Accessor<number>;
    /** The height each item is estimated at before it is measured, in pixels. */
    readonly itemHeight: number;
    /** The element that scrolls the items, undefined until it is created. */
    readonly scrollElement: Accessor<HTMLElement | undefined>;
    /** The height the element is taken to have before it is measured, in pixels, none when absent. */
    readonly initialHeight?: number;
}

/** The items of a long list a virtualizer renders and the room they take. */
export interface Virtualizer {
    /** The items in or near the visible range. */
    readonly items: Accessor<readonly virtual.VirtualItem[]>;
    /** The height of every item together, in pixels. */
    readonly height: Accessor<number>;
    /** Measure a rendered item's element, which holds its index in `data-index`. */
    readonly measure: (element: Element | undefined) => void;
    /** Scroll an item into view. */
    readonly reveal: (index: number) => void;
}

/** Follow which items of a long list are in view as its element scrolls, so only those render. */
export function createVirtualizer(options: VirtualizerOptions): Virtualizer {
    // count each change the virtualizer reports
    const [revision, setRevision] = createSignal(0, { ownedWrite: true });
    const virtualizer = new virtual.Virtualizer<HTMLElement, Element>({
        count: untrack(options.count),
        estimateSize: () => options.itemHeight,
        getScrollElement: () => untrack(options.scrollElement) ?? null,
        overscan: OVERSCAN,
        ...(options.initialHeight === undefined
            ? {}
            : { initialRect: { width: 0, height: options.initialHeight } }),
        measureElement: virtual.measureElement,
        observeElementRect: virtual.observeElementRect,
        observeElementOffset: virtual.observeElementOffset,
        scrollToFn: virtual.elementScroll,
        onChange: () => setRevision((count) => count + 1),
    });

    // follow the item count and attach to the scrolling element once it exists
    createEffect(
        () => ({ count: options.count(), element: options.scrollElement() }),
        ({ count, element }) => {
            // take the count and wait for the element before measuring
            virtualizer.setOptions({ ...virtualizer.options, count });
            if (element === undefined) {
                return undefined;
            }
            const detach = virtualizer._didMount();
            virtualizer._willUpdate();

            return detach;
        },
    );

    return {
        items: () => {
            revision();

            return virtualizer.getVirtualItems();
        },
        height: () => {
            revision();

            return virtualizer.getTotalSize();
        },
        measure: (element) => {
            if (element !== undefined) {
                virtualizer.measureElement(element);
            }
        },
        reveal: (index) => virtualizer.scrollToIndex(index, { align: "auto" }),
    };
}
