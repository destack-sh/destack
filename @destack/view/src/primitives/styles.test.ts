import { afterAll, beforeAll, expect, test, vi } from "@destack/test";
import { createRoot, flush } from "solid-js";
import { createRemSize, getRemSize } from "./styles.ts";

/** The callbacks of the stand-in resize observers. */
let resizes: (() => void)[] = [];

/** How many observers were disconnected. */
let disconnections = 0;

/** A stand-in resize observer the test fires. */
class StubObserver implements ResizeObserver {
    /** Keep the callback to fire. */
    constructor(callback: ResizeObserverCallback) {
        resizes.push(() => callback([], this));
    }

    /** Observe an element. */
    observe(): void {}

    /** Stop observing an element. */
    unobserve(): void {}

    /** Stop observing. */
    disconnect(): void {
        disconnections++;
    }
}

/** The root font size the stand-in style reports. */
let fontSize = 16;

beforeAll(() => {
    vi.stubGlobal("ResizeObserver", StubObserver);
    const original = getComputedStyle;
    vi.stubGlobal("getComputedStyle", (element: Element) =>
        element === document.documentElement ? { fontSize: `${fontSize}px` } : original(element),
    );
});

afterAll(() => {
    vi.unstubAllGlobals();
});

test("read the root's font size", () => {
    fontSize = 20;
    const size = getRemSize();
    fontSize = 16;

    expect(size).toBe(20);
});

test("follow the rem size after the probe's first resize, cleaning up the probe", () => {
    resizes = [];
    const before = disconnections;
    const { remSize, dispose } = createRoot((disposeRoot) => ({
        remSize: createRemSize(),
        dispose: disposeRoot,
    }));
    const initial = remSize();

    // fire the first resize, which changes nothing, then one at a larger size
    for (const resize of resizes) {
        resize();
    }
    flush();
    const first = remSize();
    fontSize = 20;
    for (const resize of resizes) {
        resize();
    }
    flush();
    const grown = remSize();
    fontSize = 16;
    dispose();

    expect({
        initial,
        first,
        grown,
        disconnected: disconnections - before,
        probes: document.body.children.length,
    }).toEqual({
        initial: 16,
        first: 16,
        grown: 20,
        disconnected: 1,
        probes: 0,
    });
});
