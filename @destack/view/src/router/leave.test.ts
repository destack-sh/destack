import { expect, test } from "@destack/test";
import { createRoot, createSignal, flush } from "solid-js";
import { createPageLeaveBlocker, makePageLeave } from "./leave.ts";

/** Try to leave the page, reading whether something blocked it. */
function leave(): boolean {
    const event = new Event("beforeunload", { cancelable: true });
    window.dispatchEvent(event);

    return event.defaultPrevented;
}

test("block leaving the page until cleared", () => {
    const clear = makePageLeave();
    const blocked = leave();
    clear();

    expect([blocked, leave()]).toEqual([true, false]);
});

test("block leaving the page while enabled, by default, by flag or by signal", () => {
    const observed = createRoot((disposeRoot) => {
        // block by default and by a false flag
        const results: boolean[] = [];
        const disposeDefault = createRoot((dispose) => {
            createPageLeaveBlocker();

            return dispose;
        });
        results.push(leave());
        disposeDefault();
        results.push(leave());
        createRoot((dispose) => {
            createPageLeaveBlocker(false);
            results.push(leave());
            dispose();
        });

        // block as a signal toggles
        const [isEnabled, setIsEnabled] = createSignal(true, { ownedWrite: true });
        createPageLeaveBlocker(isEnabled);
        for (const next of [true, false, true]) {
            setIsEnabled(next);
            flush();
            results.push(leave());
        }
        disposeRoot();

        return results;
    });

    expect(observed).toEqual([true, false, false, true, false, true]);
});
