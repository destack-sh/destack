import { afterAll, beforeAll, beforeEach, expect, test, vi } from "@destack/test";
import { createRoot, createSignal, flush, onCleanup } from "solid-js";
import {
    createIntervalCounter,
    createPolled,
    createTimeoutLoop,
    createTimer,
    makeTimer,
} from "./timer.ts";

beforeAll(() => {
    vi.useFakeTimers();
});

beforeEach(() => {
    vi.clearAllTimers();
});

afterAll(() => {
    vi.useRealTimers();
});

test("run a timeout once after its delay", () => {
    let count = 0;
    makeTimer(() => count++, 100, setTimeout);
    vi.advanceTimersByTime(50);
    const early = count;
    vi.advanceTimersByTime(60);
    const fired = count;
    vi.advanceTimersByTime(200);

    expect([early, fired, count]).toEqual([0, 1, 1]);
});

test("run an interval every delay until cleared", () => {
    let count = 0;
    const clear = makeTimer(() => count++, 100, setInterval);
    vi.advanceTimersByTime(350);
    const fired = count;
    clear();
    vi.advanceTimersByTime(300);

    expect([fired, count]).toEqual([3, 3]);
});

test("stop a timer made in a root once the caller cleans it up", () => {
    let count = 0;
    const dispose = createRoot((disposeRoot) => {
        onCleanup(makeTimer(() => count++, 100, setInterval));

        return disposeRoot;
    });
    vi.advanceTimersByTime(150);
    dispose();
    vi.advanceTimersByTime(200);

    expect(count).toBe(1);
});

test("stop timers when their root is disposed", () => {
    let timeouts = 0;
    let intervals = 0;
    createRoot((disposeRoot) => {
        createTimer(() => timeouts++, 20, setTimeout);
        createTimer(() => intervals++, 20, setInterval);
        disposeRoot();
    });
    vi.advanceTimersByTime(50);

    expect([timeouts, intervals]).toEqual([0, 0]);
});

test("run a timeout once and an interval every fixed delay until disposed", () => {
    let timeouts = 0;
    let intervals = 0;
    const dispose = createRoot((disposeRoot) => {
        createTimer(() => timeouts++, 100, setTimeout);
        createTimer(() => intervals++, 100, setInterval);

        return disposeRoot;
    });
    const counts: number[][] = [];
    for (const step of [50, 100, 100]) {
        vi.advanceTimersByTime(step);
        counts.push([timeouts, intervals]);
    }
    dispose();
    vi.advanceTimersByTime(100);
    counts.push([timeouts, intervals]);

    expect(counts).toEqual([
        [0, 0],
        [1, 1],
        [1, 2],
        [1, 2],
    ]);
});

test("pause timers while their delay is false, keeping the waited share", () => {
    // pause right away, then resume
    let timeouts = 0;
    let intervals = 0;
    const [isPaused, setIsPaused] = createSignal(false);
    const [delay, setDelay] = createSignal(50);
    const dispose = createRoot((disposeRoot) => {
        createTimer(
            () => timeouts++,
            () => !isPaused() && delay(),
            setTimeout,
        );
        createTimer(
            () => intervals++,
            () => !isPaused() && delay(),
            setInterval,
        );

        return disposeRoot;
    });
    setIsPaused(true);
    vi.advanceTimersByTime(300);
    setIsPaused(false);
    const resumed = [timeouts, intervals];

    // pause again, then resume at a longer delay
    setIsPaused(true);
    vi.advanceTimersByTime(100);
    setIsPaused(false);
    setDelay(100);
    flush();
    vi.advanceTimersByTime(10);
    const early = [timeouts, intervals];
    vi.advanceTimersByTime(160);
    dispose();

    expect([resumed, early, [timeouts, intervals]]).toEqual([
        [0, 0],
        [0, 0],
        [1, 1],
    ]);
});

test("stop an interval while its delay is false, and restart it after", () => {
    let count = 0;
    const [delay, setDelay] = createSignal<number | false>(100);
    const dispose = createRoot((disposeRoot) => {
        createTimer(() => count++, delay, setInterval);

        return disposeRoot;
    });
    flush();
    vi.advanceTimersByTime(100);
    const first = count;
    setDelay(false);
    flush();
    vi.advanceTimersByTime(500);
    const paused = count;
    setDelay(100);
    flush();
    vi.advanceTimersByTime(100);
    dispose();

    expect([first, paused, count]).toEqual([1, 1, 2]);
});

test("run a timeout loop every fixed delay until disposed", () => {
    let count = 0;
    const dispose = createRoot((disposeRoot) => {
        createTimeoutLoop(() => count++, 100);

        return disposeRoot;
    });
    vi.advanceTimersByTime(350);
    const fired = count;
    dispose();
    vi.advanceTimersByTime(200);

    expect([fired, count]).toEqual([3, 3]);
});

test("read a timeout loop's new delay only between runs", () => {
    // change the delay mid-wait
    let count = 0;
    const [delay, setDelay] = createSignal(100);
    const dispose = createRoot((disposeRoot) => {
        createTimeoutLoop(() => count++, delay);

        return disposeRoot;
    });
    flush();
    vi.advanceTimersByTime(100);
    setDelay(200);
    flush();

    // finish the running wait, then wait the new delay
    vi.advanceTimersByTime(100);
    const second = count;
    flush();
    vi.advanceTimersByTime(199);
    const early = count;
    vi.advanceTimersByTime(1);
    dispose();

    expect([second, early, count]).toEqual([2, 2, 3]);
});

test("stop a timeout loop after the run that reads a false delay, and restart it on a number", () => {
    // stop after the run reading false
    let count = 0;
    const [delay, setDelay] = createSignal<number | false>(100);
    const dispose = createRoot((disposeRoot) => {
        createTimeoutLoop(() => count++, delay);

        return disposeRoot;
    });
    flush();
    vi.advanceTimersByTime(100);
    setDelay(false);
    flush();
    vi.advanceTimersByTime(100);
    const last = count;
    flush();
    vi.advanceTimersByTime(500);
    const stopped = count;

    // restart on a number
    setDelay(100);
    flush();
    vi.advanceTimersByTime(200);
    dispose();

    expect([last, stopped, count]).toEqual([2, 2, 4]);
});

test("poll a computation right away and after each delay, stopping when disposed", () => {
    let calls = 0;
    const { polled, dispose } = createRoot((disposeRoot) => ({
        polled: createPolled(() => ++calls, 100),
        dispose: disposeRoot,
    }));
    const values = [polled()];
    for (let tick = 0; tick < 2; tick++) {
        vi.advanceTimersByTime(100);
        flush();
        values.push(polled());
    }
    dispose();
    vi.advanceTimersByTime(100);
    flush();

    expect([...values, polled(), calls]).toEqual([1, 2, 3, 3, 3]);
});

test("poll a computation again when its dependencies change", () => {
    const [source, setSource] = createSignal(0);
    const { polled, dispose } = createRoot((disposeRoot) => ({
        polled: createPolled(source, 100),
        dispose: disposeRoot,
    }));
    const before = polled();
    setSource(1);
    flush();
    const after = polled();
    dispose();

    expect([before, after]).toEqual([0, 1]);
});

test("pass the initial value to a polled computation's first run", () => {
    const { polled, dispose } = createRoot((disposeRoot) => ({
        polled: createPolled((previous: number) => previous + 1, 100, 10),
        dispose: disposeRoot,
    }));
    const value = polled();
    dispose();

    expect(value).toBe(11);
});

test("poll at a reactive delay", () => {
    let calls = 0;
    const [delay, setDelay] = createSignal(100);
    const { polled, dispose } = createRoot((disposeRoot) => ({
        polled: createPolled(() => ++calls, delay),
        dispose: disposeRoot,
    }));
    const values = [polled()];
    flush();
    vi.advanceTimersByTime(100);
    flush();
    values.push(polled());
    setDelay(50);
    flush();
    vi.advanceTimersByTime(50);
    flush();
    values.push(polled());
    dispose();

    expect(values).toEqual([1, 2, 3]);
});

test("poll on each delay before the accessor is first read", () => {
    let calls = 0;
    const { polled, dispose } = createRoot((disposeRoot) => ({
        polled: createPolled(() => ++calls, 100),
        dispose: disposeRoot,
    }));
    vi.advanceTimersByTime(100);
    flush();
    vi.advanceTimersByTime(100);
    flush();
    const value = polled();
    dispose();

    expect(value).toBe(3);
});

test("count the delays that pass from zero, at a fixed or reactive delay", () => {
    // count at a fixed delay
    const fixed = createRoot((disposeRoot) => ({
        count: createIntervalCounter(100),
        dispose: disposeRoot,
    }));
    const counts = [fixed.count()];
    vi.advanceTimersByTime(100);
    flush();
    counts.push(fixed.count());
    fixed.dispose();

    // count at a delay that shortens
    const [delay, setDelay] = createSignal(100);
    const reactive = createRoot((disposeRoot) => ({
        count: createIntervalCounter(delay),
        dispose: disposeRoot,
    }));
    flush();
    vi.advanceTimersByTime(100);
    flush();
    counts.push(reactive.count());
    setDelay(50);
    flush();
    vi.advanceTimersByTime(50);
    flush();
    counts.push(reactive.count());
    reactive.dispose();

    expect(counts).toEqual([0, 1, 1, 2]);
});
