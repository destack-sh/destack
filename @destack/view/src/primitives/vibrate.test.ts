import { TEST_START } from "@destack/test/world";
import { testWorld } from "../test/browser.ts";
import { BrowserSimulation } from "@destack/environment/browser/simulation";
import { Simulation } from "@destack/environment/simulation";
import { afterEach, beforeEach, expect, test } from "@destack/test";
import { createSignal, flush } from "solid-js";
import { renderHook } from "../test/dom.ts";
import {
    createPulse,
    createVibrate,
    frequencyToPattern,
    makePulse,
    makeVibrate,
    type VibratePattern,
} from "./vibrate.ts";

/** The patterns the device was asked to vibrate. */
let vibrations: VibratePattern[] = [];

beforeEach(() => {
    // stand in for the device's motor, recording each pattern
    vibrations = [];
    Object.defineProperty(navigator, "vibrate", {
        configurable: true,
        value: (pattern: VibratePattern) => {
            vibrations.push(pattern);

            return true;
        },
    });
});

afterEach(() => {
    Reflect.deleteProperty(navigator, "vibrate");
});

test("vibrate once, or every interval until stopped", async () => {
    // vibrate once, then every half second for a second
    await using simulation = new Simulation({ seed: 1, start: TEST_START });
    const [once] = makeVibrate(200, {}, simulation.clock);
    once();
    const [start, stop] = makeVibrate([100, 50], { interval: 500 }, simulation.clock);
    start();
    await simulation.run({ to: TEST_START + 1000 });
    stop();
    await simulation.run({ to: TEST_START + 2000 });

    expect(vibrations).toEqual([200, [100, 50], [100, 50], [100, 50], 0]);
});

test("follow a vibration, restarting with its pattern as it changes, and stop on cleanup", async () => {
    // start a vibration in a tab, change its pattern and dispose it
    const { tab } = testWorld();
    const [pattern, setPattern] = createSignal<VibratePattern>(200);
    const { result, cleanup } = renderHook(() => createVibrate(pattern), { environment: tab });
    flush();
    result.start();
    flush();
    const during = result.vibrating();
    setPattern(300);
    flush();
    cleanup();

    expect([during, vibrations]).toEqual([true, [200, 300, 0]]);
});

test("turn a frequency into one cycle of vibration and pause", () => {
    expect([frequencyToPattern(2), frequencyToPattern(4, 0.25)]).toEqual([
        [250, 250],
        [63, 188],
    ]);
});

test("pulse a number of times a second, restarting at a changed frequency", async () => {
    // pulse at a fixed frequency
    await using simulation = new Simulation({ seed: 1, start: TEST_START });
    const [start, stop] = makePulse(2, {}, simulation.clock);
    start();
    stop();
    const fixed = [...vibrations];
    vibrations = [];

    // pulse in a tab at a frequency that doubles
    const tab = new BrowserSimulation(simulation).environment("tab");
    const [hz, setHz] = createSignal(1);
    const { result, cleanup } = renderHook(() => createPulse(hz), { environment: tab });
    flush();
    result.start();
    setHz(2);
    flush();
    cleanup();

    expect([fixed, vibrations]).toEqual([
        [[250, 250, 250, 250], 0],
        [[500, 500], [250, 250, 250, 250], 0],
    ]);
});

test("stay still where the device cannot vibrate", async () => {
    // start a vibration on a device without a motor
    Reflect.deleteProperty(navigator, "vibrate");
    const { tab } = testWorld();
    const { result } = renderHook(() => createVibrate(200), { environment: tab });
    result.start();

    expect(result.vibrating()).toBe(false);
});
