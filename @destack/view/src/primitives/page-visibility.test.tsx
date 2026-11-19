import { afterEach, expect, test } from "@destack/test";
import { createRoot, flush } from "solid-js";
import { createPageVisibility } from "./page-visibility.ts";

afterEach(() => {
    Reflect.deleteProperty(document, "visibilityState");
});

test("follow whether the page is visible, until disposed", () => {
    // start prerendered, then show and hide the page
    let state = "prerender";
    Object.defineProperty(document, "visibilityState", { configurable: true, get: () => state });
    const observed = createRoot((disposeRoot) => {
        const visibility = createPageVisibility();
        const values = [visibility()];
        for (const next of ["visible", "hidden"]) {
            state = next;
            document.dispatchEvent(new Event("visibilitychange"));
            flush();
            values.push(visibility());
        }
        disposeRoot();

        // show it once more after disposal
        state = "visible";
        document.dispatchEvent(new Event("visibilitychange"));
        flush();
        values.push(visibility());

        return values;
    });

    expect(observed).toEqual([false, true, false, false]);
});
