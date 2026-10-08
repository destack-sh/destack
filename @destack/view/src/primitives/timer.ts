import { isServer } from "@solidjs/web";
import {
    type Accessor,
    createEffect,
    createMemo,
    createSignal,
    type MemoOptions,
    onCleanup,
    untrack,
} from "solid-js";
import { TRANSPARENT } from "./utils.ts";

/** A delay in milliseconds, or an accessor of one that pauses the timer while false. */
export type TimeoutSource = number | Accessor<number | false>;

/** The timer a timer primitive runs on: once after its delay, or every delay. */
export type Timer = typeof setTimeout | typeof setInterval;

/** Run a callback after a delay, or every delay, until the returned function is called. */
export function makeTimer(callback: () => void, delay: number, timer: Timer): () => void {
    // run nothing on the server
    if (isServer) {
        return () => {};
    }
    const handle = timer(callback, delay);

    return () => clear(handle, timer);
}

/** Run a callback after a delay, or every delay, following a reactive delay and keeping the elapsed share across pauses. */
export function createTimer(callback: () => void, delay: TimeoutSource, timer: Timer): void {
    // run nothing on the server, and a fixed delay until cleanup
    if (isServer) {
        return;
    }
    if (typeof delay === "number") {
        onCleanup(makeTimer(callback, delay, timer));

        return;
    }

    // track when the callback last ran, how much of a delay has passed, and whether a timeout is done
    let isDone = false;
    let previousTime = performance.now();
    let fractionDone = 0;
    let shouldHandleFraction = false;
    const run = (): void => {
        untrack(callback);
        previousTime = performance.now();
        isDone = timer === setTimeout;
    };

    // restart the timer whenever the delay changes, keeping the share of the delay already waited
    createEffect(
        delay,
        (current, previous) => {
            if (isDone) {
                return undefined;
            }

            // bank the waited share while paused
            if (current === false) {
                if (typeof previous === "number") {
                    fractionDone += (performance.now() - previousTime) / previous;
                }

                return undefined;
            }
            if (previous === false) {
                previousTime = performance.now();
            }

            // finish the banked share of the delay first, then go on at the new delay
            if (shouldHandleFraction) {
                if (typeof previous === "number") {
                    fractionDone += (performance.now() - previousTime) / previous;
                }
                previousTime = performance.now();
                if (fractionDone >= 1) {
                    fractionDone = 0;
                    run();
                } else if (fractionDone > 0) {
                    const remaining = (1 - fractionDone) * current;
                    fractionDone = 0;
                    let main: ReturnType<Timer> | undefined;
                    const reconcile = setTimeout(() => {
                        shouldHandleFraction = false;
                        run();
                        if (!isDone) {
                            main = timer(run, current);
                        }
                    }, remaining);

                    return () => {
                        clearTimeout(reconcile);
                        if (main !== undefined) {
                            clear(main, timer);
                        }
                    };
                }
            }

            // start the timer at the delay
            fractionDone = 0;
            shouldHandleFraction = true;
            if (isDone) {
                return undefined;
            }
            const handle = timer(run, current);

            return () => clear(handle, timer);
        },
        TRANSPARENT,
    );
}

/** Run a callback again and again, reading a reactive delay only between runs. */
export function createTimeoutLoop(handler: () => void, timeout: TimeoutSource): void {
    // run nothing on the server, and a fixed delay until cleanup
    if (isServer) {
        return;
    }
    if (typeof timeout === "number") {
        onCleanup(makeTimer(handler, timeout, setInterval));

        return;
    }

    // take the delay each run reads, restarting from the source when stopped
    const [current, setCurrent] = createSignal(untrack(timeout), { ownedWrite: true });
    createEffect(
        () => {
            const source = timeout();
            const latest = current();

            return latest === false ? source : latest;
        },
        (delay) => {
            if (delay === false) {
                return undefined;
            }
            const handle = setInterval(() => {
                handler();
                setCurrent(timeout());
            }, delay);

            return () => clearInterval(handle);
        },
        TRANSPARENT,
    );
}

/** Poll a computation every delay and whenever its own dependencies change. */
export function createPolled<Value>(
    compute: (previous: Value | undefined) => Value,
    timeout: TimeoutSource,
    value?: undefined,
    options?: MemoOptions<Value>,
): Accessor<Value>;
/** Poll a computation every delay and on its dependencies, starting from an initial value. */
export function createPolled<Value extends Previous, Initial = Value, Previous = Value>(
    compute: (previous: Previous | Initial) => Value,
    timeout: TimeoutSource,
    value: Initial,
    options?: MemoOptions<Value>,
): Accessor<Value>;
/** Poll a computation, starting from an optional initial value. */
export function createPolled<Value>(
    compute: (previous: Value | undefined) => Value,
    timeout: TimeoutSource,
    value?: Value,
    options?: MemoOptions<Value>,
): Accessor<Value> {
    // compute once on the server
    if (isServer) {
        const polled = compute(value);

        return () => polled;
    }

    // recompute on each tick of the timer and on the computation's own dependencies
    const [tick, setTick] = createSignal(0, { ownedWrite: true });
    let previous = value;
    const polled = createMemo(
        () => {
            tick();
            previous = compute(previous);

            return previous;
        },
        { ...options, ...TRANSPARENT },
    );
    createTimer(() => setTick((count) => count + 1), timeout, setInterval);

    return polled;
}

/** Count the delays that pass, from zero. */
export function createIntervalCounter(
    timeout: TimeoutSource,
    options?: MemoOptions<number>,
): Accessor<number> {
    // count nothing on the server
    if (isServer) {
        return () => 0;
    }

    return createPolled((previous) => previous + 1, timeout, -1, options);
}

/** Clear a timer with the clearing function of its kind. */
function clear(handle: ReturnType<Timer>, timer: Timer): void {
    if (timer === setTimeout) {
        clearTimeout(handle);
    } else {
        clearInterval(handle);
    }
}
