import { isServer } from "@solidjs/web";
import { type Accessor, createSignal, getObserver, getOwner, onCleanup } from "solid-js";

/** Schedule a callback, debounced or throttled by a wait in milliseconds. */
export type ScheduleCallback = <Args extends unknown[]>(
    callback: (...args: Args) => void,
    wait?: number,
) => Scheduled<Args>;

/** A scheduled callback, cancellable before it runs. */
export interface Scheduled<Args extends unknown[]> {
    /** Schedule the callback with these arguments. */
    (...args: Args): void;
    /** Cancel the run in progress. */
    clear: () => void;
}

/** Where a leading and trailing schedule stands: idle, called on the leading edge, or due on the trailing one. */
type Edge = "ready" | "leading" | "trailing";

/** Debounce a callback to its trailing edge, cancelled on cleanup. */
export const debounce: ScheduleCallback = (callback, wait) => {
    // run nothing on the server
    if (isServer) {
        return Object.assign(() => {}, { clear: () => {} });
    }

    // cancel the wait on cleanup
    let handle: ReturnType<typeof setTimeout> | undefined;
    const clear = (): void => clearTimeout(handle);
    if (getOwner() !== null) {
        onCleanup(clear);
    }

    // restart the wait on each call
    const debounced: typeof callback = (...args) => {
        clear();
        handle = setTimeout(() => callback(...args), wait);
    };

    return Object.assign(debounced, { clear });
};

/** Throttle a callback to its trailing edge with the latest arguments, cancelled on cleanup. */
export const throttle: ScheduleCallback = (callback, wait) => {
    // run nothing on the server
    if (isServer) {
        return Object.assign(() => {}, { clear: () => {} });
    }

    // run once per wait with the arguments of the latest call
    let isThrottled = false;
    let handle: ReturnType<typeof setTimeout> | undefined;
    let latest: Parameters<typeof callback> | undefined;
    const throttled: typeof callback = (...args) => {
        // keep the latest arguments, and start a wait unless one runs
        latest = args;
        if (isThrottled) {
            return;
        }
        isThrottled = true;
        handle = setTimeout(() => {
            isThrottled = false;
            if (latest !== undefined) {
                callback(...latest);
            }
        }, wait);
    };
    const clear = (): void => {
        clearTimeout(handle);
        isThrottled = false;
    };
    if (getOwner() !== null) {
        onCleanup(clear);
    }

    return Object.assign(throttled, { clear });
};

/** Throttle a callback to the browser's idle time, waiting at most a while. */
export const scheduleIdle: ScheduleCallback = (callback, maxWait) => {
    // run nothing on the server, and throttle where the browser has no idle callbacks
    if (isServer) {
        return Object.assign(() => {}, { clear: () => {} });
    }
    if (typeof requestIdleCallback === "undefined") {
        return throttle(callback);
    }

    // run once per idle period with the arguments of the latest call
    let isDeferred = false;
    let handle = 0;
    let latest: Parameters<typeof callback> | undefined;
    const deferred: typeof callback = (...args) => {
        // keep the latest arguments, and request an idle period unless one is pending
        latest = args;
        if (isDeferred) {
            return;
        }
        isDeferred = true;
        handle = requestIdleCallback(
            () => {
                isDeferred = false;
                if (latest !== undefined) {
                    callback(...latest);
                }
            },
            maxWait === undefined ? {} : { timeout: maxWait },
        );
    };
    const clear = (): void => {
        cancelIdleCallback(handle);
        isDeferred = false;
    };
    if (getOwner() !== null) {
        onCleanup(clear);
    }

    return Object.assign(deferred, { clear });
};

/** Run a callback on the leading edge of a debounce or throttle. */
export function leading<Args extends unknown[]>(
    schedule: ScheduleCallback,
    callback: (...args: Args) => void,
    wait?: number,
): Scheduled<Args> {
    // run the first call alone on the server
    if (isServer) {
        return Object.assign(once(callback), { clear: () => {} });
    }

    // run when no wait is in progress, and start one either way
    let isScheduled = false;
    const scheduled = schedule(() => {
        isScheduled = false;
    }, wait);
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
    wait?: number,
): Scheduled<Args> {
    // run the first call alone on the server
    if (isServer) {
        return Object.assign(once(callback), { clear: () => {} });
    }

    // run the trailing edge only when called again during the wait
    let edge: Edge = "ready";
    const scheduled = schedule((values: Args) => {
        if (edge === "trailing") {
            callback(...values);
        }
        edge = "ready";
    }, wait);
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

/** Wrap a callback to run on its first call alone. */
function once<Args extends unknown[]>(callback: (...args: Args) => void): (...args: Args) => void {
    let isCalled = false;

    return (...args) => {
        if (!isCalled) {
            isCalled = true;
            callback(...args);
        }
    };
}
