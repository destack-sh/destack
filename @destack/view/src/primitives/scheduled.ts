import type { Clock } from "@destack/environment";
import type { Display } from "@destack/environment/browser";
import { isServer } from "@solidjs/web";
import { type Accessor, createSignal, getObserver, getOwner, onCleanup } from "solid-js";
import { callbackOf } from "./utils.ts";

/** Schedule a callback on a clock, debounced or throttled by a wait in milliseconds. */
export type ScheduleCallback = <Args extends unknown[]>(
    callback: (...args: Args) => void,
    wait: number | undefined,
    clock: Clock,
) => Scheduled<Args>;

/** A scheduled callback, cancellable before it runs. */
export interface Scheduled<Args extends unknown[]> {
    /** Schedule the callback with these arguments. */
    (...args: Args): void;
    /** Cancel the run in progress. */
    clear: () => void;
}

/** Where a leading and trailing schedule stands. */
type Edge = "ready" | "leading" | "trailing";

/** Debounce a callback to its trailing edge, cancelled on cleanup. */
export const debounce: ScheduleCallback = (callback, wait, clock) => {
    // run nothing on the server
    if (isServer) {
        return Object.assign(() => {}, { clear: () => {} });
    }

    // cancel the wait on cleanup
    let handle: number | undefined;
    const clear = (): void => clock.clearTimeout(handle);
    if (getOwner() !== null) {
        onCleanup(clear);
    }

    // restart the wait on each call
    const debounced: typeof callback = (...args) => {
        clear();
        handle = clock.setTimeout(() => callback(...args), wait);
    };

    return Object.assign(debounced, { clear });
};

/** Throttle a callback to its trailing edge with the latest arguments, cancelled on cleanup. */
export const throttle: ScheduleCallback = (callback, wait, clock) => {
    // run nothing on the server
    if (isServer) {
        return Object.assign(() => {}, { clear: () => {} });
    }

    // run once per wait with the arguments of the latest call
    let isThrottled = false;
    let handle: number | undefined;
    let latest: Parameters<typeof callback> | undefined;
    const throttled: typeof callback = (...args) => {
        // keep the latest arguments, and start a wait unless one runs
        latest = args;
        if (isThrottled) {
            return;
        }
        isThrottled = true;
        handle = clock.setTimeout(() => {
            isThrottled = false;
            if (latest !== undefined) {
                callback(...latest);
            }
        }, wait);
    };
    const clear = (): void => {
        clock.clearTimeout(handle);
        isThrottled = false;
    };
    if (getOwner() !== null) {
        onCleanup(clear);
    }

    return Object.assign(throttled, { clear });
};

/** Call back in a display's next idle period, at most a timeout later, returning a cancel. */
export function requestIdle(
    callback: () => void,
    timeout: number | undefined,
    display: Display,
): () => void {
    return callbackOf((signal) => display.idle(timeout, signal), callback);
}

/** Throttle a callback to a display's idle periods, waiting at most a while. */
export function scheduleIdle<Args extends unknown[]>(
    callback: (...args: Args) => void,
    maxWait: number | undefined,
    display: Display,
): Scheduled<Args> {
    // run nothing on the server
    if (isServer) {
        return Object.assign(() => {}, { clear: () => {} });
    }

    // run once per idle period with the arguments of the latest call
    let cancel: (() => void) | undefined;
    let latest: Args | undefined;
    const deferred = (...args: Args): void => {
        // keep the latest arguments, and wait for an idle period unless one is pending
        latest = args;
        cancel ??= requestIdle(
            () => {
                cancel = undefined;
                if (latest !== undefined) {
                    callback(...latest);
                }
            },
            maxWait,
            display,
        );
    };

    // cancel the pending period, by hand or on cleanup
    const clear = (): void => {
        cancel?.();
        cancel = undefined;
    };
    if (getOwner() !== null) {
        onCleanup(clear);
    }

    return Object.assign(deferred, { clear });
}

/** Run a callback on the leading edge of a debounce or throttle. */
export function leading<Args extends unknown[]>(
    schedule: ScheduleCallback,
    callback: (...args: Args) => void,
    wait: number | undefined,
    clock: Clock,
): Scheduled<Args> {
    // run only the first call on the server
    if (isServer) {
        return Object.assign(once(callback), { clear: () => {} });
    }

    // run when no wait is in progress, and start one either way
    let isScheduled = false;
    const scheduled = schedule(
        () => {
            isScheduled = false;
        },
        wait,
        clock,
    );
    const run: typeof callback = (...args) => {
        if (!isScheduled) {
            callback(...args);
        }
        isScheduled = true;
        scheduled();
    };
    const clear = (): void => {
        isScheduled = false;
        scheduled.clear();
    };
    if (getOwner() !== null) {
        onCleanup(clear);
    }

    return Object.assign(run, { clear });
}

/** Run a callback on the leading edge of the first call and the trailing edge of the rest. */
export function leadingAndTrailing<Args extends unknown[]>(
    schedule: ScheduleCallback,
    callback: (...args: Args) => void,
    wait: number | undefined,
    clock: Clock,
): Scheduled<Args> {
    // run only the first call on the server
    if (isServer) {
        return Object.assign(once(callback), { clear: () => {} });
    }

    // run the trailing edge only when called again during the wait
    let edge: Edge = "ready";
    const scheduled = schedule(
        (values: Args) => {
            if (edge === "trailing") {
                callback(...values);
            }
            edge = "ready";
        },
        wait,
        clock,
    );
    const run: typeof callback = (...args) => {
        if (edge === "ready") {
            callback(...args);
            edge = "leading";
        } else if (edge === "leading") {
            edge = "trailing";
        }
        scheduled(args);
    };
    const clear = (): void => {
        edge = "ready";
        scheduled.clear();
    };
    if (getOwner() !== null) {
        onCleanup(clear);
    }

    return Object.assign(run, { clear });
}

/** Gate computations on a schedule: true once the schedule fires since the last read. */
export function createScheduled(schedule: (callback: () => void) => () => void): Accessor<boolean> {
    // mark dirty when the schedule fires, waking what tracks the gate
    let listeners = 0;
    let isDirty = false;
    const [track, dirty] = createSignal(undefined, { equals: false, ownedWrite: true });
    const call = schedule(() => {
        isDirty = true;
        dirty(undefined);
    });

    return (): boolean => {
        // schedule a run and track the gate while clean
        if (!isDirty) {
            call();
            track();
        }

        // report dirty once, staying dirty for the other listeners
        if (isDirty) {
            isDirty = listeners > 0;

            return true;
        }
        if (getObserver() !== null) {
            listeners++;
            onCleanup(() => listeners--);
        }

        return false;
    };
}

/** Wrap a callback to run only on its first call. */
function once<Args extends unknown[]>(callback: (...args: Args) => void): (...args: Args) => void {
    let isCalled = false;

    return (...args) => {
        if (!isCalled) {
            isCalled = true;
            callback(...args);
        }
    };
}
