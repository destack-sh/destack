import * as virtual from "@tanstack/virtual-core";
import { type Accessor, createEffect, createSignal, untrack } from "@destack/view";

/** The rows a virtualizer renders beyond the visible ones on either side. */
const OVERSCAN = 8;

/** What a virtualizer measures: how many items, how tall each is at first, and the element that scrolls them. */
export interface VirtualizerOptions {
    /** The number of items. */
    readonly count: Accessor<number>;
    /** Estimate the height of an item before it is measured, in pixels. */
    readonly estimateSize: (index: number) => number;
    /** Read the element that scrolls the items, undefined until it is created. */
    readonly getScrollElement: Accessor<HTMLElement | undefined>;
    /** The size the element is taken to have before it is measured, in pixels, none when absent. */
    readonly initialRect?: virtual.Rect;
}

/** The items of a long list a virtualizer renders and the room they take. */
export interface Virtualizer {
    /** Read the items in or near the visible range. */
    readonly getVirtualItems: Accessor<readonly virtual.VirtualItem[]>;
    /** Read the height of every item together, in pixels. */
    readonly getTotalSize: Accessor<number>;
    /** Measure a rendered item's element, which holds its index in `data-index`. */
    readonly measureElement: (element: Element | undefined) => void;
    /** Scroll an item into view. */
    readonly scrollToIndex: (index: number) => void;
}

/** Follow which items of a long list are in view as its element scrolls, so only those render. */
export function createVirtualizer(options: VirtualizerOptions): Virtualizer {
    // count each change the virtualizer reports
    const [revision, setRevision] = createSignal(0, { ownedWrite: true });
    const virtualizer = new virtual.Virtualizer<HTMLElement, Element>({
        count: untrack(options.count),
        estimateSize: options.estimateSize,
        getScrollElement: () => untrack(options.getScrollElement) ?? null,
        overscan: OVERSCAN,
        ...(options.initialRect === undefined ? {} : { initialRect: options.initialRect }),
        measureElement: virtual.measureElement,
        observeElementRect: virtual.observeElementRect,
        observeElementOffset: virtual.observeElementOffset,
        scrollToFn: virtual.elementScroll,
        onChange: () => setRevision((count) => count + 1),
    });

    // follow the item count and attach to the scrolling element once it exists
    createEffect(
        () => ({ count: options.count(), element: options.getScrollElement() }),
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
        getVirtualItems: () => {
            revision();

            return virtualizer.getVirtualItems();
        },
        getTotalSize: () => {
            revision();

            return virtualizer.getTotalSize();
        },
        measureElement: (element) => {
            if (element !== undefined) {
                virtualizer.measureElement(element);
            }
        },
        scrollToIndex: (index) => virtualizer.scrollToIndex(index, { align: "auto" }),
    };
}
