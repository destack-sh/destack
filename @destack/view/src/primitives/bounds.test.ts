import { afterAll, beforeAll, expect, test } from "@destack/test";
import { createRoot, flush } from "solid-js";
import { createElementBounds, getElementBounds } from "./bounds.ts";

/** The callbacks of the stand-in resize observers, to report resizes to. */
let resizes: (() => void)[] = [];

/** A stand-in for the browser's resize observer, which the tests trigger. */
class StubObserver implements ResizeObserver {
    /** Keep the callback to report resizes to. */
    constructor(callback: ResizeObserverCallback) {
        resizes.push(() => callback([], this));
    }

    /** Observe an element. */
    observe(): void {}

    /** Stop observing an element. */
    unobserve(): void {}

    /** Stop observing every element. */
    disconnect(): void {}
}

/** The browser's resize observer, restored after the tests. */
const original = globalThis.ResizeObserver;

beforeAll(() => {
    globalThis.ResizeObserver = StubObserver;
});

afterAll(() => {
    globalThis.ResizeObserver = original;
});

/** Make an element whose rectangle the test moves. */
function movable(): { element: HTMLElement; move: (top: number) => void } {
    let top = 0;
    const element = document.createElement("div");
    element.getBoundingClientRect = () => DOMRect.fromRect({ x: 0, y: top, width: 10, height: 20 });

    return {
        element,
        move: (next) => {
            top = next;
        },
    };
}

/** Wait for the observers to deliver their records. */
async function settle(): Promise<void> {
    await new Promise((resolve) => {
        setTimeout(resolve, 0);
    });
}

test("read an element's rectangle, all null without an element", () => {
    expect([getElementBounds(document.createElement("div")), getElementBounds(undefined)]).toEqual([
        { top: 0, left: 0, bottom: 0, right: 0, width: 0, height: 0 },
        { top: null, left: null, bottom: null, right: null, width: null, height: null },
    ]);
});

test("follow a rectangle as its element resizes, its holder scrolls and the document mutates", async () => {
    // follow an element in a scrolled container
    resizes = [];
    const { element, move } = movable();
    const container = document.createElement("div");
    container.append(element);
    document.body.append(container);
    const { bounds, dispose } = createRoot((disposeRoot) => ({
        bounds: createElementBounds(element),
        dispose: disposeRoot,
    }));
    const tops = [bounds.top];

    // move it through a resize, a scroll, and a class change
    move(5);
    for (const resize of resizes) {
        resize();
    }
    flush();
    tops.push(bounds.top);
    move(10);
    container.dispatchEvent(new Event("scroll"));
    flush();
    tops.push(bounds.top);
    move(15);
    container.className = "moved";
    await settle();
    flush();
    tops.push(bounds.top);
    dispose();
    container.remove();

    expect(tops).toEqual([0, 5, 10, 15]);
});

test("read null bounds while an accessor gives no element", () => {
    const bounds = createRoot((disposeRoot) => {
        const nulled = { ...createElementBounds(() => undefined) };
        disposeRoot();

        return nulled;
    });

    expect(bounds).toEqual({
        top: null,
        left: null,
        bottom: null,
        right: null,
        width: null,
        height: null,
    });
});
