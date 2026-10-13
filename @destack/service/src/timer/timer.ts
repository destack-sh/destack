/** The longest timer delay, the largest signed 32 bit integer, in milliseconds. */
export const MAX_TIMER_DELAY = 2 ** 31 - 1;

/** The runtime's `setImmediate` on Bun, Node and workerd, absent in browsers. */
const SET_IMMEDIATE = loadSetImmediate();

/** Wait a delay in milliseconds, as `scheduler.wait` does, rejecting once the signal aborts. */
export function wait(
    delay: number,
    options: { readonly signal?: AbortSignal } = {},
): Promise<void> {
    const signal = options.signal;

    return new Promise((resolve, reject) => {
        // reject at once for an aborted signal
        if (signal?.aborted === true) {
            reject(signal.reason);

            return;
        }

        // wait one timer at a time until done or aborted
        let remaining = delay;
        let timer: ReturnType<typeof setTimeout>;
        const next = () => {
            const step = Math.min(remaining, MAX_TIMER_DELAY);
            remaining -= step;
            timer = setTimeout(() => {
                if (remaining > 0) {
                    next();
                } else {
                    signal?.removeEventListener("abort", abort);
                    resolve();
                }
            }, step);
        };
        const abort = () => {
            clearTimeout(timer);
            reject(signal?.reason);
        };
        signal?.addEventListener("abort", abort, { once: true });
        next();
    });
}

/** Wait until the signal aborts, resolving at once for an aborted signal. */
export function until(signal: AbortSignal): Promise<void> {
    return new Promise((resolve) => {
        if (signal.aborted) {
            resolve();
        } else {
            signal.addEventListener("abort", () => resolve(), { once: true });
        }
    });
}

/** Let the event loop run its waiting tasks before going on, like the web's `scheduler.yield`. */
export function yieldToEventLoop(): Promise<void> {
    return new Promise((resolve) => {
        // run after the waiting I/O callbacks on a server runtime
        if (SET_IMMEDIATE !== undefined) {
            SET_IMMEDIATE(resolve);
        }
        // run after the queued tasks in a browser
        else {
            setTimeout(resolve, 0);
        }
    });
}

/** Read the runtime's `setImmediate`, absent in browsers. */
function loadSetImmediate(): ((callback: () => void) => unknown) | undefined {
    const scope: object = globalThis;

    return "setImmediate" in scope && isCallback(scope.setImmediate)
        ? scope.setImmediate
        : undefined;
}

/** Report whether a value schedules a callback. */
function isCallback(value: unknown): value is (callback: () => void) => unknown {
    return typeof value === "function";
}
