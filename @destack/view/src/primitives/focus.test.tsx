import { testWorld } from "../test/browser.ts";
import { afterEach, expect, test } from "@destack/test";
import { createSignal, flush } from "solid-js";
import { createPageRoot } from "../test/dom.ts";
import { autofocus, createAutofocus, createFocusSignal } from "./focus.ts";

afterEach(() => {
    document.body.replaceChildren();
});

/** Add a button to the document. */
function button(): HTMLButtonElement {
    const element = document.createElement("button");
    document.body.append(element);

    return element;
}

test("follow whether an element, fixed or given by an accessor, holds the focus", async () => {
    const { tab } = testWorld();
    // focus one element before following, then move the focus
    const first = button();
    const second = button();
    first.focus();
    const observed = createPageRoot((disposeRoot) => {
        const isFirst = createFocusSignal(first);
        const isSecond = createFocusSignal(() => second);
        flush();
        const before = [isFirst(), isSecond()];
        second.focus();
        flush();
        const after = [isFirst(), isSecond()];
        disposeRoot();

        return [before, after];
    }, tab);

    expect(observed).toEqual([
        [true, false],
        [false, true],
    ]);
});

test("focus a ref's element once rendered only when it asks for it", async () => {
    const { simulation, tab } = testWorld();
    const asking = button();
    asking.setAttribute("autofocus", "");
    const silent = button();
    const focused: boolean[] = [];
    for (const target of [silent, asking]) {
        document.body.focus();
        const dispose = createPageRoot((disposeRoot) => {
            autofocus()(target);

            return disposeRoot;
        }, tab);
        flush();
        await simulation.idle();
        dispose();
        focused.push(document.activeElement === target);
    }

    expect(focused).toEqual([false, true]);
});

test("focus each element an accessor gives, and none after disposal", async () => {
    const { simulation, tab } = testWorld();
    const first = button();
    const second = button();
    const [ref, setRef] = createSignal<HTMLButtonElement>();
    const dispose = createPageRoot((disposeRoot) => {
        createAutofocus(ref);

        return disposeRoot;
    }, tab);
    flush();
    await simulation.idle();
    const before = document.activeElement;
    setRef(first);
    flush();
    await simulation.idle();
    const afterFirst = document.activeElement;
    setRef(second);
    flush();
    await simulation.idle();
    const afterSecond = document.activeElement;
    dispose();
    setRef(first);
    flush();
    await simulation.idle();

    expect([before, afterFirst, afterSecond, document.activeElement]).toEqual([
        document.body,
        first,
        second,
        second,
    ]);
});
