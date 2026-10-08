import { expect, test } from "@destack/test";
import { createRoot } from "solid-js";
import { createMs, createRAF, targetFPS } from "./raf.ts";

/** A frame callback that does nothing. */
function callback(): void {}

test("keep frames stopped on the server, with the callback unchanged and the count at zero", () => {
    // start and stop a frame loop
    const [running, start, stop] = createRAF(() => {});
    start();
    const isRunning = running();
    stop();

    // filter a callback and count milliseconds
    const ms = createRoot((disposeRoot) => {
        const value = createMs(60)();
        disposeRoot();

        return value;
    });

    expect([isRunning, targetFPS(callback, 60) === callback, ms]).toEqual([false, true, 0]);
});
