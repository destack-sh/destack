import type { Clock } from "@destack/environment";
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
import { useEnvironment } from "../page/view.ts";
import { TRANSPARENT } from "./utils.ts";

/** A delay in milliseconds, or an accessor of one that pauses the timer while false. */
export type TimeoutSource = number | Accessor<number | false>;

/** How a timer primitive runs: once after its delay, or every delay. */
export type TimerKind = "timeout" | "interval";

/** One pending timeout on the page's clock, replaced by each new one. */
export class Timeout {
    /** The page's clock. */
    readonly #clock: Clock;
    /** The pending timeout's handle, absent while none is pending. */
    #handle: number | undefined;

    /** Create a timeout of the component being set up, stopping as it disposes. */
    constructor() {
        this.#clock = useEnvironment().clock;
        onCleanup(() => this.clear());
    }

    /** Run a function once some milliseconds pass, replacing the pending timeout. */
    set(handler: () => void, delay: number): void {
        // run nothing on the server, else stop the pending timeout and start the new one
        if (isServer) {
            return;
        }
        this.clear();
        this.#handle = this.#clock.setTimeout(() => {
            this.#handle = undefined;
            handler();
        }, delay);
    }

    /** Stop the pending timeout. */
    clear(): void {
        this.#clock.clearTimeout(this.#handle);
        this.#handle = undefined;
    }
}

/** Run a callback on a clock after a delay, or every delay, until stopped. */
export function makeTimer(
    callback: () => void,
    delay: number,
    kind: TimerKind,
    clock: Clock,
): () => void {
    // run nothing on the server
    if (isServer) {
        return () => {};
    }
    const handle = start(callback, delay, kind, clock);

    return () => stop(handle, kind, clock);
}

/** Run a callback on the page's clock after a delay, or every delay, following a reactive delay. */
export function createTimer(callback: () => void, delay: TimeoutSource, kind: TimerKind): void {
    // run nothing on the server, and a fixed delay until cleanup
    if (isServer) {
        return;
    }
    const { clock } = useEnvironment();
    if (typeof delay === "number") {
        onCleanup(makeTimer(callback, delay, kind, clock));

        return;
    }

    // track when the callback last ran, how much of a delay has passed, and whether a timeout is done
    let isDone = false;
    let previousTime = clock.now();
    let fractionDone = 0;
    let shouldHandleFraction = false;
    const run = (): void => {
        untrack(callback);
        previousTime = clock.now();
        isDone = kind === "timeout";
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
                    fractionDone += (clock.now() - previousTime) / previous;
                }

                return undefined;
            }
            if (previous === false) {
                previousTime = clock.now();
            }

            // finish the banked share of the delay first, then go on at the new delay
            if (shouldHandleFraction) {
                if (typeof previous === "number") {
                    fractionDone += (clock.now() - previousTime) / previous;
                }
                previousTime = clock.now();
                if (fractionDone >= 1) {
                    fractionDone = 0;
                    run();
                } else if (fractionDone > 0) {
                    const remaining = (1 - fractionDone) * current;
                    fractionDone = 0;
                    let main: number | undefined;
                    const reconcile = clock.setTimeout(() => {
                        shouldHandleFraction = false;
                        run();
                        if (!isDone) {
                            main = start(run, current, kind, clock);
                        }
                    }, remaining);

                    return () => {
                        clock.clearTimeout(reconcile);
                        if (main !== undefined) {
                            stop(main, kind, clock);
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
            const handle = start(run, current, kind, clock);

            return () => stop(handle, kind, clock);
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
    const { clock } = useEnvironment();
    if (typeof timeout === "number") {
        onCleanup(makeTimer(handler, timeout, "interval", clock));

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
            const handle = clock.setInterval(() => {
                handler();
                setCurrent(timeout());
            }, delay);

            return () => clock.clearInterval(handle);
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
    createTimer(() => setTick((count) => count + 1), timeout, "interval");

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

/** Start a timer of a kind on a clock. */
function start(callback: () => void, delay: number, kind: TimerKind, clock: Clock): number {
    return kind === "timeout"
        ? clock.setTimeout(callback, delay)
        : clock.setInterval(callback, delay);
}

/** Stop a timer of a kind on a clock. */
function stop(handle: number, kind: TimerKind, clock: Clock): void {
    if (kind === "timeout") {
        clock.clearTimeout(handle);
    } else {
        clock.clearInterval(handle);
    }
}
