import { afterAll, beforeAll, beforeEach, expect, test, vi } from "@destack/test";
import { createRoot, flush } from "solid-js";
import { createIdleTimer, type IdleTimerOptions } from "./idle.ts";

beforeAll(() => {
    vi.useFakeTimers();
});

beforeEach(() => {
    vi.clearAllTimers();
});

afterAll(() => {
    vi.useRealTimers();
});

/** Make an idle timer in a root. */
function idleTimer(
    options: IdleTimerOptions,
): ReturnType<typeof createIdleTimer> & { dispose: () => void } {
    return createRoot((disposeRoot) => ({ ...createIdleTimer(options), dispose: disposeRoot }));
}

/** Let time pass and commit the writes. */
function wait(milliseconds: number): void {
    vi.advanceTimersByTime(milliseconds);
    flush();
}

test("prompt after the idle timeout and go idle after the prompt timeout", () => {
    const timer = idleTimer({ idleTimeout: 5, promptTimeout: 5 });
    flush();
    const phases: boolean[][] = [];
    for (const step of [2, 5, 5]) {
        wait(step);
        phases.push([timer.isPrompted(), timer.isIdle()]);
    }
    timer.dispose();

    expect(phases).toEqual([
        [false, false],
        [true, false],
        [false, true],
    ]);
});

test("start, reset and stop the timers by hand", () => {
    // start, go idle, reset and go idle again
    const timer = idleTimer({ idleTimeout: 5, startManually: true });
    wait(50);
    const unstarted = timer.isIdle();
    timer.start();
    wait(50);
    const started = timer.isIdle();
    timer.reset();
    wait(4);
    const reset = timer.isIdle();
    wait(50);
    const again = timer.isIdle();

    // stop and wait
    timer.stop();
    wait(50);
    const stopped = timer.isIdle();
    timer.dispose();

    expect([unstarted, started, reset, again, stopped]).toEqual([false, true, false, true, false]);
});

test("go idle by hand, then report activity and run the timers as before", () => {
    // go idle on an element watched for clicks
    const element = document.createElement("div");
    const seen: string[] = [];
    const timer = idleTimer({
        element,
        events: ["click"],
        idleTimeout: 10,
        onActive: () => seen.push("active"),
        onIdle: (event) => seen.push(`idle ${event.type}`),
        onPrompt: () => seen.push("prompt"),
        startManually: true,
    });
    timer.triggerIdle();
    flush();
    const idle = timer.isIdle();

    // click, then wait out the timers
    element.click();
    flush();
    const active = timer.isIdle();
    wait(15);
    const idleAgain = timer.isIdle();
    timer.dispose();

    expect({ idle, active, idleAgain, seen }).toEqual({
        idle: true,
        active: false,
        idleAgain: true,
        seen: ["idle manualidle", "active", "prompt", "idle click"],
    });
});

test("watch only the chosen events on the chosen element", () => {
    // run the timers to idle
    const element = document.createElement("div");
    let status = "initial";
    const timer = idleTimer({
        promptTimeout: 30,
        idleTimeout: 30,
        startManually: true,
        onActive: () => (status = "active"),
        onIdle: () => (status = "idle"),
        onPrompt: () => (status = "prompted"),
        element,
        events: ["click"],
    });
    wait(10);
    const unstarted = status;
    timer.start();
    wait(50);
    const prompted = status;
    wait(60);
    const idle = status;

    // press a key, then click
    element.dispatchEvent(new KeyboardEvent("keydown", { key: "a" }));
    const afterKey = status;
    element.click();
    timer.dispose();

    expect([unstarted, prompted, idle, afterKey, status]).toEqual([
        "initial",
        "prompted",
        "idle",
        "idle",
        "active",
    ]);
});
