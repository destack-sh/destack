/** The runtime's `setImmediate` on Bun, Node and workerd, absent in browsers. */
const SET_IMMEDIATE = loadSetImmediate();

/** Wait until the signal aborts, resolving at once for an aborted signal. */
export function until(signal: AbortSignal): Promise<void> {
    return new Promise((resolve) => {
        // resolve at once for an aborted signal
        if (signal.aborted) {
            resolve();
        }
        // resolve once it aborts
        else {
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
