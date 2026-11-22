import { expect, onTestFinished, test } from "@destack/test";
import { createRoot, createSignal, flush } from "@destack/view";
import { Virtualizer, type VirtualizerOptions } from "./virtualizer.ts";

/** The height of the test viewport, in pixels. */
const VIEWPORT_HEIGHT = 100;

/** The height of each test item, in pixels. */
const ITEM_HEIGHT = 20;

/** Make a scrolling element of the test viewport's height, which the test DOM does not lay out. */
function viewport(): HTMLElement {
    const element = document.createElement("div");
    Object.defineProperty(element, "clientHeight", { value: VIEWPORT_HEIGHT });
    Object.defineProperty(element, "clientWidth", { value: VIEWPORT_HEIGHT });
    Object.defineProperty(element, "scrollHeight", { value: 100 * ITEM_HEIGHT });
    Object.defineProperty(element, "scrollWidth", { value: 100 * ITEM_HEIGHT });
    element.getBoundingClientRect = () =>
        DOMRect.fromRect({ width: VIEWPORT_HEIGHT, height: VIEWPORT_HEIGHT });
    document.body.append(element);
    onTestFinished(() => element.remove());

    return element;
}

/** Create a virtualizer over a hundred items scrolled by an element, disposed after the test. */
function createList(element: HTMLElement, options: Partial<VirtualizerOptions> = {}): Virtualizer {
    return createRoot((dispose) => {
        onTestFinished(dispose);

        return new Virtualizer({
            count: () => 100,
            estimateSize: () => ITEM_HEIGHT,
            scrollElement: () => element,
            ...options,
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
    expect({ top, scrolled: range(list), height: list.totalSize() }).toEqual({
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

        return new Virtualizer({
            count,
            estimateSize: () => ITEM_HEIGHT,
            scrollElement: scroller,
        });
    });
    flush();
    const detached = range(list);
    setScroller(element);
    setCount(3);
    flush();

    expect({ detached, attached: range(list), height: list.totalSize() }).toEqual({
        detached: [undefined, undefined],
        attached: [0, 2],
        height: 3 * ITEM_HEIGHT,
    });
});

test("leave the room of the items out of view before and after the ones it renders", () => {
    // scroll twenty items down and read the room around the rendered ones
    const element = viewport();
    const list = createList(element);
    flush();
    element.scrollTop = 20 * ITEM_HEIGHT;
    element.dispatchEvent(new Event("scroll"));
    flush();

    expect({ spacers: list.spacers(), offset: list.scrollOffset() }).toEqual({
        spacers: { before: 12 * ITEM_HEIGHT, after: (100 - 33) * ITEM_HEIGHT },
        offset: 20 * ITEM_HEIGHT,
    });
});

test("lay items out in lanes with a gap, as the rows of a grid", () => {
    // put a hundred items in four lanes, four pixels apart
    const element = viewport();
    const list = createList(element, { lanes: () => 4, gap: 4, overscan: 0 });
    flush();
    const first = list.items().slice(0, 5);

    expect({
        places: first.map((item) => [item.index, item.lane, item.start]),
        height: list.totalSize(),
    }).toEqual({
        places: [
            [0, 0, 0],
            [1, 1, 0],
            [2, 2, 0],
            [3, 3, 0],
            [4, 0, ITEM_HEIGHT + 4],
        ],
        height: 25 * ITEM_HEIGHT + 24 * 4,
    });
});

test("run along the horizontal axis, and find the item at a position and the position of an item", () => {
    const element = viewport();
    const list = createList(element, { orientation: "horizontal", overscan: 0 });
    flush();

    expect({
        range: range(list),
        at: list.itemAt(3 * ITEM_HEIGHT + 1)?.index,
        offset: list.offsetOf(10, "start"),
    }).toEqual({ range: [0, 4], at: 3, offset: 10 * ITEM_HEIGHT });
});

test("scroll an item into view, and to the end, where it reports being", () => {
    // scroll to the fiftieth item's start, then to the end
    const element = viewport();
    const list = createList(element, { overscan: 0 });
    flush();
    list.scrollToIndex(50, { align: "start" });
    const top = element.scrollTop;
    const isAtEndBefore = list.isAtEnd();
    list.scrollToEnd();
    element.dispatchEvent(new Event("scroll"));
    flush();

    expect({
        top,
        isAtEndBefore,
        end: element.scrollTop,
        distance: list.distanceFromEnd(),
    }).toEqual({
        top: 50 * ITEM_HEIGHT,
        isAtEndBefore: false,
        end: 100 * ITEM_HEIGHT - VIEWPORT_HEIGHT,
        distance: 0,
    });
});

test("render nothing while disabled, and render again once enabled", () => {
    const element = viewport();
    const [isEnabled, setEnabled] = createSignal(false);
    const list = createList(element, { isEnabled, overscan: 0 });
    flush();
    const disabled = range(list);
    setEnabled(true);
    flush();

    expect({ disabled, enabled: range(list) }).toEqual({
        disabled: [undefined, undefined],
        enabled: [0, 4],
    });
});

test("reuse a measured item's size, and hand the measurements to a later virtualizer", () => {
    // resize the first item, then start a second virtualizer from the first one's snapshot
    const element = viewport();
    const list = createList(element, { overscan: 0 });
    flush();
    list.resize(0, 3 * ITEM_HEIGHT);
    flush();
    const restored = createList(viewport(), { overscan: 0, initialMeasurements: list.snapshot() });
    flush();

    expect({
        first: list.items()[0]?.size,
        height: list.totalSize(),
        restored: restored.items()[1]?.start,
    }).toEqual({
        first: 3 * ITEM_HEIGHT,
        height: 102 * ITEM_HEIGHT,
        restored: 3 * ITEM_HEIGHT,
    });
});
