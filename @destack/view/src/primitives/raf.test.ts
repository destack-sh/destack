import { TEST_START } from "@destack/test/world";
import { testWorld } from "../test/browser.ts";
import { expect, test } from "@destack/test";
import { flush, untrack } from "solid-js";
import { createPageRoot } from "../test/dom.ts";
import { createMs, createRAF, loopFrames, targetFPS } from "./raf.ts";

test("run a callback on each frame of the page's display once started, until stopped", async () => {
    // run frames for 60 milliseconds, then stop and let as long pass
    const { simulation, tab } = testWorld();
    const times: number[] = [];
    const { running, start, stop, dispose } = createPageRoot((disposeRoot) => {
        const [isRunning, startFrames, stopFrames] = createRAF((time) =>
            times.push(time - TEST_START),
        );

        return { running: isRunning, start: startFrames, stop: stopFrames, dispose: disposeRoot };
    }, tab);
    const before = running();
    start();
    flush();
    const during = running();
    await simulation.run({ to: TEST_START + 60 });
    stop();
    await simulation.run({ to: TEST_START + 120 });
    dispose();

    expect({ before, during, times }).toEqual({ before: false, during: true, times: [17, 34, 50] });
});

test("stop a frame loop from inside its own callback", async () => {
    // stop the loop on its second frame, then let more frames pass
    const { simulation, tab } = testWorld();
    const times: number[] = [];
    const stop = loopFrames((time) => {
        times.push(time - TEST_START);
        if (times.length === 2) {
            stop();
        }
    }, tab.display);
    await simulation.run({ to: TEST_START + 100 });

    expect(times).toEqual([17, 34]);
});

test("run a frame callback at most at its frame rate", () => {
    const times: number[] = [];
    const filtered = targetFPS((time) => times.push(time), 60);
    for (const time of [1000, 1017, 1024, 1034]) {
        filtered(time);
    }

    expect(times).toEqual([1000, 1017, 1034]);
});

test("count frame milliseconds from the first frame, from zero again at the limit", async () => {
    // count at sixty frames a second up to a limit of thirty milliseconds
    const { simulation, tab } = testWorld();
    const { counter, dispose } = createPageRoot(
        (disposeRoot) => ({ counter: createMs(60, 30), dispose: disposeRoot }),
        tab,
    );

    // read the count after each of four frames
    const counts: number[] = [];
    for (const frame of [17, 34, 50, 67]) {
        await simulation.run({ to: TEST_START + frame });
        flush();
        counts.push(untrack(counter));
    }
    dispose();

    expect(counts).toEqual([0, 17, 33, 0]);
});
