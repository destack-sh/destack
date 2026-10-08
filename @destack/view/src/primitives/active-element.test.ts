import { afterEach, expect, test } from "@destack/test";
import { createRoot, flush } from "solid-js";
import { createActiveElement, focus, makeActiveElementListener } from "./active-element.ts";

afterEach(() => {
    document.body.replaceChildren();
});

/** Add a button to the document. */
function button(): HTMLButtonElement {
    const element = document.createElement("button");
    document.body.append(element);

    return element;
}

test("call back with the focused element as focus moves, until cleared or disposed", () => {
    // listen in a root, focus, and dispose
    const target = button();
    const seen: (Element | null)[] = [];
    createRoot((disposeRoot) => {
        makeActiveElementListener((element) => seen.push(element));
        target.focus();
        disposeRoot();
    });
    target.blur();

    // listen until cleared
    const clear = makeActiveElementListener((element) => seen.push(element));
    target.focus();
    clear();
    target.blur();

    expect(seen).toEqual([target, target]);
});

test("follow the focused element, null while the body holds the focus", () => {
    const target = button();
    const observed = createRoot((disposeRoot) => {
        const active = createActiveElement();
        const before = active();
        target.focus();
        flush();
        const during = active();
        target.blur();
        flush();
        const after = active();
        disposeRoot();

        return [before, during, after];
    });

    expect(observed).toEqual([null, target, null]);
});

test("report whether a ref's element holds the focus, until disposed", () => {
    const target = button();
    const seen: boolean[] = [];
    createRoot((disposeRoot) => {
        focus((isActive) => seen.push(isActive))(target);
        target.focus();
        target.blur();
        disposeRoot();
    });
    target.focus();

    expect(seen).toEqual([false, true, false]);
});
