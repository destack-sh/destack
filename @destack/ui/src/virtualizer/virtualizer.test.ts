import { expect, onTestFinished, test } from "@destack/test";
import { createRoot, createSignal, flush } from "@destack/view";
import { createVirtualizer, type Virtualizer } from "./virtualizer.ts";

/** The height of the test viewport, in pixels. */
const VIEWPORT_HEIGHT = 100;

/** The height of each test item, in pixels. */
const ITEM_HEIGHT = 20;

/** Make a scrolling element of the test viewport's height, which the test DOM does not lay out. */
function viewport(): HTMLElement {
    const element = document.createElement("div");
    Object.defineProperty(element, "offsetHeight", { value: VIEWPORT_HEIGHT });
    Object.defineProperty(element, "offsetWidth", { value: VIEWPORT_HEIGHT });
    element.getBoundingClientRect = () =>
        DOMRect.fromRect({ width: VIEWPORT_HEIGHT, height: VIEWPORT_HEIGHT });
    document.body.append(element);
    onTestFinished(() => element.remove());

    return element;
}

/** Create a virtualizer over a hundred items scrolled by an element, disposed after the test. */
function createList(element: HTMLElement): Virtualizer {
    return createRoot((dispose) => {
        onTestFinished(dispose);

        return createVirtualizer({
            count: () => 100,
            itemHeight: ITEM_HEIGHT,
            scrollElement: () => element,
        });
    });
}

/** Read the first and last index of the items a virtualizer renders. */
function range(list: Virtualizer): readonly [number | undefined, number | undefined] {
    const items = list.items();

    return [items[0]?.index, items.at(-1)?.index];
}

test("render the visible items with eight to spare on either side as the element scrolls", () => {
    // measure the top, then scroll twenty items down
    const element = viewport();
    const list = createList(element);
    flush();
    const top = range(list);
    element.scrollTop = 20 * ITEM_HEIGHT;
    element.dispatchEvent(new Event("scroll"));
    flush();

    // the top shows items 0 to 4 and the scrolled view items 20 to 24, each with the overscan
    expect({ top, scrolled: range(list), height: list.height() }).toEqual({
        top: [0, 12],
        scrolled: [12, 32],
        height: 100 * ITEM_HEIGHT,
    });
});

test("follow the item count and wait for the element before rendering items", () => {
    // start without an element, then attach one and shrink the list
    const element = viewport();
    const [scroller, setScroller] = createSignal<HTMLElement | undefined>(undefined);
    const [count, setCount] = createSignal(100);
    const list = createRoot((dispose) => {
        onTestFinished(dispose);

        return createVirtualizer({ count, itemHeight: ITEM_HEIGHT, scrollElement: scroller });
    });
    flush();
    const detached = range(list);
    setScroller(element);
    setCount(3);
    flush();

    expect({ detached, attached: range(list), height: list.height() }).toEqual({
        detached: [undefined, undefined],
        attached: [0, 2],
        height: 3 * ITEM_HEIGHT,
    });
});
