import { afterEach, beforeEach, expect, test, vi } from "@destack/test";
import { untilAborted, waitAborted, yieldToEventLoop } from "./wait.ts";

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
    const waiting = waitAborted(controller.signal).then(() => (isDone = true));
    await Promise.resolve();
    expect(isDone).toBe(false);
    controller.abort();
    await waiting;

    // resolve at once for an aborted signal
    await waitAborted(AbortSignal.abort());
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

test("wait for a promise or an abort, letting go of the signal once either settles", async () => {
    // count the abort listeners a long-lived signal holds
    const stopping = new AbortController();
    const { signal } = stopping;
    let listeners = 0;
    const add = signal.addEventListener.bind(signal);
    const remove = signal.removeEventListener.bind(signal);
    signal.addEventListener = (...entry: Parameters<AbortSignal["addEventListener"]>) => {
        listeners += 1;
        add(...entry);
    };
    signal.removeEventListener = (...entry: Parameters<AbortSignal["removeEventListener"]>) => {
        listeners -= 1;
        remove(...entry);
    };

    // settle a hundred waits on settled promises first, and fail one
    const settled = await Promise.all(
        Array.from({ length: 100 }, (_, index) =>
            untilAborted(Promise.resolve(index), signal).then(() => true),
        ),
    );
    const failed = await untilAborted(Promise.reject(new Error("failed")), signal).then(
        () => "settled",
        (error: unknown) => (error instanceof Error ? error.message : "other"),
    );
    const held = listeners;

    // abort while one still waits, and wait once more on the aborted signal
    const waiting = untilAborted(new Promise<number>(() => {}), signal).then(() => "aborted");
    stopping.abort();
    const aborted = await waiting;
    const late = await untilAborted(new Promise<number>(() => {}), signal).then(() => "late");

    // expect every wait settled, the failure, no listener left behind, and the aborted waits settled
    expect({ settled: settled.length, failed, held, aborted, late }).toEqual({
        settled: 100,
        failed: "failed",
        held: 0,
        aborted: "aborted",
        late: "late",
    });
});
