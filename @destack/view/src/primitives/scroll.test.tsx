import { afterEach, beforeEach, expect, test, vi } from "@destack/test";
import { createRoot, createSignal, flush } from "solid-js";
import { createPreventScroll, createScrollPosition, getScrollPosition } from "./scroll.ts";

beforeEach(() => {
    document.documentElement.removeAttribute("style");
    vi.stubGlobal("scrollTo", vi.fn());
});

afterEach(() => {
    document.documentElement.removeAttribute("style");
    vi.unstubAllGlobals();
});

/** Scroll an element to an offset, telling its listeners when asked. */
function scrollTo(element: Element, left: number, top: number, shouldDispatch = true): void {
    Object.assign(element, { scrollLeft: left, scrollTop: top });
    if (shouldDispatch) {
        element.dispatchEvent(new Event("scroll"));
    }
}

/** Read the document's overflow style. */
function overflow(): string {
    return document.documentElement.style.overflow;
}

test("read an element's scroll position, zero without a target", () => {
    const target = document.createElement("div");
    document.body.append(target);
    scrollTo(target, 222, 123, false);
    const position = getScrollPosition(target);
    target.remove();

    expect([getScrollPosition(undefined), position]).toEqual([
        { x: 0, y: 0 },
        { x: 222, y: 123 },
    ]);
});

test("follow an element's scroll position through its scroll events", () => {
    const target = document.createElement("div");
    const positions = createRoot((disposeRoot) => {
        const scroll = createScrollPosition(target);
        const seen = [{ ...scroll }];
        scrollTo(target, 100, 34);
        flush();
        seen.push({ ...scroll });
        scrollTo(target, 42, 11);
        flush();
        seen.push({ ...scroll });
        disposeRoot();

        return seen;
    });

    expect(positions).toEqual([
        { x: 0, y: 0 },
        { x: 100, y: 34 },
        { x: 42, y: 11 },
    ]);
});

test("follow the scroll position of the element an accessor gives", () => {
    const first = document.createElement("div");
    const second = document.createElement("div");
    scrollTo(first, 100, 34, false);
    scrollTo(second, 42, 11, false);
    const positions = createRoot((disposeRoot) => {
        const [target, setTarget] = createSignal<Element | undefined>(first, { ownedWrite: true });
        const scroll = createScrollPosition(target);
        const seen = [{ ...scroll }];
        setTarget(second);
        flush();
        seen.push({ ...scroll });
        setTarget(undefined);
        flush();
        seen.push({ ...scroll });
        disposeRoot();

        return seen;
    });

    expect(positions).toEqual([
        { x: 100, y: 34 },
        { x: 42, y: 11 },
        { x: 0, y: 0 },
    ]);
});

test("hide the document's overflow while scroll is prevented, restoring it on cleanup", () => {
    const observed = createRoot((disposeRoot) => {
        createPreventScroll();
        flush();
        const during = overflow();
        disposeRoot();

        return [during, overflow(), document.documentElement.hasAttribute("style")];
    });

    expect(observed).toEqual(["hidden", "", false]);
});

test("leave the overflow alone when told not to hide the scrollbar or not enabled", () => {
    const observed = [{ hideScrollbar: false }, { enabled: false }].map((properties) =>
        createRoot((disposeRoot) => {
            createPreventScroll(properties);
            flush();
            const value = overflow();
            disposeRoot();

            return value;
        }),
    );

    expect(observed).toEqual(["", ""]);
});

test("prevent scroll while a signal enables it", () => {
    const observed = createRoot((disposeRoot) => {
        const [isEnabled, setIsEnabled] = createSignal(false, { ownedWrite: true });
        createPreventScroll({ enabled: isEnabled });
        flush();
        const values = [overflow()];
        setIsEnabled(true);
        flush();
        values.push(overflow());
        setIsEnabled(false);
        flush();
        values.push(overflow());
        disposeRoot();

        return values;
    });

    expect(observed).toEqual(["", "hidden", ""]);
});

test("cancel wheel scrolling outside the element and let it through inside a scrolling one", () => {
    // allow scrolling a tall container only
    const container = document.createElement("div");
    container.style.overflowY = "auto";
    Object.defineProperty(container, "scrollHeight", { value: 500 });
    Object.defineProperty(container, "clientHeight", { value: 100 });
    const inside = document.createElement("p");
    container.append(inside);
    const outside = document.createElement("div");
    document.body.append(container, outside);
    const cancelled = createRoot((disposeRoot) => {
        createPreventScroll({ element: container });
        flush();

        // turn the wheel over each
        const results = [outside, inside].map((target) => {
            const event = new WheelEvent("wheel", { bubbles: true, cancelable: true, deltaY: 100 });
            target.dispatchEvent(event);

            return event.defaultPrevented;
        });
        disposeRoot();

        return results;
    });
    container.remove();
    outside.remove();

    expect(cancelled).toEqual([true, false]);
});

test("handle wheel events in the latest instance only, restoring the document after the last", () => {
    // stack two instances, each allowing its own element
    const first = document.createElement("div");
    const second = document.createElement("div");
    first.style.overflowY = "auto";
    Object.defineProperty(first, "scrollHeight", { value: 500 });
    Object.defineProperty(first, "clientHeight", { value: 100 });
    document.body.append(first, second);
    const observed = createRoot((disposeOuter) => {
        createPreventScroll({ element: first });
        flush();
        const disposeInner = createRoot((disposeRoot) => {
            createPreventScroll({ element: second });
            flush();

            return disposeRoot;
        });

        // turn the wheel over the first element while the second instance is on top, then after
        const wheel = (): boolean => {
            const event = new WheelEvent("wheel", { bubbles: true, cancelable: true, deltaY: 100 });
            first.dispatchEvent(event);

            return event.defaultPrevented;
        };
        const covered = wheel();
        disposeInner();
        flush();
        const uncovered = wheel();
        const held = overflow();
        disposeOuter();

        return { covered, uncovered, held, released: overflow() };
    });
    first.remove();
    second.remove();

    expect(observed).toEqual({ covered: true, uncovered: false, held: "hidden", released: "" });
});
