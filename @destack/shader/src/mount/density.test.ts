import { expect, test } from "@destack/test";
import { Density } from "./mount.ts";

/** Observe a window of twenty frames at an interval, starting after a time. */
function frames(density: Density, start: number, interval: number): number {
    let now = start;
    for (let frame = 0; frame <= 20; frame += 1) {
        now += interval;
        density.observe(now);
    }

    return now;
}

test("coarsen the density on a slow window and sharpen it only after four smooth windows", () => {
    const density = new Density();

    // one window at 30 frames a second coarsens a step at once
    let now = frames(density, 0, 33);
    const slow = density.factor;

    // three smooth windows keep the step, the fourth sharpens it
    now = frames(density, now, 16);
    now = frames(density, now, 16);
    now = frames(density, now, 16);
    const waiting = density.factor;
    frames(density, now, 16);

    expect([slow, waiting, density.factor]).toEqual([0.75, 0.75, 1]);
});

test("ignore the gaps of paused pages when judging frames", () => {
    const density = new Density();

    // frames a second apart are pauses, not slow frames
    frames(density, 0, 1000);

    expect(density.factor).toBe(1);
});
