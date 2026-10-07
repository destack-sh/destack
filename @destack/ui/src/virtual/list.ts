import {
    elementScroll,
    observeElementOffset,
    observeElementRect,
    type VirtualItem,
    Virtualizer,
} from "@tanstack/virtual-core";
import { type Accessor, createEffect, createSignal, untrack } from "@destack/view";

/** The rows a virtual list renders beyond the visible ones on either side. */
const OVERSCAN = 8;

/** What a virtual list measures: how many items, how tall each is at first, and the element that scrolls them. */
export interface VirtualListOptions {
    /** The number of items. */
    readonly count: Accessor<number>;
    /** The height each item is estimated at before it is measured, in pixels. */
    readonly itemHeight: number;
    /** The element that scrolls the items, undefined until it is created. */
    readonly scrollElement: Accessor<HTMLElement | undefined>;
}

/** The items a virtual list renders and the room they take. */
export interface VirtualList {
    /** The items in or near the visible range. */
    readonly items: Accessor<readonly VirtualItem[]>;
    /** The height of every item together, in pixels. */
    readonly height: Accessor<number>;
}

/** Follow which items of a long list are in view as its element scrolls, so only those render. */
export function createVirtualList(options: VirtualListOptions): VirtualList {
    // count each change the virtualizer reports
    const [revision, setRevision] = createSignal(0, { ownedWrite: true });
    const virtualizer = new Virtualizer<HTMLElement, Element>({
        count: untrack(options.count),
        estimateSize: () => options.itemHeight,
        getScrollElement: () => untrack(options.scrollElement) ?? null,
        overscan: OVERSCAN,
        observeElementRect,
        observeElementOffset,
        scrollToFn: elementScroll,
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
    };
}
