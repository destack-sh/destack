import { afterEach, beforeEach, expect, test, vi } from "@destack/test";
import { Clock, LONGEST_DELAY } from "./clock.ts";

beforeEach(() => {
    vi.useFakeTimers();
});

afterEach(() => {
    vi.useRealTimers();
});

test("end a system sleep once its delay passes", async () => {
    // settle only once the whole delay passed
    let isDone = false;
    const sleeping = Clock.system.sleep(1000).then(() => (isDone = true));
    await vi.advanceTimersByTimeAsync(999);
    const early = isDone;
    await vi.advanceTimersByTimeAsync(1);
    await sleeping;
    expect([early, isDone]).toEqual([false, true]);
});

test("reject a system sleep with the signal's reason on abort, and at once for an aborted signal", async () => {
    // reject on abort and clear the timer
    const controller = new AbortController();
    const sleeping = Clock.system.sleep(1000, controller.signal);
    controller.abort(new Error("stopped"));
    await expect(sleeping).rejects.toThrow(new Error("stopped"));
    expect(vi.getTimerCount()).toBe(0);

    // reject an aborted signal without starting a timer
    await expect(Clock.system.sleep(1000, controller.signal)).rejects.toThrow(new Error("stopped"));
    expect(vi.getTimerCount()).toBe(0);
});

test("sleep beyond the longest timer delay through a chain of timers", async () => {
    // settle once the whole delay passed
    let isDone = false;
    const sleeping = Clock.system.sleep(LONGEST_DELAY + 1000).then(() => (isDone = true));
    await vi.advanceTimersByTimeAsync(LONGEST_DELAY);
    expect([isDone, vi.getTimerCount()]).toEqual([false, 1]);
    await vi.advanceTimersByTimeAsync(1000);
    await sleeping;
    expect(isDone).toBe(true);
});

test("run, repeat and stop system timers by their numeric handles", async () => {
    // start a single timer, an interval and a timer stopped before it runs
    const runs: string[] = [];
    const once = Clock.system.setTimeout(() => runs.push("once"), 100);
    const every = Clock.system.setInterval(() => runs.push("every"), 40);
    const stopped = Clock.system.setTimeout(() => runs.push("stopped"), 50);
    Clock.system.clearTimeout(stopped);

    // run the interval twice and the single timer once, then stop the interval
    await vi.advanceTimersByTimeAsync(100);
    Clock.system.clearInterval(every);
    await vi.advanceTimersByTimeAsync(100);
    expect({
        handles: [once, every, stopped].map((handle) => typeof handle),
        runs,
        pending: vi.getTimerCount(),
    }).toEqual({
        handles: ["number", "number", "number"],
        runs: ["every", "every", "once"],
        pending: 0,
    });
});
