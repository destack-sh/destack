import { isServer } from "@solidjs/web";
import { type Accessor, onCleanup } from "solid-js";
import { makeEventListener } from "./event-listener.ts";
import { makeFocusListener } from "./focus.ts";
import { createHydratableSignal } from "./utils.ts";

/** Call back with the focused element, null when the body holds the focus, until cleanup or the returned function. */
export function makeActiveElementListener(callback: (element: Element | null) => void): () => void {
    // listen nowhere on the server
    if (isServer) {
        return () => {};
    }

    // read the focused element whenever focus moves anywhere in the window
    const report = (): void => callback(activeElement());
    const stopBlur = makeEventListener(window, "blur", report, true);
    const stopFocus = makeEventListener(window, "focus", report, true);

    return () => {
        stopBlur();
        stopFocus();
    };
}

/** Follow the focused element, null while the body holds the focus. */
export function createActiveElement(): Accessor<Element | null> {
    // follow nothing on the server
    if (isServer) {
        return () => null;
    }
    const [active, setActive] = createHydratableSignal<Element | null>(null, activeElement);
    makeActiveElementListener(setActive);

    return active;
}

/** Report whether the element a ref receives holds the focus. */
export function focus(callback: (isActive: boolean) => void): (target: Element) => void {
    // report nothing on the server
    if (isServer) {
        return () => {};
    }

    // listen on the element the ref receives, stopping with the calling owner
    let stop: (() => void) | undefined;
    onCleanup(() => stop?.());

    return (target) => {
        callback(document.activeElement === target);
        stop = makeFocusListener(target, callback);
    };
}

/** Read the focused element, null while the body holds the focus. */
function activeElement(): Element | null {
    return document.activeElement === document.body ? null : document.activeElement;
}
