import { expect, onTestFinished, test, vi } from "@destack/test";
import { createRoot, flush, untrack } from "solid-js";
import { createMs, createRAF, targetFPS } from "./raf.ts";

test("request animation frames once started", () => {
    const request = vi.spyOn(globalThis, "requestAnimationFrame");
    const observed = createRoot((disposeRoot) => {
        const [running, start, stop] = createRAF(() => {});
        const before = [running(), request.mock.calls.length];
        start();
        flush();
        const after = [running(), request.mock.calls.length > 0];
        stop();
        disposeRoot();

        return [before, after];
    });
    request.mockRestore();

    expect(observed).toEqual([
        [false, 0],
        [true, true],
    ]);
});

test("run a frame callback at most at its frame rate", () => {
    const times: number[] = [];
    const filtered = targetFPS((time) => times.push(time), 60);
    for (const time of [1000, 1017, 1024, 1034]) {
        filtered(time);
    }

    expect(times).toEqual([1000, 1017, 1034]);
});

test("count frame milliseconds from the first frame, from zero again at the limit", () => {
    // run the frames by hand
    const frames: FrameRequestCallback[] = [];
    vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) =>
        frames.push(callback),
    );
    vi.stubGlobal("cancelAnimationFrame", () => {});
    onTestFinished(() => {
        vi.unstubAllGlobals();
    });

    // count at sixty frames a second up to a limit of thirty milliseconds
    const counts = createRoot((disposeRoot) => {
        const counter = createMs(60, 30);
        const read: number[] = [];
        for (const time of [1000, 1017, 1034, 1051]) {
            frames.shift()?.(time);
            flush();
            read.push(untrack(counter));
        }
        disposeRoot();

        return read;
    });

    expect(counts).toEqual([0, 17, 34, 0]);
});
