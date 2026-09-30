import { afterEach, beforeEach, expect, test } from "@destack/test";
import { vi } from "vitest";
import { MAX_TIMER_DELAY, until, wait } from "./timer.ts";

beforeEach(() => {
    vi.useFakeTimers();
});

afterEach(() => {
    vi.useRealTimers();
});

test("resolve a wait once its delay passes", async () => {
    // settle once the time passed
    let isDone = false;
    const waiting = wait(1000, { signal: new AbortController().signal }).then(
        () => (isDone = true),
    );
    await vi.advanceTimersByTimeAsync(999);
    expect(isDone).toBe(false);
    await vi.advanceTimersByTimeAsync(1);
    await waiting;
    expect(isDone).toBe(true);
});

test("reject a wait with the signal's reason on abort, and at once for an aborted signal", async () => {
    // reject on abort and clear the timer
    const controller = new AbortController();
    const waiting = wait(1000, { signal: controller.signal });
    controller.abort(new Error("stopped"));
    await expect(waiting).rejects.toThrow(new Error("stopped"));
    expect(vi.getTimerCount()).toBe(0);

    // reject an aborted signal without starting a timer
    await expect(wait(1000, { signal: controller.signal })).rejects.toThrow(new Error("stopped"));
    expect(vi.getTimerCount()).toBe(0);
});

test("wait delays beyond one timer through a chain of timers", async () => {
    // settle once the whole delay passed
    let isDone = false;
    const waiting = wait(MAX_TIMER_DELAY + 1000).then(() => (isDone = true));
    await vi.advanceTimersByTimeAsync(MAX_TIMER_DELAY);
    expect([isDone, vi.getTimerCount()]).toEqual([false, 1]);
    await vi.advanceTimersByTimeAsync(1000);
    await waiting;
    expect(isDone).toBe(true);
});

test("wait until a signal aborts, and at once for an aborted signal", async () => {
    // resolve once the signal aborts
    const controller = new AbortController();
    let isDone = false;
    const waiting = until(controller.signal).then(() => (isDone = true));
    await Promise.resolve();
    expect(isDone).toBe(false);
    controller.abort();
    await waiting;

    // resolve at once for an aborted signal
    await until(AbortSignal.abort());
    expect(isDone).toBe(true);
});
