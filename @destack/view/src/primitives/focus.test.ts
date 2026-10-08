import { afterAll, afterEach, beforeAll, expect, test, vi } from "@destack/test";
import { createRoot, createSignal, flush } from "solid-js";
import { autofocus, createAutofocus, createFocusSignal } from "./focus.ts";

beforeAll(() => {
    vi.useFakeTimers();
});

afterEach(() => {
    vi.clearAllTimers();
    document.body.replaceChildren();
});

afterAll(() => {
    vi.useRealTimers();
});

/** Add a button to the document. */
function button(): HTMLButtonElement {
    const element = document.createElement("button");
    document.body.append(element);

    return element;
}

/** Run effects and timers. */
function settle(): void {
    flush();
    vi.runAllTimers();
}

test("follow whether an element, fixed or given by an accessor, holds the focus", () => {
    // focus one element before following, then move the focus
    const first = button();
    const second = button();
    first.focus();
    const observed = createRoot((disposeRoot) => {
        const isFirst = createFocusSignal(first);
        const isSecond = createFocusSignal(() => second);
        flush();
        const before = [isFirst(), isSecond()];
        second.focus();
        flush();
        const after = [isFirst(), isSecond()];
        disposeRoot();

        return [before, after];
    });

    expect(observed).toEqual([
        [true, false],
        [false, true],
    ]);
});

test("focus a ref's element once rendered only when it asks for it", () => {
    const asking = button();
    asking.setAttribute("autofocus", "");
    const silent = button();
    const focused = [silent, asking].map((target) => {
        document.body.focus();
        const dispose = createRoot((disposeRoot) => {
            autofocus()(target);

            return disposeRoot;
        });
        settle();
        dispose();

        return document.activeElement === target;
    });

    expect(focused).toEqual([false, true]);
});

test("focus each element an accessor gives, and none after disposal", () => {
    const first = button();
    const second = button();
    const [ref, setRef] = createSignal<HTMLButtonElement>();
    const dispose = createRoot((disposeRoot) => {
        createAutofocus(ref);

        return disposeRoot;
    });
    settle();
    const before = document.activeElement;
    setRef(first);
    settle();
    const afterFirst = document.activeElement;
    setRef(second);
    settle();
    const afterSecond = document.activeElement;
    dispose();
    setRef(first);
    settle();

    expect([before, afterFirst, afterSecond, document.activeElement]).toEqual([
        document.body,
        first,
        second,
        second,
    ]);
});
