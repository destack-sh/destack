import { afterEach, beforeEach, expect, test, vi } from "@destack/test";
import { until, yieldToEventLoop } from "./wait.ts";

beforeEach(() => {
    vi.useFakeTimers();
});

afterEach(() => {
    vi.useRealTimers();
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

test("run the tasks waiting on the event loop before a yielded turn goes on", async () => {
    // go on only after the task waiting before the turn ran
    vi.useRealTimers();
    const order: string[] = [];
    setImmediate(() => order.push("task"));
    await yieldToEventLoop();
    order.push("turn");
    expect(order).toEqual(["task", "turn"]);
});
