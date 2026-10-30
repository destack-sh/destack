import { testWorld } from "../test/browser.ts";
import { expect, test } from "@destack/test";
import { createSignal, flush, onCleanup } from "solid-js";
import { createPageRoot, renderHook } from "../test/dom.ts";
import {
    createIntervalCounter,
    createPolled,
    createTimeoutLoop,
    createTimer,
    makeTimer,
} from "./timer.ts";

test("run a timeout once after its delay", async () => {
    const { simulation, tab } = testWorld();
    let count = 0;
    makeTimer(() => count++, 100, "timeout", tab.clock);
    await simulation.clock.sleep(50);
    await simulation.idle();
    const early = count;
    await simulation.clock.sleep(60);
    await simulation.idle();
    const fired = count;
    await simulation.clock.sleep(200);
    await simulation.idle();

    expect([early, fired, count]).toEqual([0, 1, 1]);
});

test("run an interval every delay until cleared", async () => {
    const { simulation, tab } = testWorld();
    let count = 0;
    const clear = makeTimer(() => count++, 100, "interval", tab.clock);
    await simulation.clock.sleep(350);
    await simulation.idle();
    const fired = count;
    clear();
    await simulation.clock.sleep(300);
    await simulation.idle();

    expect([fired, count]).toEqual([3, 3]);
});

test("stop a timer made in a root once the caller cleans it up", async () => {
    const { simulation, tab } = testWorld();
    let count = 0;
    const dispose = createPageRoot((disposeRoot) => {
        onCleanup(makeTimer(() => count++, 100, "interval", tab.clock));

        return disposeRoot;
    }, tab);
    await simulation.clock.sleep(150);
    await simulation.idle();
    dispose();
    await simulation.clock.sleep(200);
    await simulation.idle();

    expect(count).toBe(1);
});

test("stop timers when their root is disposed", async () => {
    const { simulation, tab } = testWorld();
    let timeouts = 0;
    let intervals = 0;
    const { cleanup } = renderHook(
        () => {
            createTimer(() => timeouts++, 20, "timeout");
            createTimer(() => intervals++, 20, "interval");
        },
        { environment: tab },
    );
    cleanup();
    await simulation.clock.sleep(50);
    await simulation.idle();

    expect([timeouts, intervals]).toEqual([0, 0]);
});

test("run a timeout once and an interval every fixed delay until disposed", async () => {
    const { simulation, tab } = testWorld();
    let timeouts = 0;
    let intervals = 0;
    const dispose = createPageRoot((disposeRoot) => {
        createTimer(() => timeouts++, 100, "timeout");
        createTimer(() => intervals++, 100, "interval");

        return disposeRoot;
    }, tab);
    const counts: number[][] = [];
    for (const step of [50, 100, 100]) {
        await simulation.clock.sleep(step);
        await simulation.idle();
        counts.push([timeouts, intervals]);
    }
    dispose();
    await simulation.clock.sleep(100);
    await simulation.idle();
    counts.push([timeouts, intervals]);

    expect(counts).toEqual([
        [0, 0],
        [1, 1],
        [1, 2],
        [1, 2],
    ]);
});

test("pause timers while their delay is false, keeping the waited share", async () => {
    const { simulation, tab } = testWorld();
    // pause right away, then resume
    let timeouts = 0;
    let intervals = 0;
    const [isPaused, setIsPaused] = createSignal(false);
    const [delay, setDelay] = createSignal(50);
    const dispose = createPageRoot((disposeRoot) => {
        createTimer(
            () => timeouts++,
            () => !isPaused() && delay(),
            "timeout",
        );
        createTimer(
            () => intervals++,
            () => !isPaused() && delay(),
            "interval",
        );

        return disposeRoot;
    }, tab);
    setIsPaused(true);
    await simulation.clock.sleep(300);
    await simulation.idle();
    setIsPaused(false);
    const resumed = [timeouts, intervals];

    // pause again, then resume at a longer delay
    setIsPaused(true);
    await simulation.clock.sleep(100);
    await simulation.idle();
    setIsPaused(false);
    setDelay(100);
    flush();
    await simulation.clock.sleep(10);
    await simulation.idle();
    const early = [timeouts, intervals];
    await simulation.clock.sleep(160);
    await simulation.idle();
    dispose();

    expect([resumed, early, [timeouts, intervals]]).toEqual([
        [0, 0],
        [0, 0],
        [1, 1],
    ]);
});

test("stop an interval while its delay is false, and restart it after", async () => {
    const { simulation, tab } = testWorld();
    let count = 0;
    const [delay, setDelay] = createSignal<number | false>(100);
    const dispose = createPageRoot((disposeRoot) => {
        createTimer(() => count++, delay, "interval");

        return disposeRoot;
    }, tab);
    flush();
    await simulation.clock.sleep(100);
    await simulation.idle();
    const first = count;
    setDelay(false);
    flush();
    await simulation.clock.sleep(500);
    await simulation.idle();
    const paused = count;
    setDelay(100);
    flush();
    await simulation.clock.sleep(100);
    await simulation.idle();
    dispose();

    expect([first, paused, count]).toEqual([1, 1, 2]);
});

test("run a timeout loop every fixed delay until disposed", async () => {
    const { simulation, tab } = testWorld();
    let count = 0;
    const dispose = createPageRoot((disposeRoot) => {
        createTimeoutLoop(() => count++, 100);

        return disposeRoot;
    }, tab);
    await simulation.clock.sleep(350);
    await simulation.idle();
    const fired = count;
    dispose();
    await simulation.clock.sleep(200);
    await simulation.idle();

    expect([fired, count]).toEqual([3, 3]);
});

test("read a timeout loop's new delay only between runs", async () => {
    const { simulation, tab } = testWorld();
    // change the delay mid-wait
    let count = 0;
    const [delay, setDelay] = createSignal(100);
    const dispose = createPageRoot((disposeRoot) => {
        createTimeoutLoop(() => count++, delay);

        return disposeRoot;
    }, tab);
    flush();
    await simulation.clock.sleep(100);
    await simulation.idle();
    setDelay(200);
    flush();

    // finish the running wait, then wait the new delay
    await simulation.clock.sleep(100);
    await simulation.idle();
    const second = count;
    flush();
    await simulation.clock.sleep(199);
    await simulation.idle();
    const early = count;
    await simulation.clock.sleep(1);
    await simulation.idle();
    dispose();

    expect([second, early, count]).toEqual([2, 2, 3]);
});

test("stop a timeout loop after the run that reads a false delay, and restart it on a number", async () => {
    const { simulation, tab } = testWorld();
    // stop after the run reading false
    let count = 0;
    const [delay, setDelay] = createSignal<number | false>(100);
    const dispose = createPageRoot((disposeRoot) => {
        createTimeoutLoop(() => count++, delay);

        return disposeRoot;
    }, tab);
    flush();
    await simulation.clock.sleep(100);
    await simulation.idle();
    setDelay(false);
    flush();
    await simulation.clock.sleep(100);
    await simulation.idle();
    const last = count;
    flush();
    await simulation.clock.sleep(500);
    await simulation.idle();
    const stopped = count;

    // restart on a number
    setDelay(100);
    flush();
    await simulation.clock.sleep(200);
    await simulation.idle();
    dispose();

    expect([last, stopped, count]).toEqual([2, 2, 4]);
});

test("poll a computation right away and after each delay, stopping when disposed", async () => {
    const { simulation, tab } = testWorld();
    let calls = 0;
    const { polled, dispose } = createPageRoot(
        (disposeRoot) => ({
            polled: createPolled(() => ++calls, 100),
            dispose: disposeRoot,
        }),
        tab,
    );
    const values = [polled()];
    for (let tick = 0; tick < 2; tick++) {
        await simulation.clock.sleep(100);
        await simulation.idle();
        flush();
        values.push(polled());
    }
    dispose();
    await simulation.clock.sleep(100);
    await simulation.idle();
    flush();

    expect([...values, polled(), calls]).toEqual([1, 2, 3, 3, 3]);
});

test("poll a computation again when its dependencies change", async () => {
    const { tab } = testWorld();
    const [source, setSource] = createSignal(0);
    const { polled, dispose } = createPageRoot(
        (disposeRoot) => ({
            polled: createPolled(source, 100),
            dispose: disposeRoot,
        }),
        tab,
    );
    const before = polled();
    setSource(1);
    flush();
    const after = polled();
    dispose();

    expect([before, after]).toEqual([0, 1]);
});

test("pass the initial value to a polled computation's first run", async () => {
    const { tab } = testWorld();
    const { polled, dispose } = createPageRoot(
        (disposeRoot) => ({
            polled: createPolled((previous: number) => previous + 1, 100, 10),
            dispose: disposeRoot,
        }),
        tab,
    );
    const value = polled();
    dispose();

    expect(value).toBe(11);
});

test("poll at a reactive delay", async () => {
    const { simulation, tab } = testWorld();
    let calls = 0;
    const [delay, setDelay] = createSignal(100);
    const { polled, dispose } = createPageRoot(
        (disposeRoot) => ({
            polled: createPolled(() => ++calls, delay),
            dispose: disposeRoot,
        }),
        tab,
    );
    const values = [polled()];
    flush();
    await simulation.clock.sleep(100);
    await simulation.idle();
    flush();
    values.push(polled());
    setDelay(50);
    flush();
    await simulation.clock.sleep(50);
    await simulation.idle();
    flush();
    values.push(polled());
    dispose();

    expect(values).toEqual([1, 2, 3]);
});

test("poll on each delay before the accessor is first read", async () => {
    const { simulation, tab } = testWorld();
    let calls = 0;
    const { polled, dispose } = createPageRoot(
        (disposeRoot) => ({
            polled: createPolled(() => ++calls, 100),
            dispose: disposeRoot,
        }),
        tab,
    );
    await simulation.clock.sleep(100);
    await simulation.idle();
    flush();
    await simulation.clock.sleep(100);
    await simulation.idle();
    flush();
    const value = polled();
    dispose();

    expect(value).toBe(3);
});

test("count the delays that pass from zero, at a fixed or reactive delay", async () => {
    const { simulation, tab } = testWorld();
    // count at a fixed delay
    const fixed = createPageRoot(
        (disposeRoot) => ({
            count: createIntervalCounter(100),
            dispose: disposeRoot,
        }),
        tab,
    );
    const counts = [fixed.count()];
    await simulation.clock.sleep(100);
    await simulation.idle();
    flush();
    counts.push(fixed.count());
    fixed.dispose();

    // count at a delay that shortens
    const [delay, setDelay] = createSignal(100);
    const reactive = createPageRoot(
        (disposeRoot) => ({
            count: createIntervalCounter(delay),
            dispose: disposeRoot,
        }),
        tab,
    );
    flush();
    await simulation.clock.sleep(100);
    await simulation.idle();
    flush();
    counts.push(reactive.count());
    setDelay(50);
    flush();
    await simulation.clock.sleep(50);
    await simulation.idle();
    flush();
    counts.push(reactive.count());
    reactive.dispose();

    expect(counts).toEqual([0, 1, 1, 2]);
});
