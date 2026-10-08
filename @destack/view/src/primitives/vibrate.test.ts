import { afterEach, beforeEach, expect, test, vi } from "@destack/test";
import { createRoot, createSignal, flush } from "solid-js";
import {
    createPulse,
    createVibrate,
    frequencyToPattern,
    makePulse,
    makeVibrate,
} from "./vibrate.ts";

/** The patterns the device was asked to vibrate. */
let vibrations: VibratePattern[] = [];

beforeEach(() => {
    vi.useFakeTimers();
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
    vi.useRealTimers();
    Reflect.deleteProperty(navigator, "vibrate");
});

test("vibrate once, or every interval until stopped", () => {
    const [once] = makeVibrate(200);
    once();
    const [start, stop] = makeVibrate([100, 50], { interval: 500 });
    start();
    vi.advanceTimersByTime(1000);
    stop();
    vi.advanceTimersByTime(1000);

    expect(vibrations).toEqual([200, [100, 50], [100, 50], [100, 50], 0]);
});

test("follow a vibration, restarting with its pattern as it changes, and stop on cleanup", () => {
    const [pattern, setPattern] = createSignal<VibratePattern>(200);
    const { vibrator, dispose } = createRoot((disposeRoot) => ({
        vibrator: createVibrate(pattern),
        dispose: disposeRoot,
    }));
    flush();
    vibrator.start();
    flush();
    const during = [vibrator.vibrating(), vibrator.supported];
    setPattern(300);
    flush();
    dispose();

    expect([during, vibrations]).toEqual([
        [true, true],
        [200, 300, 0],
    ]);
});

test("turn a frequency into one cycle of vibration and pause", () => {
    expect([frequencyToPattern(2), frequencyToPattern(4, 0.25)]).toEqual([
        [250, 250],
        [63, 188],
    ]);
});

test("pulse a number of times a second, restarting at a changed frequency", () => {
    // pulse at a fixed frequency
    const [start, stop] = makePulse(2);
    start();
    stop();
    const fixed = [...vibrations];
    vibrations = [];

    // pulse at a frequency that doubles
    const [hz, setHz] = createSignal(1);
    const { pulse, dispose } = createRoot((disposeRoot) => ({
        pulse: createPulse(hz),
        dispose: disposeRoot,
    }));
    flush();
    pulse.start();
    setHz(2);
    flush();
    dispose();

    expect([fixed, vibrations]).toEqual([
        [[250, 250, 250, 250], 0],
        [[500, 500], [250, 250, 250, 250], 0],
    ]);
});

test("do nothing where the device cannot vibrate, saying so", () => {
    Reflect.deleteProperty(navigator, "vibrate");
    const observed = createRoot((disposeRoot) => {
        const { supported, start, vibrating } = createVibrate(() => 200);
        start();
        disposeRoot();

        return [supported, vibrating()];
    });

    expect(observed).toEqual([false, false]);
});
