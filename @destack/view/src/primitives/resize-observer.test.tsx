import { afterAll, beforeAll, beforeEach, expect, test } from "@destack/test";
import { createRoot, createSignal, createStore, flush } from "solid-js";
import {
    createElementSize,
    createResizeObserver,
    getElementSize,
    getWindowSize,
    type SizeWithClient,
} from "./resize-observer.ts";

/** The elements the stand-in observers observe, in the order they were added. */
let observed = new Set<Element>();

/** Every element passed to `observe`, counting repeats. */
let observeCalls: Element[] = [];

/** How many observers were disconnected. */
let disconnections = 0;

/** The callbacks of the stand-in observers, to report resizes to. */
let callbacks: ResizeObserverCallback[] = [];

/** A stand-in for the browser's resize observer, recording what it observes. */
class StubObserver implements ResizeObserver {
    /** Keep the callback to report resizes to. */
    constructor(callback: ResizeObserverCallback) {
        callbacks.push(callback);
    }

    /** Record an observed element. */
    observe(target: Element): void {
        observed.add(target);
        observeCalls.push(target);
    }

    /** Forget an observed element. */
    unobserve(target: Element): void {
        observed.delete(target);
    }

    /** Forget every observed element. */
    disconnect(): void {
        observed.clear();
        disconnections++;
    }
}

/** The browser's resize observer, restored after the tests. */
const original = globalThis.ResizeObserver;

beforeAll(() => {
    globalThis.ResizeObserver = StubObserver;
});

beforeEach(() => {
    observed = new Set();
    observeCalls = [];
    callbacks = [];
});

afterAll(() => {
    globalThis.ResizeObserver = original;
});

/** Make an element measure to fixed sizes. */
function sized(size: SizeWithClient): HTMLElement {
    const element = document.createElement("div");
    element.getBoundingClientRect = () =>
        DOMRect.fromRect({ x: 0, y: 0, width: size.width, height: size.height });
    Object.defineProperty(element, "clientWidth", { value: size.clientWidth });
    Object.defineProperty(element, "clientHeight", { value: size.clientHeight });

    return element;
}

/** Report a resize of an element to every observer. */
function resize(target: Element, width: number, height: number): void {
    const rectangle = DOMRectReadOnly.fromRect({ x: 0, y: 0, width, height });
    const box: ResizeObserverSize = { inlineSize: width, blockSize: height };
    const entry: ResizeObserverEntry = {
        target,
        contentRect: rectangle,
        borderBoxSize: [box],
        contentBoxSize: [box],
        devicePixelContentBoxSize: [box],
    };
    const current = [...callbacks];
    const observer = new StubObserver(() => {});
    for (const callback of current) {
        callback([entry], observer);
    }
}

test("disconnect a resize observer on cleanup", () => {
    const before = disconnections;
    createRoot((disposeRoot) => {
        createResizeObserver(document.createElement("div"), () => {});
        disposeRoot();
    });

    expect(disconnections).toBe(before + 1);
});

test("observe one or several fixed targets from the first effect", () => {
    const first = document.createElement("div");
    const second = document.createElement("div");
    const dispose = createRoot((disposeRoot) => {
        createResizeObserver([first, second], () => {});

        return disposeRoot;
    });
    flush();
    const during = [...observed];
    dispose();

    expect([during, observed.size]).toEqual([[first, second], 0]);
});

test("follow the targets of a signal or store, observing each element once", () => {
    // observe through a signal and through a store
    const first = document.createElement("div");
    const second = document.createElement("div");
    const third = document.createElement("div");
    const observedLists = (["signal", "store"] as const).map((source) => {
        observed = new Set();
        observeCalls = [];
        const { dispose, setTargets } = createRoot((disposeRoot) => {
            const [signalTargets, setSignalTargets] = createSignal<HTMLElement[]>([first, second]);
            const [storeTargets, setStoreTargets] = createStore<HTMLElement[]>([first, second]);
            if (source === "signal") {
                createResizeObserver(signalTargets, () => {});
            } else {
                createResizeObserver(storeTargets, () => {});
            }

            return {
                dispose: disposeRoot,
                setTargets: (targets: HTMLElement[]) =>
                    source === "signal"
                        ? setSignalTargets(targets)
                        : setStoreTargets(() => targets),
            };
        });
        const beforeEffect = observed.size;
        flush();
        const initial = [...observed];

        // swap one target, then add all, then clear them
        setTargets([first, third]);
        flush();
        const swapped = [...observed];
        const calls = [...observeCalls];
        setTargets([first, second, third]);
        flush();
        const grown = observed.size;
        setTargets([]);
        flush();
        dispose();

        return { beforeEffect, initial, swapped, calls, grown, cleared: observed.size };
    });

    expect(observedLists).toEqual([
        {
            beforeEffect: 0,
            initial: [first, second],
            swapped: [first, third],
            calls: [first, second, third],
            grown: 3,
            cleared: 0,
        },
        {
            beforeEffect: 0,
            initial: [first, second],
            swapped: [first, third],
            calls: [first, second, third],
            grown: 3,
            cleared: 0,
        },
    ]);
});

test("call back only when a target's rounded size changes", () => {
    const target = document.createElement("div");
    const sizes: number[][] = [];
    const dispose = createRoot((disposeRoot) => {
        createResizeObserver(target, (rectangle, element) => {
            sizes.push([rectangle.width, rectangle.height, element === target ? 1 : 0]);
        });

        return disposeRoot;
    });
    flush();
    resize(target, 100, 50);
    resize(target, 100.2, 50.3);
    resize(target, 120, 50);
    dispose();

    expect(sizes).toEqual([
        [100, 50, 1],
        [120, 50, 1],
    ]);
});

test("read the window's size and an element's sizes, all null without an element", () => {
    expect([
        getWindowSize(),
        getElementSize(document.createElement("div")),
        getElementSize(undefined),
    ]).toEqual([
        { width: 1024, height: 768 },
        { width: 0, height: 0, clientWidth: 0, clientHeight: 0 },
        { width: null, height: null, clientWidth: null, clientHeight: null },
    ]);
});

test("measure a fixed element right away", () => {
    const element = sized({ width: 100, height: 200, clientWidth: 90, clientHeight: 190 });
    const size = createRoot((disposeRoot) => {
        const measured = { ...createElementSize(element) };
        disposeRoot();

        return measured;
    });

    expect(size).toEqual({ width: 100, height: 200, clientWidth: 90, clientHeight: 190 });
});

test("follow the element an accessor gives through replacements and absences", () => {
    // measure the first element once the effect runs
    const first = sized({ width: 100, height: 200, clientWidth: 90, clientHeight: 190 });
    const next = sized({ width: 300, height: 400, clientWidth: 290, clientHeight: 390 });
    const before = disconnections;
    const { size, setTarget, dispose } = createRoot((disposeRoot) => {
        const [target, retarget] = createSignal<Element | null>(first);

        return { size: createElementSize(target), setTarget: retarget, dispose: disposeRoot };
    });
    const beforeEffect = { ...size };
    flush();
    const steps = [{ ...size, observed: [...observed] }];

    // replace the element, remove it, and give it back
    for (const target of [next, null, first]) {
        setTarget(target);
        flush();
        steps.push({ ...size, observed: [...observed] });
    }
    dispose();

    expect({
        beforeEffect,
        steps,
        observed: observed.size,
        disconnected: disconnections - before,
    }).toEqual({
        beforeEffect: { width: null, height: null, clientWidth: null, clientHeight: null },
        steps: [
            { width: 100, height: 200, clientWidth: 90, clientHeight: 190, observed: [first] },
            { width: 300, height: 400, clientWidth: 290, clientHeight: 390, observed: [next] },
            { width: null, height: null, clientWidth: null, clientHeight: null, observed: [] },
            { width: 100, height: 200, clientWidth: 90, clientHeight: 190, observed: [first] },
        ],
        observed: 0,
        disconnected: 1,
    });
});
