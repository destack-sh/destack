import { afterEach, beforeEach, expect, test } from "@destack/test";
import { vi } from "vitest";
import { wait } from "./timer.ts";

beforeEach(() => {
    vi.useFakeTimers();
});

afterEach(() => {
    vi.useRealTimers();
});

test("resolve a wait once its delay passes", async () => {
    // settle only once the time passed
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
    // reject on abort without the delay passing, clearing the timer
    const controller = new AbortController();
    const waiting = wait(1000, { signal: controller.signal });
    controller.abort(new Error("stopped"));
    await expect(waiting).rejects.toThrow(new Error("stopped"));
    expect(vi.getTimerCount()).toBe(0);

    // reject an aborted signal without starting a timer
    await expect(wait(1000, { signal: controller.signal })).rejects.toThrow(new Error("stopped"));
    expect(vi.getTimerCount()).toBe(0);
});
